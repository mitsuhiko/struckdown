//! Implements an HTML renderer.
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use v_htmlescape::escape;

use crate::event::{
    Alignment, AnnotatedEvent, Attrs, CheckboxEvent, CodeBlockEvent, DirectiveEvent, EndTagEvent,
    ErrorEvent, Event, FootnoteReferenceEvent, ImageEvent, InlineCodeEvent, InterpretedTextEvent,
    RawHtmlEvent, StartTagEvent, Str, Tag, TextEvent,
};

/// Customizes the HTML rendering.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct HtmlRendererOptions {
    /// When enabled `_foo_` renders into underlines.
    pub render_underlines: bool,
    /// The class to emit for footnote references.
    pub footnote_reference_class: String,
    /// The class to emit for footnote definitions.
    pub footnote_definition_class: String,
    /// The initial level for headlines
    pub initial_headline_level: usize,
}

impl Default for HtmlRendererOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlRendererOptions {
    /// Creates the default configuraiton for the renderer options.
    pub fn new() -> HtmlRendererOptions {
        HtmlRendererOptions {
            render_underlines: false,
            footnote_reference_class: "footnote-reference".into(),
            footnote_definition_class: "footnote-definition".into(),
            initial_headline_level: 1,
        }
    }
}

/// Object capable of rendering events to HTML.
pub struct HtmlRenderer<'data, 'options, F> {
    out: F,
    footnotes: HashMap<Str<'data>, usize>,
    options: &'options HtmlRendererOptions,
}

impl<'data, 'options, F: Write> HtmlRenderer<'data, 'options, F> {
    /// Creates a new renderer that writes into a writer.
    pub fn new(out: F, options: &'options HtmlRendererOptions) -> HtmlRenderer<'data, 'options, F> {
        HtmlRenderer {
            out,
            footnotes: HashMap::new(),
            options,
        }
    }

    /// Consumes the writer and returns the inner file.
    pub fn into_writer(self) -> F {
        self.out
    }

    fn is_block_tag(&self, tag: Tag) -> bool {
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
            Tag::EmphasisAlt => false,
            Tag::Strong => false,
            Tag::Strikethrough => false,
            Tag::Link => false,
            Tag::TableHeader => true,
            Tag::TableBody => true,
            Tag::Container => true,
            Tag::Span => false,
        }
    }

    fn tag_to_html_tag(&self, tag: Tag) -> &'static str {
        match tag {
            Tag::Paragraph => "p",
            Tag::Heading1
            | Tag::Heading2
            | Tag::Heading3
            | Tag::Heading4
            | Tag::Heading5
            | Tag::Heading6 => {
                let level = tag.header_level().unwrap();
                let target_level = (self.options.initial_headline_level.saturating_sub(1)) + level;
                match target_level {
                    1 => "h1",
                    2 => "h2",
                    3 => "h3",
                    4 => "h4",
                    5 => "h5",
                    _ => "h6",
                }
            }
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
            Tag::EmphasisAlt => {
                if self.options.render_underlines {
                    "u"
                } else {
                    "em"
                }
            }
            Tag::Strong => "strong",
            Tag::Strikethrough => "ss",
            Tag::Link => "a",
            Tag::Container => "div",
            Tag::Span => "span",
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

        let mut combined_class = Cow::Borrowed("");
        if let Some(ref custom) = attrs.custom {
            for (key, value) in custom.iter() {
                if key == "style" {
                    if !combined_style.is_empty() {
                        combined_style.push_str("; ");
                    }
                    combined_style.push_str(value.as_str());
                } else if key == "class" {
                    combined_class = Cow::Borrowed(value.as_str());
                } else {
                    write!(self.out, " {}=\"{}\"", key, escape(value.as_str()))?;
                }
            }
        }

        if tag == Tag::FootnoteDefinition {
            combined_class = if combined_class.is_empty() {
                Cow::Borrowed(&self.options.footnote_definition_class)
            } else {
                Cow::Owned(format!(
                    "{} {}",
                    combined_class, &self.options.footnote_definition_class
                ))
            };
        }

        if !combined_class.is_empty() {
            write!(self.out, " class=\"{}\"", escape(&combined_class))?;
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
            if self.is_block_tag(tag) { "\n" } else { "" }
        )?;

        Ok(())
    }

    /// Feeds a single event into the renderer.
    pub fn feed_event(&mut self, event: &AnnotatedEvent<'data>) -> Result<(), io::Error> {
        match event.event {
            Event::DocumentStart(_) | Event::MetaData(_) => {}
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
                writeln!(self.out, ">{}</code></pre>", escape(code.as_str()))?;
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
            Event::SoftBreak => writeln!(self.out)?,
            Event::HardBreak => writeln!(self.out, "<br>")?,
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
                    "<sup class=\"{}\"><a href=\"#{}\">{}</a></sup>",
                    escape(&self.options.footnote_reference_class),
                    escape(target.as_str()),
                    number,
                )?;
            }
            Event::Error(ErrorEvent {
                ref title,
                ref description,
            }) => {
                write!(
                    self.out,
                    "<div class=\"error\">\n<h3>{}</h3>\n<p>{}</p>\n</div>",
                    escape(title.as_str()),
                    escape(description.as_ref().map_or("No details", |x| x.as_str())),
                )?;
            }
        }
        Ok(())
    }

    /// Feeds an event stream into the renderer.
    pub fn feed_stream<I>(&mut self, iter: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = AnnotatedEvent<'data>>,
    {
        for event in iter {
            self.feed_event(&event)?;
        }
        Ok(())
    }
}

impl<'data, 'options> HtmlRenderer<'data, 'options, Vec<u8>> {
    /// Creates a new html renderer writing into a buffer.
    pub fn new_buffered(options: &'options HtmlRendererOptions) -> Self {
        HtmlRenderer::new(Vec::new(), options)
    }

    /// Converts the renderer into a string.
    pub fn into_string(self) -> String {
        unsafe { String::from_utf8_unchecked(self.into_writer()) }
    }
}

/// Convenience shortcut that renders an event stream into HTML.
pub fn to_html<'a, I: Iterator<Item = AnnotatedEvent<'a>>>(
    iter: I,
    options: &HtmlRendererOptions,
) -> String {
    let mut renderer = HtmlRenderer::new_buffered(options);
    renderer.feed_stream(iter).unwrap();
    renderer.into_string()
}
