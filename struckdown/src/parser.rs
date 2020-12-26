//! Gives access to the stream parser.
use std::collections::BTreeMap;
use std::iter::{self, once};
use std::ops::Range;

use either::Either;
use lazy_static::lazy_static;
use pulldown_cmark as cm;
use regex::Regex;

use crate::event::{
    Alignment, AnnotatedEvent, Attrs, CheckboxEvent, CodeBlockEvent, DirectiveEvent,
    DocumentStartEvent, EndTagEvent, Event, FootnoteReferenceEvent, ImageEvent, InlineCodeEvent,
    InterpretedTextEvent, Location, RawHtmlEvent, StartTagEvent, Str, Tag, TextEvent,
};
use crate::value::Value;

lazy_static! {
    static ref TEXT_ROLE_RE: Regex = Regex::new(r"\{([^\r\n\}]+)\}$").unwrap();
    static ref DIRECTIVE_RE: Regex = Regex::new(r"^\{([^\r\n\}]+)\}(?:\s+(.*?))?$").unwrap();
    static ref HEADING_ID_RE: Regex = Regex::new(r"\s+\{#([^\r\n\}]+)\}\s*$").unwrap();
    static ref FRONTMATTER_RE: Regex = Regex::new(r"(?sm)\A---\s*$(.*?)^---\s*$\r?\n?").unwrap();
    static ref FRONTMATTER_FULL_RE: Regex = Regex::new(r"(?sm)\A---\s*$(.*)").unwrap();
    static ref CODE_LANG_RE: Regex = Regex::new(r#"(\S+)\s+"#).unwrap();
    static ref CODE_ARG_RE: Regex = Regex::new(r#"([^=\s]+)(?:="([^"]*)"|\S+)?"#).unwrap();
}

/// Configures the parser.
///
/// By default all features are enabled.
#[derive(Debug, Clone)]
pub struct ParserOptions {
    /// Enables or disables front matter.
    pub enable_frontmatter: bool,
    /// Enables or disables directives.
    pub enable_directives: bool,
    /// Enables or disables roles.
    pub enable_roles: bool,
    /// Enables or disables tables.
    pub enable_tables: bool,
    /// Enables or disables strikethrough.
    pub enable_strikethrough: bool,
    /// Enables or disables task lists.
    pub enable_tasklists: bool,
    /// Enables or disables footnotes.
    pub enable_footnotes: bool,
    /// Enables or disables explicit anchors.
    pub enable_anchors: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        ParserOptions {
            enable_frontmatter: true,
            enable_directives: true,
            enable_roles: true,
            enable_tables: true,
            enable_strikethrough: true,
            enable_tasklists: true,
            enable_footnotes: true,
            enable_anchors: true,
        }
    }
}

/// A configurable parser for struckdown.
pub struct Parser {
    options: ParserOptions,
}

impl Default for Parser {
    fn default() -> Self {
        Parser::new(&ParserOptions::default())
    }
}

impl Parser {
    /// Creates a new parser with specific options.
    pub fn new(options: &ParserOptions) -> Parser {
        Parser {
            options: options.clone(),
        }
    }

    /// Parses structured cmark into an event stream.
    ///
    /// The given structured cmark input will be parsed into a well formed event
    /// stream.  Events are wrapped in the [`AnnotatedEvent`] struct which provides
    /// appropriate location information.
    ///
    /// Note that even without any further processing not all events that come out
    /// of a markdown document will have a location information set.  For instance
    /// tables produce a [`Tag::TableHeader`] and [`Tag::TableBody`] wrapper which
    /// does not exist as such in the source.  Both of those will not have a location
    /// attached.
    pub fn parse<'data>(&self, s: &'data str) -> impl Iterator<Item = AnnotatedEvent<'data>> {
        parse_internal(s, self.options.clone())
    }
}

