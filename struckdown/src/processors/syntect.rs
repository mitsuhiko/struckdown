use std::borrow::Cow;
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;

use crate::event::{AnnotatedEvent, CodeBlockEvent, Event, RawHtmlEvent};
use crate::processors::Processor;

const DEFAULT_THEME: &str = "InspiredGitHub";

/// Implements syntax highlighting via [`syntect`].
///
/// When applied this wraps the stream in a [`SyntectIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Syntect {
    /// The name of the theme to use.  If both this and `theme_path` is not
    /// set then a default theme is loaded.  If `theme_path` is set then the
    /// theme in the `theme_path` folder is used.
    pub theme: Option<String>,
    /// When `theme` is not set, then the path to the `.tmTheme` file to load
    /// otherwise the folder to a collection of theme files.
    pub theme_path: Option<PathBuf>,
}

impl Default for Syntect {
    fn default() -> Syntect {
        Syntect {
            theme: None,
            theme_path: None,
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
        let theme = match (&options.theme, &options.theme_path) {
            (Some(theme), None) => {
                let mut theme_set = ThemeSet::load_defaults();
                match theme_set.themes.remove(theme) {
                    Some(theme) => theme,
                    None => theme_set.themes.remove(DEFAULT_THEME).unwrap(),
                }
            }
            (Some(theme), Some(path)) => {
                let mut theme_set =
                    ThemeSet::load_from_folder(path).expect("failed to initialized theme folder");
                match theme_set.themes.remove(theme) {
                    Some(theme) => theme,
                    None => theme_set.themes.remove(DEFAULT_THEME).unwrap(),
                }
            }
            (None, Some(ref path)) => {
                ThemeSet::get_theme(path).expect("failed to load theme by path")
            }
            (None, None) => {
                let mut theme_set = ThemeSet::load_defaults();
                theme_set.themes.remove(DEFAULT_THEME).unwrap()
            }
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
