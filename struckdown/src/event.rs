//! Implements all types emitted from an event.
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::{self, Debug, Display};

use pulldown_cmark as cm;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::value::Value;

/// An internal string type.
///
/// This is not so much a string type as a container holding different
/// types of strings.  It does not even attempt to deref into a string
/// instead just provides a method [`Str::as_str`] to return the inner
/// value as string slice.
///
/// This string can be constructed from various other string types
/// via the [`From::from`] method.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Str<'data> {
    inner: cm::CowStr<'data>,
}

impl<'data> Serialize for Str<'data> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'data, 'de> Deserialize<'de> for Str<'data> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cow = Cow::<'data, str>::deserialize(deserializer)?;
        Ok(cow.into())
    }
}

impl From<String> for Str<'static> {
    fn from(value: String) -> Str<'static> {
        Str {
            inner: value.into(),
        }
    }
}

impl From<Box<str>> for Str<'static> {
    fn from(value: Box<str>) -> Str<'static> {
        Str {
            inner: cm::CowStr::Boxed(value),
        }
    }
}

impl<'data> From<&'data str> for Str<'data> {
    fn from(value: &'data str) -> Str<'data> {
        Cow::Borrowed(value).into()
    }
}

impl<'data> From<Cow<'data, str>> for Str<'data> {
    fn from(value: Cow<'data, str>) -> Str<'data> {
        match value {
            Cow::Borrowed(val) => Str {
                inner: cm::CowStr::Borrowed(val),
            },
            Cow::Owned(val) => Str {
                inner: cm::CowStr::Boxed(val.into()),
            },
        }
    }
}

impl<'data> Str<'data> {
    /// Creates a new string from a static literal.
    pub const fn new(val: &'data str) -> Str<'data> {
        Str {
            inner: cm::CowStr::Borrowed(val),
        }
    }

    /// Returns the contained string as string slice.
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Creates a string from a cmark string.
    pub(crate) fn from_cm_str(value: cm::CowStr<'data>) -> Str<'data> {
        Str { inner: value }
    }

    /// Slices the string down.
    pub(crate) fn slice(&self, start: usize, end: usize) -> Str<'data> {
        Str {
            inner: match self.inner {
                cm::CowStr::Borrowed(val) => cm::CowStr::Borrowed(&val[start..end]),
                cm::CowStr::Boxed(ref val) => {
                    cm::CowStr::Boxed(val[start..end].to_string().into_boxed_str())
                }
                cm::CowStr::Inlined(ref val) => {
                    cm::CowStr::Inlined((&val)[start..end].try_into().unwrap())
                }
            },
        }
    }

    /// Converts the string into a static version.
    fn into_static(self) -> Str<'static> {
        Str {
            inner: match self.inner {
                cm::CowStr::Borrowed(val) => cm::CowStr::Boxed(val.to_string().into_boxed_str()),
                cm::CowStr::Boxed(ref val) => cm::CowStr::Boxed(val.to_string().into_boxed_str()),
                cm::CowStr::Inlined(val) => cm::CowStr::Inlined(val),
            },
        }
    }
}

impl<'data> Display for Str<'data> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<'data> Debug for Str<'data> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<'data> PartialOrd for Str<'data> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl<'data> Ord for Str<'data> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Location information for an annotated event.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Location {
    /// Byte offset within the source document.
    pub offset: usize,
    /// Length in bytes in the source document.
    pub len: usize,
    /// Line in the source document (1 indexed).
    pub line: usize,
    /// Column in the source document (0 indexed).
    pub column: usize,
}

/// Event with annotations.
///
/// An annotated event is generally the same as an [`Event`] but it contains
/// optional annotations.  Annotations are generally just the location information
/// about where the event ocurred in the original source document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedEvent<'data> {
    /// The actual event.
    #[serde(flatten)]
    pub event: Event<'data>,
    /// The optional location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl<'data> AnnotatedEvent<'data> {
    /// Shortcut to create annotated events.
    pub fn new<I: Into<Event<'data>>>(
        value: I,
        location: Option<Location>,
    ) -> AnnotatedEvent<'data> {
        AnnotatedEvent {
            event: value.into(),
            location,
        }
    }

    /// Converts the event into a static one.
    pub fn into_static(self) -> AnnotatedEvent<'static> {
        AnnotatedEvent {
            event: self.event.into_static(),
            location: self.location,
        }
    }
}

