use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::{self, Debug, Display};

use pulldown_cmark as cm;
use serde::{Deserialize, Serialize};

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

impl<'data> From<cm::CowStr<'data>> for Str<'data> {
    fn from(value: cm::CowStr<'data>) -> Str<'data> {
        Str { inner: value }
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
    pub fn slice(&self, start: usize, end: usize) -> Str<'data> {
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

impl<'data> Str<'data> {
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub offset: usize,
    pub len: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event<'data>(EventType<'data>, Location);

impl<'data> Event<'data> {
    pub fn new(ty: EventType<'data>, loc: Location) -> Event<'data> {
        Event(ty, loc)
    }

    pub fn ty(&self) -> &EventType<'data> {
        &self.0
    }

    pub fn ty_mut(&mut self) -> &mut EventType<'data> {
        &mut self.0
    }

    pub fn location(&self) -> &Location {
        &self.1
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    BlockQuote,
    IndentedCode,
    FencedCode,
    OrderedList,
    UnorderedList,
    ListItem,
    FootnoteDefinition,
    Table,
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    Directive,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Attrs<'data> {
    /// A role determines for the processing system how to interpret
    /// an element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) role: Option<Str<'data>>,
    /// The arguments for a code block or directive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) argument: Option<Str<'data>>,
    /// Holds the start for a list.  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start: Option<u32>,
    /// Holds the alignments for tables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) alignments: Option<Vec<Alignment>>,
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
        self.role.is_none()
            && self.argument.is_none()
            && self.start.is_none()
            && self.alignments.is_none()
            && self.id.is_none()
            && self.title.is_none()
            && self.target.is_none()
            && self.custom.is_none()
    }

    /// Returns the stored role.
    pub fn role(&self) -> Option<&str> {
        self.role.as_ref().map(|x| x.as_str())
    }

    /// Returns the argument to a code block or directive.
    pub fn argument(&self) -> Option<&str> {
        self.argument.as_ref().map(|x| x.as_str())
    }

    /// Returns the "start" information.
    pub fn start(&self) -> Option<u32> {
        self.start
    }

    /// Returns the alignments.
    pub fn alignments(&self) -> &[Alignment] {
        match self.alignments {
            None => &[],
            Some(ref alignments) => &alignments[..],
        }
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum EventType<'data> {
    /// Starts a tag with optional attributes
    StartTag {
        tag: Tag,
        #[serde(skip_serializing_if = "Attrs::is_empty")]
        attrs: Attrs<'data>,
    },
    /// Ends a specific tag.
    EndTag { tag: Tag },
    /// Raw, uninterpreted text
    Text { text: Str<'data> },
    /// Interpreted text (text with role)
    InterpretedText { text: Str<'data>, role: Str<'data> },
    /// Inline cod
    InlineCode { code: Str<'data> },
    /// An image.
    Image {
        target: Str<'data>,
        alt: Option<Str<'data>>,
        title: Option<Str<'data>>,
    },
    /// Raw HTML
    RawHtml { html: Str<'data> },
    /// Soft break (a space)
    SoftBreak,
    /// Hard break (rendered line break)
    HardBreak,
    /// Horizontal ruler
    Rule,
    /// A task list item.
    Checkbox { checked: bool },
    /// Links to a footnote
    FootnoteReference { target: Str<'data> },
}
