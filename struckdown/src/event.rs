//! Implements all types emitted from an event.
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;

use pulldown_cmark as cm;
use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug)]
pub struct AnnotatedEvent<'data> {
    /// The actual event.
    pub event: Event<'data>,
    /// The optional location.
    pub location: Option<Location>,
}
impl<'data> From<Event<'data>> for AnnotatedEvent<'data> {
    fn from(event: Event<'data>) -> AnnotatedEvent<'data> {
        AnnotatedEvent {
            event,
            location: None,
        }
    }
}

impl<'data> Serialize for AnnotatedEvent<'data> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref location) = self.location {
            let mut s = serializer.serialize_tuple(2)?;
            s.serialize_element(&self.event)?;
            s.serialize_element(location)?;
            s.end()
        } else {
            self.event.serialize(serializer)
        }
    }
}

impl<'de, 'data> Deserialize<'de> for AnnotatedEvent<'data> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayOrStruct<'data>(PhantomData<AnnotatedEvent<'data>>);

        impl<'de, 'data> Visitor<'de> for ArrayOrStruct<'data> {
            type Value = AnnotatedEvent<'data>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("array or map")
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                Deserialize::deserialize(SeqAccessDeserializer::new(seq))
                    .map(|(event, location)| AnnotatedEvent { event, location })
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                Deserialize::deserialize(MapAccessDeserializer::new(map)).map(|event| {
                    AnnotatedEvent {
                        event,
                        location: None,
                    }
                })
            }
        }

        deserializer.deserialize_any(ArrayOrStruct(PhantomData))
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
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Attributes for tags.
#[derive(Debug, Default, Serialize, Deserialize)]
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
            && self.title.is_none()
            && self.target.is_none()
            && self.custom.is_none()
    }
}

/// Emitted at the start of a document.
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentStartEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_matter: Option<Value>,
}

/// Emitted when a tag starts.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartTagEvent<'data> {
    /// The tag that was started by this event.
    pub tag: Tag,
    /// Attached attributes to this event.
    #[serde(default, skip_serializing_if = "Attrs::is_empty")]
    pub attrs: Attrs<'data>,
}

/// Emitted when a tag ends.
#[derive(Debug, Serialize, Deserialize)]
pub struct EndTagEvent {
    /// The tag that ends.
    pub tag: Tag,
}
/// A text event
#[derive(Debug, Serialize, Deserialize)]
pub struct TextEvent<'data> {
    pub text: Str<'data>,
}

/// Not yet filled in interpreted text.
#[derive(Debug, Serialize, Deserialize)]
pub struct InterpretedTextEvent<'data> {
    /// The name of the role.
    pub role: Str<'data>,
    /// Text to the interpreted with that role.
    pub text: Str<'data>,
}

/// Code block
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeBlockEvent<'data> {
    /// The language argument to the code block.
    pub language: Option<Str<'data>>,
    /// The raw code to be emitted.
    pub code: Str<'data>,
}

/// Directive block
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct InlineCodeEvent<'data> {
    /// The raw code to be emitted.
    pub code: Str<'data>,
}

/// An embedded image
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageEvent<'data> {
    /// The target location of the image
    pub target: Str<'data>,
    /// The optional alt text of the image
    pub alt: Option<Str<'data>>,
    /// The optional title of the image
    pub title: Option<Str<'data>>,
}

/// Embedded raw HTML
#[derive(Debug, Serialize, Deserialize)]
pub struct RawHtmlEvent<'data> {
    pub html: Str<'data>,
}

/// A checkbox from a task list.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckboxEvent {
    pub checked: bool,
}

/// A reference to a footnote.
#[derive(Debug, Serialize, Deserialize)]
pub struct FootnoteReferenceEvent<'data> {
    pub target: Str<'data>,
}

/// An inline meta data event.
///
/// Meta data can be emitted during processing and can be collected by a
/// processor or final renderer to do something with it.  It's not defined
/// what the keys are or what happens if multiple values are emitted for
/// the same key.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetaDataEvent<'data> {
    pub key: Str<'data>,
    pub value: Value,
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
#[derive(Debug, Serialize, Deserialize)]
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
}

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
}
