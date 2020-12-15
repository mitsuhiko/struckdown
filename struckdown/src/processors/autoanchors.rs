use serde::{Deserialize, Serialize};
use slug::slugify;
use std::borrow::Cow;
use std::collections::VecDeque;

use crate::event::{AnnotatedEvent, Event, StartTagEvent};
use crate::processors::Processor;

/// Automatically add anchors to all headers when missing.
///
/// When applied this wraps the stream in a [`AutoAnchorsIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AutoAnchors {
    /// The maximum level of headline that should get IDs.
    pub max_level: usize,
}

impl Default for AutoAnchors {
    fn default() -> AutoAnchors {
        AutoAnchors { max_level: 6 }
    }
}

impl Processor for AutoAnchors {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(AutoAnchorsIter::new(iter, Cow::Owned(*self)))
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(AutoAnchorsIter::new(iter, Cow::Borrowed(self)))
    }
}

/// The iterator implementing [`AutoAnchors`].
pub struct AutoAnchorsIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source: I,
    buffer: VecDeque<AnnotatedEvent<'data>>,
    options: Cow<'options, AutoAnchors>,
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>>
    AutoAnchorsIter<'data, 'options, I>
{
    pub fn new(iterator: I, options: Cow<'options, AutoAnchors>) -> Self {
        Self {
            source: iterator,
            buffer: VecDeque::new(),
            options,
        }
    }
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for AutoAnchorsIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(annotated_event) = self.buffer.pop_front() {
            return Some(annotated_event);
        }

        self.source.next().map(|mut annotated_event| {
            let (header_level, attrs) = match annotated_event.event {
                Event::StartTag(StartTagEvent { tag, ref mut attrs }) => {
                    if let Some(header_level) = tag.header_level() {
                        (header_level, attrs)
                    } else {
                        return annotated_event;
                    }
                }
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
