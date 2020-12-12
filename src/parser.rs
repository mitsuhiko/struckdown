use std::iter;
use std::ops::Range;

use itertools::Either;
use lazy_static::lazy_static;
use pulldown_cmark as cm;
use regex::Regex;

use crate::event::{Alignment, Attrs, Event, EventType, Location, Str, Tag};

lazy_static! {
    static ref TEXT_ROLE_RE: Regex = Regex::new(r"\{([^\r\n\}]+)\}$").unwrap();
    static ref CODE_ROLE_RE: Regex = Regex::new(r"^\{([^\r\n\}]+)\}(?:\s+(.*?))$").unwrap();
    static ref HEADING_ID_RE: Regex = Regex::new(r"\s+\{#([^\r\n\}]+)\}\s*$").unwrap();
}

/// Reads until the end of an image tag all embedded content as raw string.
///
/// We do this because in markdown/cmark the alt text of an image is in fact
/// supporting a lot of markdown syntax but it usually gets rendered out into
/// an alt attribute.  Because of that we normalize this into text during parsing
/// already so that stream processors don't need to deal with this oddity.
fn read_raw_alt<'a, 'b, I: Iterator<Item = (cm::Event<'b>, Range<usize>)>>(
    iter: &'a mut I,
) -> String {
    let mut depth = 1;
    let mut rv = String::new();
    while let Some((event, _)) = iter.next() {
        match event {
            cm::Event::Start(..) => depth += 1,
            cm::Event::End(..) => depth -= 1,
            cm::Event::Text(text) => rv.push_str(&text),
            cm::Event::Code(code) => rv.push_str(&code),
            cm::Event::SoftBreak | cm::Event::HardBreak => rv.push('\n'),
            _ => {}
        }
        if depth == 0 {
            break;
        }
    }
    rv
}

/// A trailer is information that gets attached to the start tag when the end
/// tag is emitted.
///
/// Trailers are supported internally on all tags for which [`tag_supports_trailers`]
/// returns `true`.
pub enum Trailer<'data> {
    /// Defines the id attribute via trailer.
    Id(Str<'data>),
}

/// Checks if a tag supports trailers.
///
/// Currently all headlines are the only tags supporting trailers.
fn tag_supports_trailers(tag: Tag) -> bool {
    match tag {
        Tag::Heading1 => true,
        Tag::Heading2 => true,
        Tag::Heading3 => true,
        Tag::Heading4 => true,
        Tag::Heading5 => true,
        Tag::Heading6 => true,
        _ => false,
    }
}

