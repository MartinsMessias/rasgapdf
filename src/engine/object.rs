use std::collections::BTreeMap;
use std::fmt;

/// Unique identifier for an indirect PDF object: (object_number, generation_number)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    pub number: u32,
    pub generation: u16,
}

impl ObjectId {
    pub fn new(number: u32, generation: u16) -> Self {
        Self { number, generation }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.number, self.generation)
    }
}

/// A PDF Object as defined in ISO 32000-1, Section 7.3
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Vec<u8>),
    Name(String),
    Array(Vec<PdfObject>),
    Dictionary(BTreeMap<String, PdfObject>),
    Reference(ObjectId),
    Stream(PdfStream),
}

/// A PDF Stream object (Dictionary + raw/decompressed bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    pub dict: BTreeMap<String, PdfObject>,
    pub data: Vec<u8>,
}

impl PdfObject {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PdfObject::Real(f) => Some(*f),
            PdfObject::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            PdfObject::Name(n) => Some(n.as_str()),
            _ => None,
        }
    }

    pub fn as_string_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(bytes) => Some(bytes.as_slice()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_string_bytes().and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&BTreeMap<String, PdfObject>> {
        match self {
            PdfObject::Dictionary(dict) => Some(dict),
            PdfObject::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            PdfObject::Reference(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    pub fn dict_get(&self, key: &str) -> Option<&PdfObject> {
        self.as_dict().and_then(|d| d.get(key))
    }
}
