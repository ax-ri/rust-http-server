//! Data structures modeling HTTP header values.
//!
//! The header value itself is made of a main value, optionally followed by list of comma separated members.
//! A member itself can have some attributes, separated by a semicolon (;) and written with the syntax name=value.

use std::{collections, fmt};

/// Known names of header value members.
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HeaderValueMemberName {
    /// quality member (q)
    Quality,
    /// any other member
    Other(String),
}

impl HeaderValueMemberName {
    #[inline(always)]
    pub fn new_other(name: &str) -> Self {
        Self::Other(String::from(name))
    }
}

impl fmt::Display for HeaderValueMemberName {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quality => write!(f, "q"),
            Self::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Value types of header value members values.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HeaderValueMemberValue {
    /// A floating-point number
    Float(ordered_float::NotNan<f32>),
    /// Any other value
    Other(String),
}

impl HeaderValueMemberValue {
    #[inline(always)]
    pub fn new_float(f: f32) -> Self {
        Self::Float(ordered_float::NotNan::new(f).unwrap())
    }

    #[inline(always)]
    pub fn new_other(name: &str) -> Self {
        Self::Other(String::from(name))
    }
}

impl fmt::Display for HeaderValueMemberValue {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(x) => write!(f, "{}", x),
            Self::Other(name) => write!(f, "{}", name),
        }
    }
}

/// A simple header value (no member parsing).
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SimpleHeaderValue {
    /// Integer
    Number(u64),
    /// Plain string
    String(String),
    /// MIME type
    Mime(mime_guess::Mime),
}

impl fmt::Display for SimpleHeaderValue {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{}", s),
            Self::Mime(m) => write!(f, "{}", m.essence_str()),
        }
    }
}

/// A complex header value, with parsed members and member attributes
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ParsedHeaderValue(
    pub  Vec<(
        // main value
        SimpleHeaderValue,
        // attribute list
        collections::BTreeMap<HeaderValueMemberName, HeaderValueMemberValue>,
    )>,
);

impl fmt::Display for ParsedHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|(name, members)| {
                    format!(
                        "{}{}{}",
                        name,
                        if members.is_empty() { "" } else { ";" },
                        members
                            .iter()
                            .map(|(name, value)| format!("{}={}", name, value))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

/// A header value is either simple or parsed, or a pair of credentials.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HeaderValue {
    Simple(SimpleHeaderValue),
    Parsed(ParsedHeaderValue),
    Credentials(String, String),
}

impl fmt::Display for HeaderValue {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple(s) => write!(f, "{}", s),
            Self::Parsed(s) => write!(f, "{}", s),
            Self::Credentials(a, b) => write!(f, "{}:{}", a, b),
        }
    }
}
