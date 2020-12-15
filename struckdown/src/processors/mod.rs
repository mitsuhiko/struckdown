//! Stream processors for struckdown.
//!
//! This module provides stream processors that manipulate a struckdown event
//! stream to enhance it.  For instance a stream processor can automatically
//! add anchors to headers if they did not already get a header set by other
//! means.
mod autoanchors;

pub use self::autoanchors::{AutoAnchors, AutoAnchorsOptions};