/// Reads until the end of a tag and read embedded content as raw string.
///
/// We do this because in markdown/cmark the alt text of an image is in fact
/// supporting a lot of markdown syntax but it usually gets rendered out into
/// an alt attribute.  Because of that we normalize this into text during parsing
/// already so that stream processors don't need to deal with this oddity.  Same
/// applies to reading code blocks.
fn read_raw<'a, 'data, I: Iterator<Item = (cm::Event<'data>, Range<usize>)>>(
    iter: &'a mut I,
) -> Str<'data> {
    let mut depth = 1;
    let mut buffer = None;
    let mut last_event = None;

    macro_rules! ensure_buf {
        () => {
            match buffer {
                Some(ref mut buffer) => buffer,
                None => {
                    let mut buf = String::new();
                    if let Some(last_event) = last_event.take() {
                        buf.push_str(&last_event);
                    }
                    buffer = Some(buf);
                    buffer.as_mut().unwrap()
                }
            }
        };
    }

    macro_rules! push {
        ($expr:expr) => {
            if last_event.is_none() && buffer.is_none() {
                last_event = Some($expr);
            } else {
                ensure_buf!().push_str(&$expr);
            }
        };
    }

    for (event, _) in iter {
        match event {
            cm::Event::Start(..) => depth += 1,
            cm::Event::End(..) => depth -= 1,
            cm::Event::Text(text) => push!(text),
            cm::Event::Code(code) => push!(code),
            cm::Event::SoftBreak | cm::Event::HardBreak => ensure_buf!().push('\n'),
            _ => {}
        }
        if depth == 0 {
            break;
        }
    }

    match (buffer, last_event) {
        (Some(buf), _) => buf.into(),
        (None, Some(event)) => Str::from_cm_str(event),
        (None, None) => "".into(),
    }
}

/// parse front matter in some text
fn split_and_parse_front_matter(source: Str<'_>) -> (Option<Value>, Str<'_>) {
    if let Some(m) = FRONTMATTER_RE.captures(source.as_str()) {
        let g0 = m.get(0).unwrap();
        if let Ok(front_matter) = serde_yaml::from_str(&m[1]) {
            return (
                Some(front_matter),
                source.slice(g0.end(), source.as_str().len()),
            );
        }
    } else if let Some(m) = FRONTMATTER_FULL_RE.captures(source.as_str()) {
        if let Ok(front_matter) = serde_yaml::from_str(&m[1]) {
            return (Some(front_matter), "".into());
        }
    }

    (None, source)
}

/// A trailer is information that gets attached to the start tag when the end
/// tag is emitted.
///
/// Trailers are supported internally on all tags for which [`tag_supports_trailers`]
/// returns `true`.
enum Trailer<'data> {
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

fn split_code_block_args<'data>(
    info: Str<'data>,
) -> (Option<Str<'data>>, Option<BTreeMap<Str<'data>, Str<'data>>>) {
    if info.as_str().trim().is_empty() {
        return (None, None);
    }

    let (code, arg_str) = if let Some(m) = CODE_LANG_RE.captures(info.as_str()) {
        let g0 = m.get(0).unwrap();
        let g1 = m.get(1).unwrap();
        (
            info.slice(g1.start(), g1.end()),
            info.slice(g0.end(), info.as_str().len()),
        )
    } else {
        return (Some(info), None);
    };

    let mut args = BTreeMap::new();
    for m in CODE_ARG_RE.captures_iter(arg_str.as_str()) {
        let g1 = m.get(1).unwrap();
        let key = arg_str.slice(g1.start(), g1.end());
        let value = if let Some(g2) = m.get(2) {
            arg_str.slice(g2.start(), g2.end())
        } else if let Some(g3) = m.get(3) {
            arg_str.slice(g3.start(), g3.end())
        } else {
            Str::from("")
        };
        args.insert(key, value);
    }

    (Some(code), if args.is_empty() { None } else { Some(args) })
}

// helper for table state
struct TableState {
    alignments: Vec<Alignment>,
    cell_is_head: bool,
    cell_index: usize,
}

