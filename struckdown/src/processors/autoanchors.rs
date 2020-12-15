use slug::slugify;
use std::collections::VecDeque;

use crate::event::{AnnotatedEvent, Event, StartTagEvent, Tag};

/// Adds anchors to all headers if missing.
pub struct AutoAnchors<'data, I> {
    source: I,
    buffer: VecDeque<AnnotatedEvent<'data>>,
    min_level: usize,
}

impl<'data, I: Iterator<Item = AnnotatedEvent<'data>>> AutoAnchors<'data, I> {
    /// Creates a new auto anchor stream processor.
    ///
    /// This sets the default minimum level to 6 which means that all headers
    /// will get an ID added.  This minimum level can be reconfigured by using
    /// the [`AutoAnchors::set_min_level`] method.
    pub fn new(iterator: I) -> Self {
        Self {
            source: iterator,
            buffer: VecDeque::new(),
            min_level: 6,
        }
    }

    /// Overrides the minimum level that should get IDs
    pub fn set_min_level(&mut self, min_level: usize) {
        self.min_level = min_level;
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

            if attrs.id.is_some() || header_level > self.min_level {
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
                if depth == 0 {
                    break;
                }
            }

            attrs.id = Some(slugify(raw_text).into());

            annotated_event
        })
    }
}
