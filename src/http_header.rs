//! Data structures for modeling HTTP headers.
//!
//! An HTTP header is made of two parts: a name and a value, separated by a colon (:) when written.

// Implementation note:
// We use the display trait to stringify the data structures, when they need to be written on the socket
// because we consider this to be user-facing output.

mod name;
pub use name::EntityHeader;
pub use name::GeneralHeader;
pub use name::ReqOnlyHeader;
pub use name::ResOnlyHeader;

mod value;
pub use value::HeaderValue;
pub use value::HeaderValueMemberName;
pub use value::HeaderValueMemberValue;
pub use value::ParsedHeaderValue;
pub use value::SimpleHeaderValue;

use std::fmt;

/// Headers that can only be present in an HTTP request.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ReqHeader {
    General(GeneralHeader),
    ReqOnly(ReqOnlyHeader),
    Entity(EntityHeader),
    Other(String),
}

impl fmt::Display for ReqHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::General(h) => write!(f, "{}", h),
            Self::ReqOnly(h) => write!(f, "{}", h),
            Self::Entity(h) => write!(f, "{}", h),
            Self::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Headers that can only be present in an HTTP response.
#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum ResHeader {
    General(GeneralHeader),
    ResOnly(ResOnlyHeader),
    Entity(EntityHeader),
    Other(String),
}

impl fmt::Display for ResHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::General(h) => write!(f, "{}", h),
            Self::ResOnly(h) => write!(f, "{}", h),
            Self::Entity(h) => write!(f, "{}", h),
            Self::Other(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections;

    #[test]
    fn display_parsed_header_value_test() {
        let mut h = ParsedHeaderValue(vec![(
            SimpleHeaderValue::String(String::from("my header value")),
            collections::BTreeMap::new(),
        )]);
        assert_eq!(format!("{}", h), "my header value");

        h = ParsedHeaderValue(vec![(
            SimpleHeaderValue::Number(68),
            collections::BTreeMap::from([(
                HeaderValueMemberName::Quality,
                HeaderValueMemberValue::new_float(0.32),
            )]),
        )]);
        assert_eq!(format!("{}", h), "68;q=0.32");

        h = ParsedHeaderValue(vec![(
            SimpleHeaderValue::Mime(mime_guess::mime::TEXT_HTML),
            collections::BTreeMap::from([
                (
                    HeaderValueMemberName::Quality,
                    HeaderValueMemberValue::new_float(0.32),
                ),
                (
                    HeaderValueMemberName::Other(String::from("my-attr")),
                    HeaderValueMemberValue::Other(String::from("my attribute value")),
                ),
                (
                    HeaderValueMemberName::Other(String::from("my-other-attr")),
                    HeaderValueMemberValue::Other(String::from("my other attribute value")),
                ),
            ]),
        )]);
        assert_eq!(
            format!("{}", h),
            "text/html;q=0.32,my-attr=my attribute value,my-other-attr=my other attribute value"
        );
    }
}