/// Parses a string into an event stream with trailers.
///
/// This is normally not used as the `parse` function will already buffer
/// as necessary to automatically attach the trailers to the start tags.
///
/// The output of this parsing function still largely reflects the cmark
/// stream in structure though some elements are already resolved.  The
/// main parse function however will attach some virtual elements such as
/// table bodies which are not there in regular cmark.
fn preliminary_parse_with_trailers<'data>(
    s: &'data str,
    options: ParserOptions,
) -> impl Iterator<Item = (AnnotatedEvent, Option<Trailer<'data>>)> {
    let mut opts = cm::Options::empty();
    if options.enable_tables {
        opts.insert(cm::Options::ENABLE_TABLES);
    }
    if options.enable_strikethrough {
        opts.insert(cm::Options::ENABLE_STRIKETHROUGH);
    }
    if options.enable_tasklists {
        opts.insert(cm::Options::ENABLE_TASKLISTS);
    }
    if options.enable_footnotes {
        opts.insert(cm::Options::ENABLE_FOOTNOTES);
    }

    let parser = cm::Parser::new_with_broken_link_callback(s, opts, None);
    let mut iter = parser.into_offset_iter().peekable();
    let mut tag_stack = vec![];
    let mut pending_role = None;
    let mut pending_trailer = None;
    let mut table_state = None;

    iter::from_fn(move || {
        let mut trailer = None;

        if let Some((event, range)) = iter.next() {
            // inefficient way to find the location
            let mut location = Some(Location {
                offset: range.start,
                len: range.end - range.start,
                line: s[..range.start].chars().filter(|&c| c == '\n').count() + 1,
                column: match s[..range.start].rfind('\n') {
                    Some(nl) => range.start - nl - 1,
                    None => range.start,
                },
            });

            // simple events
            let event = match event {
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
                                let lang = Str::from_cm_str(lang);
                                if options.enable_directives {
                                    if let Some(m) = DIRECTIVE_RE.captures(lang.as_str()) {
                                        let g1 = m.get(1).unwrap();
                                        let arg = if let Some(g2) = m.get(2) {
                                            lang.slice(g2.start(), g2.end())
                                        } else {
                                            "".into()
                                        };
                                        let body = read_raw(&mut iter);
                                        let (front_matter, body) =
                                            split_and_parse_front_matter(body);
                                        return Some((
                                            AnnotatedEvent::new(
                                                DirectiveEvent {
                                                    name: lang.slice(g1.start(), g1.end()),
                                                    argument: if arg.as_str().is_empty() {
                                                        None
                                                    } else {
                                                        Some(arg)
                                                    },
                                                    front_matter,
                                                    body,
                                                },
                                                location,
                                            ),
                                            None,
                                        ));
                                    }
                                }
                                let code = read_raw(&mut iter);
                                let (language, args) = split_code_block_args(lang);
                                return Some((
                                    AnnotatedEvent::new(
                                        CodeBlockEvent {
                                            language,
                                            args,
                                            code,
                                        },
                                        location,
                                    ),
                                    None,
                                ));
                            }
                            cm::CodeBlockKind::Indented => {
                                let code = read_raw(&mut iter);
                                return Some((
                                    AnnotatedEvent::new(
                                        CodeBlockEvent {
                                            language: None,
                                            args: None,
                                            code,
                                        },
                                        location,
                                    ),
                                    None,
                                ));
                            }
                        },
                        cm::Tag::List(None) => Tag::UnorderedList,
                        cm::Tag::List(Some(start)) => {
                            attrs.start = Some(start as u32);
                            Tag::OrderedList
                        }
                        cm::Tag::Item => Tag::ListItem,
                        cm::Tag::FootnoteDefinition(id) => {
                            attrs.id = Some(Str::from_cm_str(id));
                            Tag::FootnoteDefinition
                        }
                        cm::Tag::Table(alignments) => {
                            table_state = Some(TableState {
                                alignments: alignments
                                    .into_iter()
                                    .map(|cm_align| match cm_align {
                                        cm::Alignment::None => Alignment::None,
                                        cm::Alignment::Left => Alignment::Left,
                                        cm::Alignment::Center => Alignment::Center,
                                        cm::Alignment::Right => Alignment::Right,
                                    })
                                    .collect(),
                                cell_is_head: false,
                                cell_index: 0,
                            });
                            Tag::Table
                        }
                        cm::Tag::TableHead => {
                            let state = table_state.as_mut().expect("not in table");
                            state.cell_index = 0;
                            state.cell_is_head = true;
                            // do not emit location information for table headers.  We consider
                            // this to be a purely internal event same as with the table body
                            // event emitted by the outer parse function.
                            location = None;
                            Tag::TableHeader
                        }
                        cm::Tag::TableRow => {
                            let state = table_state.as_mut().expect("not in table");
                            state.cell_index = 0;
                            state.cell_is_head = false;
                            Tag::TableRow
                        }
                        cm::Tag::TableCell => {
                            let state = table_state.as_mut().expect("not in table");
                            attrs.alignment = state
                                .alignments
                                .get(state.cell_index)
                                .copied()
                                .unwrap_or(Alignment::None);
                            state.cell_index += 1;
                            if state.cell_is_head {
                                Tag::TableHead
                            } else {
                                Tag::TableCell
                            }
                        }
                        cm::Tag::Emphasis => {
                            if &s[range.start..range.start + 1] == "_" {
                                Tag::EmphasisAlt
                            } else {
                                Tag::Emphasis
                            }
                        }
                        cm::Tag::Strong => Tag::Strong,
                        cm::Tag::Strikethrough => Tag::Strikethrough,
                        cm::Tag::Link(_, target, title) => {
                            attrs.target = Some(Str::from_cm_str(target));
                            if !title.is_empty() {
                                attrs.title = Some(Str::from_cm_str(title));
                            }
                            Tag::Link
                        }
                        cm::Tag::Image(_, target, title) => {
                            // images are special in that we downgrade them from
                            // tags to toplevel events to not have to deal with
                            // nested text.
                            let alt = read_raw(&mut iter);
                            return Some((
                                AnnotatedEvent::new(
                                    ImageEvent {
                                        target: Str::from_cm_str(target),
                                        alt: if alt.as_str().is_empty() {
                                            None
                                        } else {
                                            Some(alt)
                                        },
                                        title: if title.is_empty() {
                                            None
                                        } else {
                                            Some(Str::from_cm_str(title))
                                        },
                                    },
                                    location,
                                ),
                                None,
                            ));
                        }
                    };
                    tag_stack.push(tag);
                    StartTagEvent { tag, attrs }.into()
                }
                cm::Event::End(_) => {
                    trailer = pending_trailer.take();
                    EndTagEvent {
                        tag: tag_stack.pop().unwrap(),
                    }
                    .into()
                }
                cm::Event::Text(text) => {
                    let mut text = Str::from_cm_str(text);

                    // handle roles
                    if options.enable_roles {
                        if let Some(&(cm::Event::Code(_), _)) = iter.peek() {
                            if let Some(m) = TEXT_ROLE_RE.captures(text.as_str()) {
                                let g0 = m.get(0).unwrap();
                                let g1 = m.get(1).unwrap();

                                // adjust the span of the text to not include the role.
                                let column_adjustment = g0.end() - g0.start();
                                if let Some(ref mut location) = location {
                                    location.len -= column_adjustment;
                                }
                                pending_role =
                                    Some((text.slice(g1.start(), g1.end()), column_adjustment));
                                text = text.slice(0, g0.start());
                            }
                        }
                    }

                    // handle explicitly defined IDs for headlines
                    if options.enable_anchors {
                        if let Some(&(cm::Event::End(cm::Tag::Heading(_)), _)) = iter.peek() {
                            if let Some(m) = HEADING_ID_RE.captures(text.as_str()) {
                                let g0 = m.get(0).unwrap();
                                let g1 = m.get(1).unwrap();

                                // adjust the span of the text to not include the role.
                                let column_adjustment = g0.end() - g0.start();
                                if let Some(ref mut location) = location {
                                    location.len -= column_adjustment;
                                }
                                pending_trailer =
                                    Some(Trailer::Id(text.slice(g1.start(), g1.end())));
                                text = text.slice(0, g0.start());
                            }
                        }
                    }

                    TextEvent { text }.into()
                }
                cm::Event::Code(value) => {
                    // if there is a pending role then we're not working with a
                    // code block, but an interpreted text one.
                    if let Some((role, column_adjustment)) = pending_role.take() {
                        if let Some(ref mut location) = location {
                            location.column -= column_adjustment;
                            location.offset -= column_adjustment;
                            location.len += column_adjustment;
                        }
                        InterpretedTextEvent {
                            text: Str::from_cm_str(value),
                            role,
                        }
                        .into()
                    } else {
                        InlineCodeEvent {
                            code: Str::from_cm_str(value),
                        }
                        .into()
                    }
                }
                cm::Event::Html(html) => RawHtmlEvent {
                    html: Str::from_cm_str(html),
                }
                .into(),
                cm::Event::FootnoteReference(target) => FootnoteReferenceEvent {
                    target: Str::from_cm_str(target),
                }
                .into(),
                cm::Event::SoftBreak => Event::SoftBreak,
                cm::Event::HardBreak => Event::HardBreak,
                cm::Event::Rule => Event::Rule,
                cm::Event::TaskListMarker(checked) => CheckboxEvent { checked }.into(),
            };

            Some((AnnotatedEvent::new(event, location), trailer))
        } else {
            None
        }
    })
}

