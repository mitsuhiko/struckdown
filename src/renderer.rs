use std::collections::HashMap;
use std::io::{self, Write};

use v_htmlescape::escape;

use crate::event::{Alignment, Attrs, Event, EventType, Str, Tag};

pub struct TableState {
    alignments: Vec<Alignment>,
    cell_is_head: bool,
    cell_index: usize,
}

pub struct HtmlRenderer<'data, F> {
    out: F,
    table_state: Option<TableState>,
    footnotes: HashMap<Str<'data>, usize>,
}

fn is_block_tag(tag: Tag) -> bool {
    match tag {
        Tag::Paragraph => true,
        Tag::Heading1 => true,
        Tag::Heading2 => true,
        Tag::Heading3 => true,
        Tag::Heading4 => true,
        Tag::Heading5 => true,
        Tag::Heading6 => true,
        Tag::BlockQuote => true,
        Tag::IndentedCode => true,
        Tag::FencedCode => true,
        Tag::OrderedList => true,
        Tag::UnorderedList => true,
        Tag::ListItem => true,
        Tag::FootnoteDefinition => true,
        Tag::Table => true,
        Tag::TableHead => true,
        Tag::TableRow => true,
        Tag::TableCell => true,
        Tag::Emphasis => false,
        Tag::Strong => false,
        Tag::Strikethrough => false,
        Tag::Link => false,
        Tag::Directive => true,
    }
}

impl<'data, F: Write> HtmlRenderer<'data, F> {
    pub fn new(out: F) -> HtmlRenderer<'data, F> {
        HtmlRenderer {
            out,
            table_state: None,
            footnotes: HashMap::new(),
        }
    }

    fn tag_to_html_tag(&self, tag: Tag) -> &'static str {
        match tag {
            Tag::Paragraph => "p",
            Tag::Heading1 => "h1",
            Tag::Heading2 => "h2",
            Tag::Heading3 => "h3",
            Tag::Heading4 => "h4",
            Tag::Heading5 => "h5",
            Tag::Heading6 => "h6",
            Tag::BlockQuote => "blockquote",
            Tag::IndentedCode => "pre",
            Tag::FencedCode => "pre",
            Tag::OrderedList => "ol",
            Tag::UnorderedList => "ul",
            Tag::ListItem => "li",
            Tag::FootnoteDefinition => "div",
            Tag::Table => "table",
            Tag::TableHead => "thead",
            Tag::TableRow => "tr",
            Tag::TableCell => match self.table_state.as_ref().map(|x| x.cell_is_head) {
                None | Some(false) => "td",
                Some(true) => "th",
            },
            Tag::Emphasis => "em",
            Tag::Strong => "strong",
            Tag::Strikethrough => "ss",
            Tag::Link => "a",
            Tag::Directive => "div",
        }
    }

    fn start_tag(&mut self, tag: Tag, attrs: &Attrs<'_>) -> Result<(), io::Error> {
        let html_tag = self.tag_to_html_tag(tag);
        write!(self.out, "<{}", html_tag)?;

        match attrs.start() {
            None | Some(1) => {}
            Some(start) => {
                write!(self.out, " start={}", start)?;
            }
        }
        if let Some(id) = attrs.id() {
            write!(self.out, " id=\"{}\"", id)?;
        }
        if let Some(title) = attrs.title() {
            write!(self.out, " title=\"{}\"", title)?;
        }
        if let Some(target) = attrs.target() {
            write!(self.out, " href=\"{}\"", escape(target))?;
        }
        for (key, value) in attrs.iter_custom() {
            write!(self.out, " {}=\"{}\"", key, escape(value.as_str()))?;
        }

        match tag {
            Tag::Table => {
                self.table_state = Some(TableState {
                    alignments: attrs.alignments().iter().copied().collect(),
                    cell_is_head: false,
                    cell_index: 0,
                });
            }
            Tag::TableHead => {
                if let Some(ref mut state) = self.table_state {
                    state.cell_is_head = true;
                    state.cell_index = 0;
                }
            }
            Tag::TableRow => {
                if let Some(ref mut state) = self.table_state {
                    state.cell_index = 0;
                }
            }
            Tag::TableCell => {
                match self
                    .table_state
                    .as_ref()
                    .and_then(|x| x.alignments.get(x.cell_index).copied())
                {
                    Some(Alignment::Left) => write!(self.out, " align=left")?,
                    Some(Alignment::Center) => write!(self.out, " align=center")?,
                    Some(Alignment::Right) => write!(self.out, " align=right")?,
                    Some(Alignment::None) | None => {}
                }
            }
            _ => {}
        }

        write!(self.out, ">")?;

        Ok(())
    }

    fn end_tag(&mut self, tag: Tag) -> Result<(), io::Error> {
        let html_tag = self.tag_to_html_tag(tag);

        if tag == Tag::Table {
            write!(self.out, "</tbody>")?;
        }

        write!(
            self.out,
            "</{}>{}",
            html_tag,
            if is_block_tag(tag) { "\n" } else { "" }
        )?;

        match tag {
            Tag::Table => {
                self.table_state = None;
            }
            Tag::TableHead => {
                write!(self.out, "<tbody>")?;
            }
            Tag::TableCell => {
                if let Some(ref mut state) = self.table_state {
                    state.cell_index += 1;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn event(&mut self, event: &Event<'data>) -> Result<(), io::Error> {
        match *event.ty() {
            EventType::StartTag { tag, ref attrs } => {
                self.start_tag(tag, attrs)?;
            }
            EventType::EndTag { tag } => {
                self.end_tag(tag)?;
            }
            EventType::Text { ref text } => {
                write!(self.out, "{}", escape(text.as_str()))?;
            }
            EventType::InterpretedText { ref text, ref role } => {
                write!(
                    self.out,
                    "<span class=\"role-{}\">{}</span>",
                    escape(role.as_str()),
                    escape(text.as_str())
                )?;
            }
            EventType::InlineCode { ref code } => {
                write!(self.out, "<code>{}</code>", escape(code.as_str()))?;
            }
            EventType::Image {
                ref target,
                ref alt,
                ref title,
            } => {
                write!(
                    self.out,
                    "<img src=\"{}\" alt=\"{}\" title=\"{}\">",
                    target,
                    alt.as_ref().map_or("", |x| x.as_str()),
                    title.as_ref().map_or("", |x| x.as_str()),
                )?;
            }
            EventType::RawHtml { ref html } => {
                write!(self.out, "{}", html)?;
            }
            EventType::SoftBreak => {}
            EventType::HardBreak => {}
            EventType::Rule => {
                write!(self.out, "<hr>")?;
            }
            EventType::Checkbox { checked } => {
                write!(
                    self.out,
                    "<input type=checkbox disabled{}>",
                    if checked { " checked" } else { "" }
                )?;
            }
            EventType::FootnoteReference { ref target } => {
                let number = match self.footnotes.get(target) {
                    Some(&num) => num,
                    None => {
                        let next_number = self.footnotes.len() + 1;
                        self.footnotes.insert(target.clone(), next_number);
                        next_number
                    }
                };
                write!(
                    self.out,
                    "<sup class=footnote-reference><a href=\"#{}\">{}</a></sup>",
                    escape(target.as_str()),
                    number,
                )?;
            }
        }
        Ok(())
    }
}

pub fn to_html<'a, I: Iterator<Item = Event<'a>>>(iter: I) -> String {
    let mut out = Vec::<u8>::new();
    let mut renderer = HtmlRenderer::new(&mut out);
    for event in iter {
        renderer.event(&event).unwrap();
    }
    unsafe { String::from_utf8_unchecked(out) }
}