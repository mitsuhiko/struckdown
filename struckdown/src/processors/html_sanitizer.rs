use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

use ammonia::Builder;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::event::{AnnotatedEvent, Event};

lazy_static! {
    static ref MARKER_RE: Regex = Regex::new(r"\A\.\.\.([a-f0-9]{32})\.\.\.").unwrap();
}

/// Automatically add anchors to all headers when missing.
///
/// When applied this wraps the stream in a [`HtmlSanitizerIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct HtmlSanitizer {
    /// Configures link rel to be added to all links.
    pub link_rel: Option<String>,
    /// Permitted URL schems.
    pub url_schemes: HashSet<String>,
    /// If configured the `class` attribute is permitted without filtering.
    pub allow_class: bool,
    /// If configured the `style` attribute is permitted without filtering.
    pub allow_style: bool,
    /// If set to `false` then comments are removed.
    pub allow_comments: bool,
}

impl Default for HtmlSanitizer {
    fn default() -> HtmlSanitizer {
        HtmlSanitizer {
            link_rel: None,
            url_schemes: [
                "ftp", "ftps", "geo", "http", "https", "im", "irc", "ircs", "magnet", "mailto",
                "mms", "mx", "news", "nntp", "sip", "sms", "smsto", "ssh", "tel", "url", "webcal",
                "wtai", "xmpp",
            ]
            .iter()
            .copied()
            .map(Into::into)
            .collect(),
            allow_class: false,
            allow_style: false,
            allow_comments: true,
        }
    }
}

implement_processor!(HtmlSanitizer, HtmlSanitizerIter);

fn make_ammonia(options: &HtmlSanitizer) -> Builder {
    let mut ammonia = Builder::default();
    let mut clean_content_tags = HashSet::new();
    clean_content_tags.insert("script");
    ammonia.url_schemes(options.url_schemes.iter().map(|x| x.as_str()).collect());

    if options.allow_class {
        ammonia.add_generic_attributes(&["class"]);
    }
    if options.allow_style {
        ammonia.add_generic_attributes(&["style"]);
        ammonia.add_tags(&["style"]);
    } else {
        clean_content_tags.insert("style");
    }
    ammonia.clean_content_tags(clean_content_tags);
    ammonia.link_rel(options.link_rel.as_deref());
    ammonia.strip_comments(!options.allow_comments);
    ammonia
}

/// The iterator implementing [`HtmlSanitizer`].
pub struct HtmlSanitizerIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source_iter: Option<I>,
    processed_iter: std::vec::IntoIter<AnnotatedEvent<'data>>,
    options: Cow<'options, HtmlSanitizer>,
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>>
    HtmlSanitizerIter<'data, 'options, I>
{
    pub fn new<O: Into<Cow<'options, HtmlSanitizer>>>(iterator: I, options: O) -> Self {
        Self {
            source_iter: Some(iterator),
            processed_iter: Vec::new().into_iter(),
            options: options.into(),
        }
    }
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for HtmlSanitizerIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(source_iter) = self.source_iter.take() {
            let marker = format!("...{}...", Uuid::new_v4().to_simple());
            let mut buffer = source_iter.collect::<Vec<_>>();
            let mut html_buf = String::new();
            let ammonia = make_ammonia(&self.options);
            let mut segments = BTreeMap::new();
            let mut idx = 0;
            for annotated_event in &mut buffer {
                if let Event::RawHtml(ref mut raw_html) = annotated_event.event {
                    idx += 1;
                    let id = Uuid::new_v4();
                    html_buf.push_str(&format!("...{}...", id.to_simple()));
                    html_buf.push_str(raw_html.html.as_str());
                    html_buf.push_str(&marker);
                    segments.insert(id, idx);
                }
            }
            let cleaned = dbg!(ammonia.clean(&html_buf).to_string());

            let mut replacements = BTreeMap::new();
            for segment in cleaned.split(&marker) {
                if let Some(id) = MARKER_RE.captures(segment) {
                    if let Some(&idx) = segments.get(&Uuid::parse_str(&id[1]).unwrap()) {
                        replacements.insert(idx, &segment[id.get(0).unwrap().end()..]);
                    }
                }
            }

            let mut idx = 0;
            for annotated_event in &mut buffer {
                if let Event::RawHtml(ref mut raw_html) = annotated_event.event {
                    idx += 1;
                    if let Some(&new) = replacements.get(&idx) {
                        raw_html.html = new.to_string().into();
                        continue;
                    }
                    raw_html.html = "".into();
                }
            }

            self.processed_iter = buffer.into_iter();
        }
        self.processed_iter.next()
    }
}