/// Recursively attaches trailers to start tags.
fn buffer_for_trailers<'data, I>(
    event: AnnotatedEvent<'data>,
    iter: &mut I,
) -> Vec<AnnotatedEvent<'data>>
where
    I: Iterator<Item = (AnnotatedEvent<'data>, Option<Trailer<'data>>)>,
{
    let mut buffer = vec![event];
    let mut depth = 1;

    while let Some((event, trailer)) = iter.next() {
        // keep track of the tag depth
        match event.event {
            Event::StartTag(StartTagEvent { tag, .. }) => {
                if tag_supports_trailers(tag) {
                    buffer.extend(buffer_for_trailers(event, iter));
                    continue;
                } else {
                    depth += 1;
                }
            }
            Event::EndTag { .. } => depth -= 1,
            _ => {}
        }
        buffer.push(event);

        // attach an end tag trailer to the start tag if needed.
        if depth == 0 {
            if let Event::StartTag(StartTagEvent { ref mut attrs, .. }) = buffer[0].event {
                if let Some(Trailer::Id(new_id)) = trailer {
                    attrs.id = Some(new_id);
                }
            }
            break;
        }
    }

    buffer
}

fn parse_internal(s: &str, options: ParserOptions) -> impl Iterator<Item = AnnotatedEvent> {
    let mut front_matter = None;
    let mut s = s;
    let mut front_matter_location = None;

    if options.enable_frontmatter {
        if let Some(m) = FRONTMATTER_RE.captures(s) {
            if let Ok(parsed_front_matter) = serde_yaml::from_str(&m[1]) {
                let g0 = m.get(0).unwrap();
                front_matter = Some(parsed_front_matter);
                front_matter_location = Some(Location {
                    offset: 0,
                    len: g0.end(),
                    line: 1,
                    column: 0,
                });
                s = &s[g0.end()..];
            }
        }
    }

    let mut iter = preliminary_parse_with_trailers(s, options);

    once(AnnotatedEvent::new(
        DocumentStartEvent { front_matter },
        front_matter_location,
    ))
    .chain(
        iter::from_fn(move || {
            iter.next().map(|(annotated_event, _)| {
                if let Event::StartTag(StartTagEvent { tag, .. }) = annotated_event.event {
                    if tag_supports_trailers(tag) {
                        return Either::Left(
                            buffer_for_trailers(annotated_event, &mut iter).into_iter(),
                        );
                    }
                }
                Either::Right(once(annotated_event))
            })
        })
        .flatten()
        .flat_map(|annotated_event| match annotated_event.event {
            // after a table header we inject an implied table body.
            Event::EndTag(EndTagEvent {
                tag: Tag::TableHeader,
            }) => Either::Left(
                once(annotated_event).chain(once(
                    Event::StartTag(StartTagEvent {
                        tag: Tag::TableBody,
                        attrs: Default::default(),
                    })
                    .into(),
                )),
            ),
            // just before the table end, we close the table body.
            Event::EndTag(EndTagEvent { tag: Tag::Table }) => Either::Left(
                once(
                    Event::EndTag(EndTagEvent {
                        tag: Tag::TableBody,
                    })
                    .into(),
                )
                .chain(once(annotated_event)),
            ),
            _ => Either::Right(once(annotated_event)),
        }),
    )
}

/// Parses structured cmark into an event stream.
pub fn parse<'data, 'options>(
    s: &'data str,
    options: &'options ParserOptions,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    Parser::new(options).parse(s)
}
