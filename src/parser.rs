use std::ops::Range;

use lazy_static::lazy_static;
use pulldown_cmark as cm;
use regex::Regex;

use crate::event::{Alignment, Attrs, Event, EventType, Location, Str, Tag};

lazy_static! {
    static ref TEXT_ROLE_RE: Regex = Regex::new(r"\{([^\r\n\}]+)\}$").unwrap();
    static ref CODE_ROLE_RE: Regex = Regex::new(r"^\{([^\r\n\}]+)\}(?:\s+(.*?))$").unwrap();
    static ref HEADING_ID_RE: Regex = Regex::new(r"{#([^\r\n\}]+)\}\s*$").unwrap();
}

/// Reads until the end of an image tag all embedded content as raw string.
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

/// Parses a structured cmark document into an event stream.
pub fn parse(s: &str) -> impl Iterator<Item = Event> {
    let mut opts = cm::Options::empty();
    opts.insert(cm::Options::ENABLE_TABLES);
    opts.insert(cm::Options::ENABLE_STRIKETHROUGH);
    opts.insert(cm::Options::ENABLE_TASKLISTS);
    opts.insert(cm::Options::ENABLE_FOOTNOTES);

    let parser = cm::Parser::new_with_broken_link_callback(s, opts, None);
    let mut iter = parser.into_offset_iter().peekable();
    let mut tag_stack = vec![];
    let mut pending_role = None;

    std::iter::from_fn(move || {
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
                            return Some(Event::new(
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
                            ));
                        }
                    };
                    tag_stack.push(tag);
                    EventType::StartTag { tag, attrs }
                }
                cm::Event::End(_) => EventType::EndTag {
                    tag: tag_stack.pop().unwrap(),
                },
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

            Some(Event::new(ty, loc))
        } else {
            None
        }
    })
}
