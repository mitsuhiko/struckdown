//! Stream processors for struckdown.
//!
//! This module provides stream processors that manipulate a struckdown event
//! stream to enhance it.  For instance a stream processor can automatically
//! add anchors to headers if they did not already get a header set by other
//! means.
#[macro_use]
mod utils;

mod toc;

#[cfg(feature = "autoanchors-processor")]
mod autoanchors;

#[cfg(feature = "external-processor")]
mod external;

#[cfg(feature = "syntect-processor")]
mod syntect;

#[cfg(feature = "html-sanitizer-processor")]
mod html_sanitizer;

use serde::Deserialize;

use crate::event::AnnotatedEvent;

pub use self::toc::{TableOfContents, TableOfContentsIter};

#[cfg(feature = "autoanchors-processor")]
pub use self::autoanchors::{AutoAnchors, AutoAnchorsIter};

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

macro_rules! builtin_processors {
    (
        $($(#[$attr:meta])* type $name:ident;)*
    ) => {
        /// Utility struct for processor configurations.
        #[derive(Debug, Deserialize, Clone)]
        #[serde(tag = "processor", rename_all = "snake_case")]
        pub enum BuiltinProcessor {
            $($(#[$attr])* $name(Box<$name>),)*
        }

        impl Processor for BuiltinProcessor {
            fn apply<'data>(
                self: Box<Self>,
                iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
            ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
                match *self {
                    $($(#[$attr])* Self::$name(options) => options.apply(iter),)*
                }
            }

            fn apply_ref<'data, 'options: 'data>(
                &'options self,
                iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
            ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
                match self {
                    $($(#[$attr])* Self::$name(options) => options.apply_ref(iter),)*
                }
            }
        }
    };
}

builtin_processors! {
    type TableOfContents;
    #[cfg(feature = "autoanchors-processor")]
    type AutoAnchors;
    #[cfg(feature = "external-processor")]
    type External;
    #[cfg(feature = "syntect-processor")]
    type Syntect;
    #[cfg(feature = "html-sanitizer-processor")]
    type HtmlSanitizer;
}