/// Parses a string into an event stream with trailers.
///
/// This is normally not used as the `parse` function will already buffer
/// as necessary to automatically attach the trailers to the start tags.
fn parse_with_trailers<'data>(
    s: &'data str,
) -> impl Iterator<Item = (Event, Option<Trailer<'data>>)> {
    let mut opts = cm::Options::empty();
    opts.insert(cm::Options::ENABLE_TABLES);
    opts.insert(cm::Options::ENABLE_STRIKETHROUGH);
    opts.insert(cm::Options::ENABLE_TASKLISTS);
    opts.insert(cm::Options::ENABLE_FOOTNOTES);

    let parser = cm::Parser::new_with_broken_link_callback(s, opts, None);
    let mut iter = parser.into_offset_iter().peekable();
    let mut tag_stack = vec![];
    let mut pending_role = None;
    let mut pending_trailer = None;

    iter::from_fn(move || {
        let mut trailer = None;

        if let Some((event, range)) = iter.next() {
            // inefficient way to find the location
            let mut loc = Location {
                offset: range.start,
                len: range.end - range.start,
                line: s[..range.start].chars().filter(|&c| c == '\n').count() + 1,
                column: match s[..range.start].rfind('\n') {
                    Some(nl) => range.start - nl - 1,
                    None => range.start,
                },
            };

            // simple events
            let ty = match event {
                cm::Event::Start(cm_tag) => {
                    let mut attrs = Attrs::default();
                    let tag = match cm_tag {
                        cm::Tag::Paragraph => Tag::Paragraph,
                        cm::Tag::Heading(1) => Tag::Heading1,
                        cm::Tag::Heading(2) => Tag::Heading2,
                        cm::Tag::Heading(3) => Tag::Heading3,
                        cm::Tag::Heading(4) => Tag::Heading4,
                        cm::Tag::Heading(5) => Tag::Heading5,
                        cm::Tag::Heading(6) => Tag::Heading6,
                        cm::Tag::Heading(_) => unreachable!(),
                        cm::Tag::BlockQuote => Tag::BlockQuote,
                        cm::Tag::CodeBlock(kind) => match kind {
                            cm::CodeBlockKind::Fenced(lang) => {
                                let lang: Str<'_> = lang.into();
                                if let Some(m) = CODE_ROLE_RE.captures(lang.as_str()) {
                                    let g1 = m.get(1).unwrap();
                                    let g2 = m.get(2).unwrap();
                                    attrs.role = Some(lang.slice(g1.start(), g1.end()));
                                    attrs.argument = Some(lang.slice(g2.start(), g2.end()));
                                    Tag::Directive
                                } else {
                                    attrs.argument = Some(lang);
                                    Tag::FencedCode
                                }
                            }
                            cm::CodeBlockKind::Indented => Tag::IndentedCode,
                        },
                        cm::Tag::List(None) => Tag::UnorderedList,
                        cm::Tag::List(Some(start)) => {
                            attrs.start = Some(start as u32);
                            Tag::OrderedList
                        }
                        cm::Tag::Item => Tag::ListItem,
                        cm::Tag::FootnoteDefinition(id) => {
                            attrs.id = Some(id.into());
                            Tag::FootnoteDefinition
                        }
                        cm::Tag::Table(alignments) => {
                            attrs.alignments = Some(
                                alignments
                                    .into_iter()
                                    .map(|cm_align| match cm_align {
                                        cm::Alignment::None => Alignment::None,
                                        cm::Alignment::Left => Alignment::Left,
                                        cm::Alignment::Center => Alignment::Center,
                                        cm::Alignment::Right => Alignment::Right,
                                    })
                                    .collect(),
                            );
                            Tag::Table
                        }
                        cm::Tag::TableHead => Tag::TableHead,
                        cm::Tag::TableRow => Tag::TableRow,
                        cm::Tag::TableCell => Tag::TableCell,
                        cm::Tag::Emphasis => Tag::Emphasis,
                        cm::Tag::Strong => Tag::Strong,
                        cm::Tag::Strikethrough => Tag::Strikethrough,
                        cm::Tag::Link(_, target, title) => {
                            attrs.target = Some(target.into());
                            if !title.is_empty() {
                                attrs.title = Some(title.into());
                            }
                            Tag::Link
                        }
                        cm::Tag::Image(_, target, title) => {
                            // images are special in that we downgrade them from
                            // tags to toplevel events to not have to deal with
                            // nested text.
                            let alt = read_raw_alt(&mut iter);
                            return Some((
                                Event::new(
                                    EventType::Image {
                                        target: target.into(),
                                        alt: if alt.is_empty() {
                                            None
                                        } else {
                                            Some(alt.into())
                                        },
                                        title: if title.is_empty() {
                                            None
                                        } else {
                                            Some(title.into())
                                        },
                                    },
                                    loc,
                                ),
                                None,
                            ));
                        }
                    };
                    tag_stack.push(tag);
                    EventType::StartTag { tag, attrs }
                }
                cm::Event::End(_) => {
                    trailer = pending_trailer.take();
                    EventType::EndTag {
                        tag: tag_stack.pop().unwrap(),
                    }
                }
                cm::Event::Text(text) => {
                    let mut text: Str<'_> = text.into();

                    // handle roles
                    if let Some(&(cm::Event::Code(_), _)) = iter.peek() {
                        if let Some(m) = TEXT_ROLE_RE.captures(text.as_str()) {
                            let g0 = m.get(0).unwrap();
                            let g1 = m.get(1).unwrap();

                            // adjust the span of the text to not include the role.
                            let column_adjustment = g0.end() - g0.start();
                            loc.len -= column_adjustment;
                            pending_role =
                                Some((text.slice(g1.start(), g1.end()), column_adjustment));
                            text = text.slice(0, g0.start());
                        }
                    }

                    // handle explicitly defined IDs for headlines
                    if let Some(&(cm::Event::End(cm::Tag::Heading(_)), _)) = iter.peek() {
                        if let Some(m) = HEADING_ID_RE.captures(text.as_str()) {
                            let g0 = m.get(0).unwrap();
                            let g1 = m.get(1).unwrap();

                            // adjust the span of the text to not include the role.
                            let column_adjustment = g0.end() - g0.start();
                            loc.len -= column_adjustment;
                            pending_trailer = Some(Trailer::Id(text.slice(g1.start(), g1.end())));
                            text = text.slice(0, g0.start());
                        }
                    }

                    EventType::Text { text }
                }
                cm::Event::Code(value) => {
                    // if there is a pending role then we're not working with a
                    // code block, but an interpreted text one.
                    if let Some((role, column_adjustment)) = pending_role.take() {
                        loc.column -= column_adjustment;
                        loc.offset -= column_adjustment;
                        loc.len += column_adjustment;
                        EventType::InterpretedText {
                            text: value.into(),
                            role: role.into(),
                        }
                    } else {
                        EventType::InlineCode { code: value.into() }
                    }
                }
                cm::Event::Html(html) => EventType::RawHtml { html: html.into() },
                cm::Event::FootnoteReference(target) => EventType::FootnoteReference {
                    target: target.into(),
                },
                cm::Event::SoftBreak => EventType::SoftBreak,
                cm::Event::HardBreak => EventType::HardBreak,
                cm::Event::Rule => EventType::Rule,
                cm::Event::TaskListMarker(checked) => EventType::Checkbox { checked },
            };

            Some((Event::new(ty, loc), trailer))
        } else {
            None
        }
    })
}

