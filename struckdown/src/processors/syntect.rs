use std::borrow::Cow;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;

use crate::event::{AnnotatedEvent, CodeBlockEvent, Event, RawHtmlEvent};
use crate::processors::Processor;

/// Passes a JSON serialized stream through an external program.
///
/// When applied this wraps the stream in a [`SyntectIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Syntect {
    /// The theme to use
    theme: String,
}

impl Default for Syntect {
    fn default() -> Syntect {
        Syntect {
            theme: "InspiredGitHub".into(),
        }
    }
}

impl Processor for Syntect {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(SyntectIter::new(iter, Cow::Owned(*self)))
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(SyntectIter::new(iter, Cow::Borrowed(self)))
    }
}

/// The iterator implementing [`Syntect`].
pub struct SyntectIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source: I,
    syntax_set: SyntaxSet,
    theme: Theme,
    options: PhantomData<&'options Syntect>,
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> SyntectIter<'data, 'options, I> {
    pub fn new(iterator: I, options: Cow<'options, Syntect>) -> Self {
        let mut theme_set = ThemeSet::load_defaults();
        let theme = match theme_set.themes.remove(&options.theme) {
            Some(theme) => theme,
            None => theme_set.themes.remove("InspiredGitHub").unwrap(),
        };
        Self {
            source: iterator,
            syntax_set: SyntaxSet::load_defaults_nonewlines(),
            theme,
            options: PhantomData,
        }
    }
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for SyntectIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        let annotated_event = self.source.next()?;
        if let Event::CodeBlock(CodeBlockEvent {
            language: Some(ref language),
            ref code,
        }) = annotated_event.event
        {
            dbg!(language);
            let language = language.as_str();
            let syntax = self
                .syntax_set
                .find_syntax_by_token(language)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            let mut h = HighlightLines::new(syntax, &self.theme);
            let regions = h.highlight(code.as_str(), &self.syntax_set);
            return Some(AnnotatedEvent {
                event: Event::RawHtml(RawHtmlEvent {
                    html: format!(
                        "<pre><code>{}</code></pre>",
                        styled_line_to_highlighted_html(&regions[..], IncludeBackground::No)
                    )
                    .into(),
                }),
                location: annotated_event.location,
            });
        }
        Some(annotated_event)
    }
}