impl<'data, T: Into<Event<'data>>> From<T> for AnnotatedEvent<'data> {
    fn from(value: T) -> AnnotatedEvent<'data> {
        AnnotatedEvent::new(value, None)
    }
}

/// Alignment information.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    /// Undefined alignment
    None,
    /// Left aligned
    Left,
    /// Centered
    Center,
    /// Right aligned
    Right,
}

impl Default for Alignment {
    fn default() -> Alignment {
        Alignment::None
    }
}

/// Tag type
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// `<p>` tag equivalent.
    Paragraph,
    /// `<h1>` tag equivalent.
    Heading1,
    /// `<h2>` tag equivalent.
    Heading2,
    /// `<h3>` tag equivalent.
    Heading3,
    /// `<h4>` tag equivalent.
    Heading4,
    /// `<h5>` tag equivalent.
    Heading5,
    /// `<h6>` tag equivalent.
    Heading6,
    /// `<blockquote>` tag equivalent.
    BlockQuote,
    /// `<ol>` equivalent.
    OrderedList,
    /// `<ul>` equivalent.
    UnorderedList,
    /// `<li>` equivalent.
    ListItem,
    /// Defines a footnote.
    FootnoteDefinition,
    /// `<table>` equivalent.
    Table,
    /// `<thead>` equivalent.
    TableHeader,
    /// `<tbody>` equivalent.
    TableBody,
    /// `<tr>` equivalent.
    TableRow,
    /// `<th>` equivalent.
    TableHead,
    /// <td>` equivalent.
    TableCell,
    /// `<em>` equivalent.
    Emphasis,
    /// Alternative emphasis (might be underline).
    EmphasisAlt,
    /// `<strong>` equivalent.
    Strong,
    /// `<ss>` equivalent.
    Strikethrough,
    /// `<a>` equivalent.
    Link,
    /// `<div>` equivalent. Not used in syntax.
    Container,
    /// `<span>` equivalent. Not used in syntax.
    Span,
}

impl Tag {
    /// Returns the header level for the tag if the tag represents a header.
    ///
    /// This is useful because headers are represented by multiple
    /// distinct tags and this avoids having to match on multiple
    /// enum values.
    pub fn header_level(self) -> Option<usize> {
        match self {
            Tag::Heading1 => Some(1),
            Tag::Heading2 => Some(2),
            Tag::Heading3 => Some(3),
            Tag::Heading4 => Some(4),
            Tag::Heading5 => Some(5),
            Tag::Heading6 => Some(6),
            _ => None,
        }
    }

    /// Creates a start tag event.
    pub fn start_tag(self, attrs: Attrs<'_>) -> StartTagEvent<'_> {
        StartTagEvent { tag: self, attrs }
    }

    /// Creates an end tag event.
    pub fn end_tag(self) -> EndTagEvent {
        EndTagEvent { tag: self }
    }
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Attributes for tags.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Attrs<'data> {
    /// Holds the start for a list.  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    /// Alignment information
    #[serde(default, skip_serializing_if = "is_default")]
    pub alignment: Alignment,
    /// An optional id for elements supporting it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Str<'data>>,
    /// Whitespace separated list of classes to attach to the element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<Str<'data>>,
    /// An optional title for elements supporting it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Str<'data>>,
    /// Link reference target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Str<'data>>,
    /// Custom attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<BTreeMap<Cow<'static, str>, Str<'data>>>,
}

impl<'data> Attrs<'data> {
    /// Returns `true` if all attrs are empty.
    pub fn is_empty(&self) -> bool {
        self.start.is_none()
            && self.alignment == Alignment::None
            && self.id.is_none()
            && self.class.is_none()
            && self.title.is_none()
            && self.target.is_none()
            && self.custom.is_none()
    }

    /// Converts the attrs into a static one.
    fn into_static(self) -> Attrs<'static> {
        Attrs {
            start: self.start,
            alignment: self.alignment,
            id: self.id.map(|x| x.into_static()),
            class: self.class.map(|x| x.into_static()),
            title: self.title.map(|x| x.into_static()),
            target: self.target.map(|x| x.into_static()),
            custom: self.custom.map(|custom| {
                custom
                    .into_iter()
                    .map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_static()))
                    .collect()
            }),
        }
    }
}

