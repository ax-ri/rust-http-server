//! Data structures for modeling HTTP headers.

use std::{collections, fmt};

#[derive(Debug, PartialEq, Eq, Hash)]
/// Http header that can be part of requests and responses.
pub enum GeneralHeader {
    CacheControl,
    Connection,
    Date,
    Pragma,
    Trailer,
    TransferEncoding,
    Upgrade,
    Via,
    Warning,
}

impl fmt::Display for GeneralHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CacheControl => write!(f, "Cache-Control"),
            Self::Connection => write!(f, "Connection"),
            Self::Date => write!(f, "Date"),
            Self::Pragma => write!(f, "Pragma"),
            Self::Trailer => write!(f, "Trailer"),
            Self::TransferEncoding => write!(f, "Transfer-Encoding"),
            Self::Upgrade => write!(f, "Upgrade"),
            Self::Via => write!(f, "Via"),
            Self::Warning => write!(f, "Warning"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HeaderValueMemberName {
    Quality,
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HeaderValueMemberValue {
    Float(ordered_float::NotNan<f32>),
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SimpleHeaderValue {
    Number(u64),
    Plain(String),
    Mime(mime_guess::Mime),
}

impl fmt::Display for SimpleHeaderValue {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::Plain(s) => write!(f, "{}", s),
            Self::Mime(m) => write!(f, "{}", m.essence_str()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ParsedHeaderValue(
    pub  Vec<(
        SimpleHeaderValue,
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ReqOnlyHeader {
    Accept,
    AcceptCharset,
    AcceptEncoding,
    AcceptLanguage,
    Authorization,
    Expect,
    From,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    IfRange,
    IfUnmodifiedSince,
    MaxForwards,
    ProxyAuthorization,
    Range,
    Referer,
    TE,
    UserAgent,
}

impl fmt::Display for ReqOnlyHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept => write!(f, "Accept"),
            Self::AcceptCharset => write!(f, "Accept-Charset"),
            Self::AcceptEncoding => write!(f, "Accept-Encoding"),
            Self::AcceptLanguage => write!(f, "Accept-Language"),
            Self::Authorization => write!(f, "Authorization"),
            Self::Expect => write!(f, "Expect"),
            Self::From => write!(f, "From"),
            Self::Host => write!(f, "Host"),
            Self::IfMatch => write!(f, "If-Match"),
            Self::IfModifiedSince => write!(f, "If-Modified-Since"),
            Self::IfNoneMatch => write!(f, "If-None-Match"),
            Self::IfRange => write!(f, "If-Range"),
            Self::IfUnmodifiedSince => write!(f, "If-Unmodified-Since"),
            Self::MaxForwards => write!(f, "Max-Forwards"),
            Self::ProxyAuthorization => write!(f, "Proxy-Authorization"),
            Self::Range => write!(f, "Range"),
            Self::Referer => write!(f, "Referer"),
            Self::TE => write!(f, "TE"),
            Self::UserAgent => write!(f, "User-Agent"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum ResOnlyHeader {
    AcceptRanges,
    Age,
    ETag,
    Location,
    ProxyAuthenticate,
    RetryAfter,
    Server,
    Vary,
    WWWAuthenticate,
}

impl fmt::Display for ResOnlyHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptRanges => write!(f, "Accept-Ranges"),
            Self::Age => write!(f, "Age"),
            Self::ETag => write!(f, "ETag"),
            Self::Location => write!(f, "Location"),
            Self::ProxyAuthenticate => write!(f, "Proxy-Authenticate"),
            Self::RetryAfter => write!(f, "Retry-After"),
            Self::Server => write!(f, "Server"),
            Self::Vary => write!(f, "Vary"),
            Self::WWWAuthenticate => write!(f, "WWW-Authenticate"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum EntityHeader {
    Allow,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentMD5,
    ContentRange,
    ContentType,
    Expires,
    LastModified,
}

impl fmt::Display for EntityHeader {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::ContentEncoding => write!(f, "Content-Encoding"),
            Self::ContentLanguage => write!(f, "Content-Language"),
            Self::ContentLength => write!(f, "Content-Length"),
            Self::ContentLocation => write!(f, "Content-Location"),
            Self::ContentMD5 => write!(f, "Content-MD5"),
            Self::ContentRange => write!(f, "Content-Range"),
            Self::ContentType => write!(f, "Content-Type"),
            Self::Expires => write!(f, "Expires"),
            Self::LastModified => write!(f, "Last-Modified"),
        }
    }
}

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

    #[test]
    fn display_parsed_header_value_test() {
        let mut h = ParsedHeaderValue(vec![(
            SimpleHeaderValue::Plain(String::from("my header value")),
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
