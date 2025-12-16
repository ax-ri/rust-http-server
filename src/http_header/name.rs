//! Data structures modeling HTTP header names.
//!
//! Header names are modeled using enumerations for known header name as defined in the HTTP specification.

use std::fmt;

/// HTTP header that can be part of both requests and responses.
#[derive(Debug, PartialEq, Eq, Hash)]
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

/// HTTP header that can only be part of an HTTP request.
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

/// HTTP header that can only be part of an HTTP response.
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

/// HTTP header used to give information about an entity.
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