/// Emitted at the start of a document.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentStartEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_matter: Option<Value>,
}

/// Emitted when a tag starts.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartTagEvent<'data> {
    /// The tag that was started by this event.
    pub tag: Tag,
    /// Attached attributes to this event.
    #[serde(default, skip_serializing_if = "Attrs::is_empty")]
    pub attrs: Attrs<'data>,
}

/// Emitted when a tag ends.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndTagEvent {
    /// The tag that ends.
    pub tag: Tag,
}
/// A text event
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextEvent<'data> {
    pub text: Str<'data>,
}

/// Not yet filled in interpreted text.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterpretedTextEvent<'data> {
    /// The name of the role.
    pub role: Str<'data>,
    /// Text to the interpreted with that role.
    pub text: Str<'data>,
}

/// Code block
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeBlockEvent<'data> {
    /// The language argument to the code block.
    pub language: Option<Str<'data>>,
    /// Arguments to the code block.
    pub args: Option<BTreeMap<Str<'data>, Str<'data>>>,
    /// The raw code to be emitted.
    pub code: Str<'data>,
}

/// Directive block
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectiveEvent<'data> {
    /// The role of the directive.
    pub name: Str<'data>,
    /// The optional argument of the directive.
    pub argument: Option<Str<'data>>,
    /// The front matter if available.
    pub front_matter: Option<Value>,
    /// The directive body.
    pub body: Str<'data>,
}

/// Inline code
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InlineCodeEvent<'data> {
    /// The raw code to be emitted.
    pub code: Str<'data>,
}

/// An embedded image
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageEvent<'data> {
    /// The target location of the image
    pub target: Str<'data>,
    /// The optional alt text of the image
    pub alt: Option<Str<'data>>,
    /// The optional title of the image
    pub title: Option<Str<'data>>,
}

/// Embedded raw HTML
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawHtmlEvent<'data> {
    pub html: Str<'data>,
}

/// A checkbox from a task list.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckboxEvent {
    pub checked: bool,
}

/// A reference to a footnote.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FootnoteReferenceEvent<'data> {
    pub target: Str<'data>,
}

/// An inline meta data event.
///
/// Meta data can be emitted during processing and can be collected by a
/// processor or final renderer to do something with it.  It's not defined
/// what the keys are or what happens if multiple values are emitted for
/// the same key.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetaDataEvent<'data> {
    pub key: Str<'data>,
    pub value: Value,
}

/// An event representing an error during processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorEvent<'data> {
    pub title: Str<'data>,
    pub description: Option<Str<'data>>,
}

