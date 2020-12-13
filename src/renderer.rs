use std::collections::HashMap;
use std::io::{self, Write};

use v_htmlescape::escape;

use crate::event::{
    Alignment, AnnotatedEvent, Attrs, CheckboxEvent, CodeBlockEvent, DirectiveEvent, EndTagEvent,
    Event, FootnoteReferenceEvent, ImageEvent, InlineCodeEvent, InterpretedTextEvent, RawHtmlEvent,
    StartTagEvent, Str, Tag, TextEvent,
};

pub struct HtmlRenderer<'data, F> {
    out: F,
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
        Tag::TableHeader => true,
        Tag::TableBody => true,
    }
}

impl<'data, F: Write> HtmlRenderer<'data, F> {
    pub fn new(out: F) -> HtmlRenderer<'data, F> {
        HtmlRenderer {
            out,
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
            Tag::OrderedList => "ol",
            Tag::UnorderedList => "ul",
            Tag::ListItem => "li",
            Tag::FootnoteDefinition => "div",
            Tag::Table => "table",
            Tag::TableHeader => "thead",
            Tag::TableBody => "tbody",
            Tag::TableRow => "tr",
            Tag::TableHead => "th",
            Tag::TableCell => "td",
            Tag::Emphasis => "em",
            Tag::Strong => "strong",
            Tag::Strikethrough => "ss",
            Tag::Link => "a",
        }
    }

    fn start_tag(&mut self, tag: Tag, attrs: &Attrs) -> Result<(), io::Error> {
        let html_tag = self.tag_to_html_tag(tag);
        write!(self.out, "<{}", html_tag)?;

        match attrs.start {
            None | Some(1) => {}
            Some(start) => {
                write!(self.out, " start={}", start)?;
            }
        }

        let mut combined_style = String::new();
        if let Some(ref id) = attrs.id {
            write!(self.out, " id=\"{}\"", escape(id.as_str()))?;
        }
        if let Some(ref title) = attrs.title {
            write!(self.out, " title=\"{}\"", escape(title.as_str()))?;
        }
        if let Some(ref target) = attrs.target {
            write!(self.out, " href=\"{}\"", escape(target.as_str()))?;
        }

        combined_style.push_str(match attrs.alignment {
            Alignment::None => "",
            Alignment::Left => "text-align: left",
            Alignment::Center => "text-align: center",
            Alignment::Right => "text-align: right",
        });

        if let Some(ref custom) = attrs.custom {
            for (key, value) in custom.iter() {
                if key == "style" {
                    if !combined_style.is_empty() {
                        combined_style.push_str("; ");
                    }
                    combined_style.push_str(value.as_str());
                } else {
                    write!(self.out, " {}=\"{}\"", key, escape(value.as_str()))?;
                }
            }
        }

        if !combined_style.is_empty() {
            write!(self.out, " style=\"{}\"", escape(&combined_style))?;
        }

        write!(self.out, ">")?;

        Ok(())
    }

    fn end_tag(&mut self, tag: Tag) -> Result<(), io::Error> {
        let html_tag = self.tag_to_html_tag(tag);

        write!(
            self.out,
            "</{}>{}",
            html_tag,
            if is_block_tag(tag) { "\n" } else { "" }
        )?;

        Ok(())
    }

    pub fn event(&mut self, event: &AnnotatedEvent<'data>) -> Result<(), io::Error> {
        match event.event {
            Event::StartTag(StartTagEvent { tag, ref attrs }) => {
                self.start_tag(tag, attrs)?;
            }
            Event::EndTag(EndTagEvent { tag }) => {
                self.end_tag(tag)?;
            }
            Event::Text(TextEvent { ref text }) => {
                write!(self.out, "{}", escape(text.as_str()))?;
            }
            Event::CodeBlock(CodeBlockEvent {
                ref code,
                ref language,
            }) => {
                write!(self.out, "<pre><code")?;
                if let Some(language) = language {
                    write!(self.out, " class=\"lang-{}\"", language.as_str())?;
                }
                write!(self.out, ">{}</code></pre>", escape(code.as_str()))?;
            }
            Event::Directive(DirectiveEvent {
                ref name, ref body, ..
            }) => {
                write!(
                    self.out,
                    "<div class=\"directive-{}\"><pre>{}</pre></div>",
                    escape(name.as_str()),
                    escape(body.as_str()),
                )?;
            }
            Event::InterpretedText(InterpretedTextEvent { ref text, ref role }) => {
                write!(
                    self.out,
                    "<span class=\"role-{}\">{}</span>",
                    escape(role.as_str()),
                    escape(text.as_str())
                )?;
            }
            Event::InlineCode(InlineCodeEvent { ref code }) => {
                write!(self.out, "<code>{}</code>", escape(code.as_str()))?;
            }
            Event::Image(ImageEvent {
                ref target,
                ref alt,
                ref title,
            }) => {
                write!(
                    self.out,
                    "<img src=\"{}\" alt=\"{}\" title=\"{}\">",
                    target,
                    alt.as_ref().map_or("", |x| x.as_str()),
                    title.as_ref().map_or("", |x| x.as_str()),
                )?;
            }
            Event::RawHtml(RawHtmlEvent { ref html }) => {
                write!(self.out, "{}", html)?;
            }
            Event::SoftBreak => {}
            Event::HardBreak => {}
            Event::Rule => {
                write!(self.out, "<hr>")?;
            }
            Event::Checkbox(CheckboxEvent { checked }) => {
                write!(
                    self.out,
                    "<input type=checkbox disabled{}>",
                    if checked { " checked" } else { "" }
                )?;
            }
            Event::FootnoteReference(FootnoteReferenceEvent { ref target }) => {
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

pub fn to_html<'a, I: Iterator<Item = AnnotatedEvent<'a>>>(iter: I) -> String {
    let mut out = Vec::<u8>::new();
    let mut renderer = HtmlRenderer::new(&mut out);
    for event in iter {
        renderer.event(&event).unwrap();
    }
    unsafe { String::from_utf8_unchecked(out) }
}
