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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneralHeader::CacheControl => write!(f, "Cache-Control"),
            GeneralHeader::Connection => write!(f, "Connection"),
            GeneralHeader::Date => write!(f, "Date"),
            GeneralHeader::Pragma => write!(f, "Pragma"),
            GeneralHeader::Trailer => write!(f, "Trailer"),
            GeneralHeader::TransferEncoding => write!(f, "Transfer-Encoding"),
            GeneralHeader::Upgrade => write!(f, "Upgrade"),
            GeneralHeader::Via => write!(f, "Via"),
            GeneralHeader::Warning => write!(f, "Warning"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HeaderValueMemberName {
    Quality,
    Other(String),
}

impl fmt::Display for HeaderValueMemberName {
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

impl fmt::Display for HeaderValueMemberValue {
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
                        "{};{}",
                        name,
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
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple(s) => write!(f, "{}", s),
            Self::Parsed(s) => write!(f, "{}", s),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReqOnlyHeader::Accept => write!(f, "Accept"),
            ReqOnlyHeader::AcceptCharset => write!(f, "Accept-Charset"),
            ReqOnlyHeader::AcceptEncoding => write!(f, "Accept-Encoding"),
            ReqOnlyHeader::AcceptLanguage => write!(f, "Accept-Language"),
            ReqOnlyHeader::Authorization => write!(f, "Authorization"),
            ReqOnlyHeader::Expect => write!(f, "Expect"),
            ReqOnlyHeader::From => write!(f, "From"),
            ReqOnlyHeader::Host => write!(f, "Host"),
            ReqOnlyHeader::IfMatch => write!(f, "If-Match"),
            ReqOnlyHeader::IfModifiedSince => write!(f, "If-Modified-Since"),
            ReqOnlyHeader::IfNoneMatch => write!(f, "If-None-Match"),
            ReqOnlyHeader::IfRange => write!(f, "If-Range"),
            ReqOnlyHeader::IfUnmodifiedSince => write!(f, "If-Unmodified-Since"),
            ReqOnlyHeader::MaxForwards => write!(f, "Max-Forwards"),
            ReqOnlyHeader::ProxyAuthorization => write!(f, "Proxy-Authorization"),
            ReqOnlyHeader::Range => write!(f, "Range"),
            ReqOnlyHeader::Referer => write!(f, "Referer"),
            ReqOnlyHeader::TE => write!(f, "TE"),
            ReqOnlyHeader::UserAgent => write!(f, "User-Agent"),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResOnlyHeader::AcceptRanges => write!(f, "Accept-Ranges"),
            ResOnlyHeader::Age => write!(f, "Age"),
            ResOnlyHeader::ETag => write!(f, "ETag"),
            ResOnlyHeader::Location => write!(f, "Location"),
            ResOnlyHeader::ProxyAuthenticate => write!(f, "Proxy-Authenticate"),
            ResOnlyHeader::RetryAfter => write!(f, "Retry-After"),
            ResOnlyHeader::Server => write!(f, "Server"),
            ResOnlyHeader::Vary => write!(f, "Vary"),
            ResOnlyHeader::WWWAuthenticate => write!(f, "WWW-Authenticate"),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityHeader::Allow => write!(f, "Allow"),
            EntityHeader::ContentEncoding => write!(f, "Content-Encoding"),
            EntityHeader::ContentLanguage => write!(f, "Content-Language"),
            EntityHeader::ContentLength => write!(f, "Content-Length"),
            EntityHeader::ContentLocation => write!(f, "Content-Location"),
            EntityHeader::ContentMD5 => write!(f, "Content-MD5"),
            EntityHeader::ContentRange => write!(f, "Content-Range"),
            EntityHeader::ContentType => write!(f, "Content-Type"),
            EntityHeader::Expires => write!(f, "Expires"),
            EntityHeader::LastModified => write!(f, "Last-Modified"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ReqHeader {
    GeneralHeader(GeneralHeader),
    ReqOnly(ReqOnlyHeader),
    Other(String),
}

impl fmt::Display for ReqHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReqHeader::GeneralHeader(h) => write!(f, "{}", h),
            ReqHeader::ReqOnly(h) => write!(f, "{}", h),
            ReqHeader::Other(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum ResHeader {
    GeneralHeader(GeneralHeader),
    ResOnlyHeader(ResOnlyHeader),
    EntityHeader(EntityHeader),
    Other(String),
}

impl fmt::Display for ResHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResHeader::GeneralHeader(h) => write!(f, "{}", h),
            ResHeader::ResOnlyHeader(h) => write!(f, "{}", h),
            ResHeader::EntityHeader(h) => write!(f, "{}", h),
            ResHeader::Other(name) => write!(f, "{}", name),
        }
    }
}
