use std::borrow::Cow;
use std::{iter, mem};

use itertools::Either;
use serde::{Deserialize, Serialize};

use crate::event::{
    AnnotatedEvent, Attrs, DirectiveEvent, Event, MetaDataEvent, StartTagEvent, Str, Tag,
};
use crate::plain::to_plain_text;
use crate::processors::Processor;
use crate::value::to_value;

/// Automatically add anchors to all headers when missing.
///
/// When applied this wraps the stream in a [`TableOfContentsIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TableOfContents {
    /// The name of the role that inserts the TOC.
    pub role_name: Option<String>,
    /// Controls if the toc should be emitted as meta data.
    pub emit_metadata: bool,
    /// The class that should be added to the TOC.
    pub class_name: Option<String>,
}

impl Default for TableOfContents {
    fn default() -> TableOfContents {
        TableOfContents {
            role_name: Some("toc".into()),
            emit_metadata: true,
            class_name: Some("table-of-contents".into()),
        }
    }
}

impl Processor for TableOfContents {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(TableOfContentsIter::new(iter, Cow::Owned(*self)))
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(TableOfContentsIter::new(iter, Cow::Borrowed(self)))
    }
}

/// The iterator implementing [`TableOfContents`].
pub struct TableOfContentsIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source_iter: Option<I>,
    iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    options: Cow<'options, TableOfContents>,
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>>
    TableOfContentsIter<'data, 'options, I>
{
    pub fn new(iterator: I, options: Cow<'options, TableOfContents>) -> Self {
        Self {
            source_iter: Some(iterator),
            iter: Box::new(None.into_iter()),
            options,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct TocItem<'data> {
    #[serde(skip_serializing)]
    level: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    anchor: Option<Str<'data>>,
    text: Option<Str<'data>>,
    events: Vec<AnnotatedEvent<'data>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<TocItem<'data>>,
}

fn with_toc_at_level<'data, F>(root: &mut TocItem<'data>, level: usize, f: F)
where
    F: FnOnce(&mut TocItem<'data>),
{
    let mut deepest = root;
    while deepest.level < level - 1 {
        if deepest.children.is_empty() {
            deepest.children.push(TocItem {
                level: deepest.level + 1,
                anchor: None,
                text: None,
                events: vec![],
                children: Vec::new(),
            });
        }
        deepest = deepest.children.last_mut().unwrap();
    }
    f(deepest)
}

fn dump_toc<'data>(out: &mut Vec<AnnotatedEvent<'data>>, toc: &TocItem<'data>) {
    out.push(Tag::ListItem.start_tag(Attrs::default()).into());
    if !toc.events.is_empty() {
        out.push(
            (match toc.anchor {
                Some(ref anchor) => Tag::Link.start_tag(Attrs {
                    target: Some(format!("#{}", anchor.as_str()).into()),
                    ..Attrs::default()
                }),
                None => Tag::Span.start_tag(Attrs::default()),
            })
            .into(),
        );
        out.extend(toc.events.iter().cloned());
        out.push(
            (if toc.anchor.is_some() {
                Tag::Link
            } else {
                Tag::Span
            })
            .end_tag()
            .into(),
        );
    }
    if !toc.children.is_empty() {
        out.push(Tag::UnorderedList.start_tag(Attrs::default()).into());
        for child in &toc.children {
            dump_toc(out, child);
        }
        out.push(Tag::UnorderedList.end_tag().into());
    }
    out.push(Tag::ListItem.end_tag().into());
}

fn extract_toc<'data, I: Iterator<Item = AnnotatedEvent<'data>>>(
    iter: I,
    with_metadata: bool,
    class_name: Option<&str>,
) -> (
    Vec<AnnotatedEvent<'data>>,
    Vec<AnnotatedEvent<'data>>,
    Option<AnnotatedEvent<'data>>,
) {
    let mut buf = Vec::with_capacity(iter.size_hint().0);
    let mut headline = None;
    let mut headline_buf = vec![];
    let mut level = 0;
    let mut toc_tree = TocItem {
        level: 0,
        anchor: None,
        text: None,
        events: Vec::new(),
        children: Vec::new(),
    };

    for annotated_event in iter {
        match annotated_event.event {
            Event::StartTag(StartTagEvent { tag, ref attrs }) => {
                if let Some(header_level) = tag.header_level() {
                    headline = Some((header_level, attrs.id.clone()));
                } else if headline.is_some() {
                    headline_buf.push(annotated_event.clone());
                }
                if headline.is_some() {
                    level += 1;
                }
            }
            Event::EndTag(..) => {
                if headline.is_some() {
                    level -= 1;
                    if level == 0 {
                        let (level, anchor) = headline.take().unwrap();
                        let events = mem::replace(&mut headline_buf, Vec::new());
                        with_toc_at_level(&mut toc_tree, level, move |toc_tree| {
                            toc_tree.children.push(TocItem {
                                level: toc_tree.level + 1,
                                anchor: anchor.clone(),
                                text: Some(to_plain_text(events.iter())),
                                events: events.clone(),
                                children: Vec::new(),
                            });
                        });
                    } else {
                        headline_buf.push(annotated_event.clone());
                    }
                }
            }
            Event::Error(..) | Event::MetaData(..) => {}
            _ => {
                if headline.is_some() {
                    headline_buf.push(annotated_event.clone());
                }
            }
        }
        buf.push(annotated_event);
    }

    let mut toc = Vec::new();
    toc.push(
        Tag::UnorderedList
            .start_tag(Attrs {
                class: class_name.map(|x| x.to_string().into()),
                ..Attrs::default()
            })
            .into(),
    );
    for child in &toc_tree.children {
        dump_toc(&mut toc, &child);
    }
    toc.push(Tag::UnorderedList.end_tag().into());

    let metadata = if with_metadata {
        Some(
            Event::MetaData(MetaDataEvent {
                key: "toc".into(),
                value: to_value(&toc_tree.children).expect("bad toc tree"),
            })
            .into(),
        )
    } else {
        None
    };

    (buf, toc, metadata)
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for TableOfContentsIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(source) = self.source_iter.take() {
            let (buf, toc, metadata) = extract_toc(
                source,
                self.options.emit_metadata,
                self.options.class_name.as_deref(),
            );
            let role_name = self.options.role_name.clone();
            self.iter = Box::new(
                buf.into_iter()
                    .flat_map(move |annotated_event| {
                        if let Event::Directive(DirectiveEvent { ref name, .. }) =
                            annotated_event.event
                        {
                            if Some(name.as_str()) == role_name.as_deref() {
                                return Either::Left(toc.clone().into_iter());
                            }
                        }
                        Either::Right(iter::once(annotated_event))
                    })
                    .chain(metadata.into_iter()),
            ) as Box<dyn Iterator<Item = _>>;
        }

        self.iter.next()
    }
}
