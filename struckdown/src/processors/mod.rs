//! Stream processors for struckdown.
//!
//! This module provides stream processors that manipulate a struckdown event
//! stream to enhance it.  For instance a stream processor can automatically
//! add anchors to headers if they did not already get a header set by other
//! means.
mod autoanchors;
mod external;

use serde::Deserialize;

use crate::event::AnnotatedEvent;

pub use self::autoanchors::{AutoAnchors, AutoAnchorsIter};
pub use self::external::{External, ExternalIter};

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
    External(Box<External>),
}

impl Processor for BuiltinProcessor {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        match *self {
            BuiltinProcessor::AutoAnchors(options) => options.apply(iter),
            BuiltinProcessor::External(options) => options.apply(iter),
        }
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        match self {
            BuiltinProcessor::AutoAnchors(options) => options.apply_ref(iter),
            BuiltinProcessor::External(options) => options.apply_ref(iter),
        }
    }
}
