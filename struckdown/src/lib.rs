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
