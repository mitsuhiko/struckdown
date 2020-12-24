//! Stream processors for struckdown.
//!
//! This module provides stream processors that manipulate a struckdown event
//! stream to enhance it.  For instance a stream processor can automatically
//! add anchors to headers if they did not already get a header set by other
//! means.
mod autoanchors;
mod toc;

#[cfg(feature = "external-processor")]
mod external;

#[cfg(feature = "syntect-processor")]
mod syntect;

#[cfg(feature = "html-sanitizer-processor")]
mod html_sanitizer;

use serde::Deserialize;

use crate::event::AnnotatedEvent;

pub use self::autoanchors::{AutoAnchors, AutoAnchorsIter};
pub use self::toc::{TableOfContents, TableOfContentsIter};

#[cfg(feature = "external-processor")]
pub use self::external::{External, ExternalIter};

#[cfg(feature = "syntect-processor")]
pub use self::syntect::{Syntect, SyntectIter};

#[cfg(feature = "html-sanitizer-processor")]
pub use self::html_sanitizer::{HtmlSanitizer, HtmlSanitizerIter};

/// Common trait for all stream processors.
pub trait Processor {
    /// Applies the processor to an event stream.
    ///
    /// This consumes the processor.
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>;

    /// Applies the processor to an event stream.
    ///
    /// This attaches the processor by reference.
    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>;
}

/// Utility struct for processor configurations.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "processor", rename_all = "snake_case")]
pub enum BuiltinProcessor {
    AutoAnchors(Box<AutoAnchors>),
    TableOfContents(Box<TableOfContents>),
    #[cfg(feature = "external-processor")]
    External(Box<External>),
    #[cfg(feature = "syntect-processor")]
    Syntect(Box<Syntect>),
    #[cfg(feature = "html-sanitizer-processor")]
    HtmlSanitizer(Box<HtmlSanitizer>),
}

impl Processor for BuiltinProcessor {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        match *self {
            BuiltinProcessor::AutoAnchors(options) => options.apply(iter),
            BuiltinProcessor::TableOfContents(options) => options.apply(iter),
            #[cfg(feature = "external-processor")]
            BuiltinProcessor::External(options) => options.apply(iter),
            #[cfg(feature = "syntect-processor")]
            BuiltinProcessor::Syntect(options) => options.apply(iter),
            #[cfg(feature = "html-sanitizer-processor")]
            BuiltinProcessor::HtmlSanitizer(options) => options.apply(iter),
        }
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        match self {
            BuiltinProcessor::AutoAnchors(options) => options.apply_ref(iter),
            BuiltinProcessor::TableOfContents(options) => options.apply_ref(iter),
            #[cfg(feature = "external-processor")]
            BuiltinProcessor::External(options) => options.apply_ref(iter),
            #[cfg(feature = "syntect-processor")]
            BuiltinProcessor::Syntect(options) => options.apply_ref(iter),
            #[cfg(feature = "html-sanitizer-processor")]
            BuiltinProcessor::HtmlSanitizer(options) => options.apply_ref(iter),
        }
    }
}