/// A event in a struckdown stream.
///
/// Struckdown events are not complete reflections of a markdown document.  In
/// fact it's not a goal to be able to go back from a struckdown event stream to
/// the original markdown document.  Certain normalizations are performed as part
/// of the parsing process to make it easier to perform stream processing.  As an
/// example images are no longer represented as tags but events.  Another example
/// are code tags which are normalized into an abstract code tag no matter the
/// original syntax.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Event<'data> {
    DocumentStart(DocumentStartEvent),
    StartTag(StartTagEvent<'data>),
    EndTag(EndTagEvent),
    Text(TextEvent<'data>),
    InterpretedText(InterpretedTextEvent<'data>),
    CodeBlock(CodeBlockEvent<'data>),
    Directive(DirectiveEvent<'data>),
    InlineCode(InlineCodeEvent<'data>),
    Image(ImageEvent<'data>),
    RawHtml(RawHtmlEvent<'data>),
    SoftBreak,
    HardBreak,
    Rule,
    Checkbox(CheckboxEvent),
    FootnoteReference(FootnoteReferenceEvent<'data>),
    MetaData(MetaDataEvent<'data>),
    Error(ErrorEvent<'data>),
}

macro_rules! impl_from_event_type {
    ($variant:ident, $name:ty, $lifetime:tt) => {
        impl<$lifetime> From<$name> for Event<$lifetime> {
            fn from(value: $name) -> Self {
                Event::$variant(value)
            }
        }
    };
    ($variant:ident, $name:ty) => {
        impl From<$name> for Event<'static> {
            fn from(value: $name) -> Self {
                Event::$variant(value)
            }
        }
    };
}

impl_from_event_type!(DocumentStart, DocumentStartEvent);
impl_from_event_type!(StartTag, StartTagEvent<'data>, 'data);
impl_from_event_type!(EndTag, EndTagEvent);
impl_from_event_type!(Text, TextEvent<'data>, 'data);
impl_from_event_type!(InterpretedText, InterpretedTextEvent<'data>, 'data);
impl_from_event_type!(CodeBlock, CodeBlockEvent<'data>, 'data);
impl_from_event_type!(Directive, DirectiveEvent<'data>, 'data);
impl_from_event_type!(InlineCode, InlineCodeEvent<'data>, 'data);
impl_from_event_type!(Image, ImageEvent<'data>, 'data);
impl_from_event_type!(RawHtml, RawHtmlEvent<'data>, 'data);
impl_from_event_type!(Checkbox, CheckboxEvent);
impl_from_event_type!(FootnoteReference, FootnoteReferenceEvent<'data>, 'data);
impl_from_event_type!(MetaData, MetaDataEvent<'data>, 'data);
impl_from_event_type!(Error, ErrorEvent<'data>, 'data);

impl<'data> Event<'data> {
    /// Returns the contents as raw text.
    pub fn raw_text(&self) -> Option<&Str<'data>> {
        static NEWLINE: Str<'static> = Str::new("\n");
        static DOUBLE_NEWLINE: Str<'static> = Str::new("\n\n");
        match *self {
            Event::Text(TextEvent { ref text })
            | Event::InterpretedText(InterpretedTextEvent { ref text, .. })
            | Event::InlineCode(InlineCodeEvent { code: ref text, .. })
            | Event::CodeBlock(CodeBlockEvent { code: ref text, .. }) => Some(text),
            Event::Directive(DirectiveEvent { body: ref text, .. }) => Some(text),
            Event::SoftBreak => Some(&NEWLINE),
            Event::HardBreak => Some(&DOUBLE_NEWLINE),
            Event::Image(ImageEvent { ref alt, .. }) => alt.as_ref(),
            _ => None,
        }
    }

    /// Converts the event into a static one.
    pub fn into_static(self) -> Event<'static> {
        match self {
            Event::DocumentStart(value) => Event::DocumentStart(value),
            Event::StartTag(value) => Event::StartTag(StartTagEvent {
                tag: value.tag,
                attrs: value.attrs.into_static(),
            }),
            Event::EndTag(value) => Event::EndTag(value),
            Event::Text(value) => Event::Text(TextEvent {
                text: value.text.into_static(),
            }),
            Event::InterpretedText(value) => Event::InterpretedText(InterpretedTextEvent {
                role: value.role.into_static(),
                text: value.text.into_static(),
            }),
            Event::CodeBlock(value) => Event::CodeBlock(CodeBlockEvent {
                language: value.language.map(|x| x.into_static()),
                args: value.args.map(|x| {
                    x.into_iter()
                        .map(|(k, v)| (k.into_static(), v.into_static()))
                        .collect()
                }),
                code: value.code.into_static(),
            }),
            Event::Directive(value) => Event::Directive(DirectiveEvent {
                name: value.name.into_static(),
                argument: value.argument.map(|x| x.into_static()),
                front_matter: value.front_matter,
                body: value.body.into_static(),
            }),
            Event::InlineCode(value) => Event::InlineCode(InlineCodeEvent {
                code: value.code.into_static(),
            }),
            Event::Image(value) => Event::Image(ImageEvent {
                target: value.target.into_static(),
                alt: value.alt.map(|x| x.into_static()),
                title: value.title.map(|x| x.into_static()),
            }),
            Event::RawHtml(value) => Event::RawHtml(RawHtmlEvent {
                html: value.html.into_static(),
            }),
            Event::SoftBreak => Event::SoftBreak,
            Event::HardBreak => Event::HardBreak,
            Event::Rule => Event::Rule,
            Event::Checkbox(value) => Event::Checkbox(value),
            Event::FootnoteReference(value) => Event::FootnoteReference(FootnoteReferenceEvent {
                target: value.target.into_static(),
            }),
            Event::MetaData(value) => Event::MetaData(MetaDataEvent {
                key: value.key.into_static(),
                value: value.value.clone(),
            }),
            Event::Error(value) => Event::Error(ErrorEvent {
                title: value.title.into_static(),
                description: value.description.map(|x| x.into_static()),
            }),
        }
    }
}
