use serde::{Deserialize, Serialize};
use slug::slugify;
use std::collections::VecDeque;

use crate::event::{AnnotatedEvent, Event, StartTagEvent, Tag};

/// Configuration options for the [`AutoAnchors`] processor.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoAnchorsOptions {
    /// The maximum level of headline that should get IDs.
    pub max_level: usize,
}

impl Default for AutoAnchorsOptions {
    fn default() -> AutoAnchorsOptions {
        AutoAnchorsOptions { max_level: 6 }
    }
}

/// Adds anchors to all headers if missing.
pub struct AutoAnchors<'data, I> {
    source: I,
    buffer: VecDeque<AnnotatedEvent<'data>>,
    options: AutoAnchorsOptions,
}

impl<'data, I: Iterator<Item = AnnotatedEvent<'data>>> AutoAnchors<'data, I> {
    /// Creates a new auto anchor stream processor with default options.
    pub fn new(iterator: I) -> Self {
        Self::new_with_options(iterator, AutoAnchorsOptions::default())
    }

    /// Creates a new auto anchor stream processor with specific options.
    pub fn new_with_options(iterator: I, options: AutoAnchorsOptions) -> Self {
        Self {
            source: iterator,
            buffer: VecDeque::new(),
            options,
        }
    }

    /// Read only reference to the options.
    pub fn options(&self) -> &AutoAnchorsOptions {
        &self.options
    }

    /// Writable reference to the options.
    pub fn options_mut(&mut self) -> &mut AutoAnchorsOptions {
        &mut self.options
    }
}

impl<'data, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator for AutoAnchors<'data, I> {
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(annotated_event) = self.buffer.pop_front() {
            return Some(annotated_event);
        }

        self.source.next().map(|mut annotated_event| {
            let (header_level, attrs) = match annotated_event.event {
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading1,
                    ref mut attrs,
                }) => (1, attrs),
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading2,
                    ref mut attrs,
                }) => (2, attrs),
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading3,
                    ref mut attrs,
                }) => (3, attrs),
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading4,
                    ref mut attrs,
                }) => (4, attrs),
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading5,
                    ref mut attrs,
                }) => (5, attrs),
                Event::StartTag(StartTagEvent {
                    tag: Tag::Heading6,
                    ref mut attrs,
                }) => (6, attrs),
                _ => return annotated_event,
            };

            if attrs.id.is_some() || header_level > self.options.max_level {
                return annotated_event;
            }

            let mut depth = 1;
            let mut raw_text = String::new();

            while let Some(next_annotated_event) = self.source.next() {
                match next_annotated_event.event {
                    Event::StartTag(..) => depth += 1,
                    Event::EndTag(..) => depth -= 1,
                    ref event => {
                        if let Some(text) = event.raw_text() {
                            raw_text.push_str(text.as_str());
                        }
                    }
                }
                self.buffer.push_back(next_annotated_event);
                if depth == 0 {
                    break;
                }
            }

            attrs.id = Some(slugify(raw_text).into());

            annotated_event
        })
    }
}