/// Recursively attaches trailers to start tags.
fn buffer_for_trailers<'data, I>(event: Event<'data>, iter: &mut I) -> Vec<Event<'data>>
where
    I: Iterator<Item = (Event<'data>, Option<Trailer<'data>>)>,
{
    let mut buffer = vec![event];
    let mut depth = 1;

    while let Some((event, trailer)) = iter.next() {
        // keep track of the tag depth
        match event.ty() {
            &EventType::StartTag { tag, .. } => {
                if tag_supports_trailers(tag) {
                    buffer.extend(buffer_for_trailers(event, iter));
                    continue;
                } else {
                    depth += 1;
                }
            }
            &EventType::EndTag { .. } => depth -= 1,
            _ => {}
        }
        buffer.push(event);

        // attach an end tag trailer to the start tag if needed.
        if depth == 0 {
            if let EventType::StartTag { attrs, .. } = buffer[0].ty_mut() {
                match trailer {
                    Some(Trailer::Id(new_id)) => {
                        attrs.id = Some(new_id);
                    }
                    None => {}
                }
            }
            break;
        }
    }

    buffer
}

/// Parses structured cmark into an event stream.
pub fn parse(s: &str) -> impl Iterator<Item = Event> {
    let mut iter = parse_with_trailers(s);
    iter::from_fn(move || {
        if let Some((event, _)) = iter.next() {
            if let &EventType::StartTag { tag, .. } = event.ty() {
                if tag_supports_trailers(tag) {
                    return Some(Either::Left(
                        buffer_for_trailers(event, &mut iter).into_iter(),
                    ));
                }
            }
            Some(Either::Right(iter::once(event)))
        } else {
            None
        }
    })
    .flatten()
}
