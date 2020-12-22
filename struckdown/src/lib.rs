//! struckdown is a text processing system based on markdown/commonmark with
//! structured extensions inspired by reStructuredText.  It parses commonmark
//! with structural extensions (roles and directives) into an event stream.
//!
//! This event stream can be processed before rendering.
pub mod event;
pub mod html;
pub mod parser;
pub mod pipeline;
pub mod processors;

// internal only for now
mod plain;

/// Gives access to [`serde_json`] value functionality.
///
/// The [`Value`](crate::value::Value) type is used to represent arbitrary data in a few instances.
/// A notable example is emitted [`front_matter`](crate::event::DocumentStartEvent::front_matter)
/// or other [`MetaData`](crate::event::MetaDataEvent).
pub mod value {
    pub use serde_json::value::*;

    /// A re-export of the [`serde_json::json`] macro.
    pub use serde_json::json as value;

    pub use serde_json::{from_value, to_value};
}
