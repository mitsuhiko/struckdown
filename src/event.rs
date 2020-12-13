use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::{self, Debug, Display};

use pulldown_cmark as cm;
use serde::{Deserialize, Serialize};

/// Represents front-matter in directives.
pub type FrontMatter = serde_yaml::Value;

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
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'data, 'de> Deserialize<'de> for Str<'data> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotatedEvent<'data>(Event<'data>, Option<Location>);

impl<'data> AnnotatedEvent<'data> {
    /// Creates an annotated event with location.
    pub fn new_with_location(event: Event<'data>, loc: Location) -> AnnotatedEvent<'data> {
        AnnotatedEvent(event, Some(loc))
    }

    /// Returns the embedded event.
    pub fn event(&self) -> &Event<'data> {
        &self.0
    }

    /// Mutable access to the event.
    pub fn event_mut(&mut self) -> &mut Event<'data> {
        &mut self.0
    }

    /// Returns the annotated location.
    pub fn location(&self) -> Option<&Location> {
        self.1.as_ref()
    }

    /// Sets a new location replacing the old.
    pub fn set_location(&mut self, location: Option<Location>) {
        self.1 = location;
    }

    /// Extracts the embedded event.
    pub fn into_event(self) -> Event<'data> {
        self.0
    }
}

impl<'data> From<Event<'data>> for AnnotatedEvent<'data> {
    fn from(event: Event<'data>) -> AnnotatedEvent<'data> {
        AnnotatedEvent(event, None)
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
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Attributes for tags.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Attrs<'data> {
    /// Holds the start for a list.  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start: Option<u32>,
    /// Alignment information
    #[serde(skip_serializing_if = "is_default")]
    pub(crate) alignment: Alignment,
    /// An optional id for elements supporting it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Str<'data>>,
    /// An optional title for elements supporting it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<Str<'data>>,
    /// Link reference target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<Str<'data>>,
    /// Custom attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) custom: Option<BTreeMap<Cow<'static, str>, Str<'data>>>,
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

    /// Returns the "start" information.
    pub fn start(&self) -> Option<u32> {
        self.start
    }

    /// Returns the alignment
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }

    /// Returns the id
    pub fn id(&self) -> Option<&str> {
        self.id.as_ref().map(|x| x.as_str())
    }

    /// Returns the title
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|x| x.as_str())
    }

    /// Returns the target
    pub fn target(&self) -> Option<&str> {
        self.target.as_ref().map(|x| x.as_str())
    }

    /// Returns a custom attribute.
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        match self.custom {
            Some(ref map) => map.get(key).map(|x| x.as_str()),
            None => None,
        }
    }

    /// Iter over custom attributes.
    pub fn iter_custom(&self) -> impl Iterator<Item = (&str, &Str<'data>)> {
        self.custom
            .iter()
            .flat_map(|attrs| attrs.iter().map(|(k, v)| (&**k, v)))
    }
}

/// Emitted when a tag starts.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartTagEvent<'data> {
    /// The tag that was started by this event.
    pub tag: Tag,
    /// Attached attributes to this event.
    #[serde(skip_serializing_if = "Attrs::is_empty")]
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
    pub front_matter: Option<FrontMatter>,
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
}
