//! Data structures for modeling HTTP headers.

use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq, Hash)]
/// Http header that can be part of requests and responses.
pub enum GeneralHttpHeader {
    CacheControl(HttpHeaderValue),
    Connection(HttpHeaderValue),
    Date(HttpHeaderValue),
    Pragma(HttpHeaderValue),
    Trailer(HttpHeaderValue),
    TransferEncoding(HttpHeaderValue),
    Upgrade(HttpHeaderValue),
    Via(HttpHeaderValue),
    Warning(HttpHeaderValue),
}

impl Display for GeneralHttpHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralHttpHeader::CacheControl(v) => write!(f, "Cache-Control: {}", v),
            GeneralHttpHeader::Connection(v) => write!(f, "Connection: {}", v),
            GeneralHttpHeader::Date(v) => write!(f, "Date: {}", v),
            GeneralHttpHeader::Pragma(v) => write!(f, "Pragma: {}", v),
            GeneralHttpHeader::Trailer(v) => write!(f, "Trailer: {}", v),
            GeneralHttpHeader::TransferEncoding(v) => write!(f, "Transfer-Encoding: {}", v),
            GeneralHttpHeader::Upgrade(v) => write!(f, "Upgrade: {}", v),
            GeneralHttpHeader::Via(v) => write!(f, "Via: {}", v),
            GeneralHttpHeader::Warning(v) => write!(f, "Warning: {}", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HttpHeaderValue {
    Plain(String),
    Parsed(Vec<(String, Vec<(String, String)>)>),
}

impl Display for HttpHeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpHeaderValue::Plain(s) => write!(f, "{}", s),
            HttpHeaderValue::Parsed(_) => todo!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ReqOnlyHttpHeader {
    Accept(HttpHeaderValue),
    AcceptCharset(HttpHeaderValue),
    AcceptEncoding(HttpHeaderValue),
    AcceptLanguage(HttpHeaderValue),
    Authorization(HttpHeaderValue),
    Expect(HttpHeaderValue),
    From(HttpHeaderValue),
    Host(HttpHeaderValue),
    IfMatch(HttpHeaderValue),
    IfModifiedSince(HttpHeaderValue),
    IfNoneMatch(HttpHeaderValue),
    IfRange(HttpHeaderValue),
    IfUnmodifiedSince(HttpHeaderValue),
    MaxForwards(HttpHeaderValue),
    ProxyAuthorization(HttpHeaderValue),
    Range(HttpHeaderValue),
    Referer(HttpHeaderValue),
    TE(HttpHeaderValue),
    UserAgent(HttpHeaderValue),
}

impl Display for ReqOnlyHttpHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReqOnlyHttpHeader::Accept(v) => write!(f, "Accept: {}", v),
            ReqOnlyHttpHeader::AcceptCharset(v) => write!(f, "Accept-Charset: {}", v),
            ReqOnlyHttpHeader::AcceptEncoding(v) => write!(f, "Accept-Encoding: {}", v),
            ReqOnlyHttpHeader::AcceptLanguage(v) => write!(f, "Accept-Language: {}", v),
            ReqOnlyHttpHeader::Authorization(v) => write!(f, "Authorization: {}", v),
            ReqOnlyHttpHeader::Expect(v) => write!(f, "Expect: {}", v),
            ReqOnlyHttpHeader::From(v) => write!(f, "From: {}", v),
            ReqOnlyHttpHeader::Host(v) => write!(f, "Host: {}", v),
            ReqOnlyHttpHeader::IfMatch(v) => write!(f, "If-Match: {}", v),
            ReqOnlyHttpHeader::IfModifiedSince(v) => write!(f, "If-Modified-Since: {}", v),
            ReqOnlyHttpHeader::IfNoneMatch(v) => write!(f, "If-None-Match: {}", v),
            ReqOnlyHttpHeader::IfRange(v) => write!(f, "If-Range: {}", v),
            ReqOnlyHttpHeader::IfUnmodifiedSince(v) => write!(f, "If-Unmodified-Since: {}", v),
            ReqOnlyHttpHeader::MaxForwards(v) => write!(f, "Max-Forwards: {}", v),
            ReqOnlyHttpHeader::ProxyAuthorization(v) => write!(f, "Proxy-Authorization: {}", v),
            ReqOnlyHttpHeader::Range(v) => write!(f, "Range: {}", v),
            ReqOnlyHttpHeader::Referer(v) => write!(f, "Referer: {}", v),
            ReqOnlyHttpHeader::TE(v) => write!(f, "TE: {}", v),
            ReqOnlyHttpHeader::UserAgent(v) => write!(f, "User-Agent: {}", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ResOnlyHttpHeader {
    AcceptRanges(HttpHeaderValue),
    Age(HttpHeaderValue),
    ETag(HttpHeaderValue),
    Location(HttpHeaderValue),
    ProxyAuthenticate(HttpHeaderValue),
    RetryAfter(HttpHeaderValue),
    Server(HttpHeaderValue),
    Vary(HttpHeaderValue),
    WWWAuthenticate(HttpHeaderValue),
}

impl Display for ResOnlyHttpHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResOnlyHttpHeader::AcceptRanges(v) => write!(f, "Accept-Ranges: {}", v),
            ResOnlyHttpHeader::Age(v) => write!(f, "Age: {}", v),
            ResOnlyHttpHeader::ETag(v) => write!(f, "ETag: {}", v),
            ResOnlyHttpHeader::Location(v) => write!(f, "Location: {}", v),
            ResOnlyHttpHeader::ProxyAuthenticate(v) => write!(f, "Proxy-Authenticate: {}", v),
            ResOnlyHttpHeader::RetryAfter(v) => write!(f, "Retry-After: {}", v),
            ResOnlyHttpHeader::Server(v) => write!(f, "Server: {}", v),
            ResOnlyHttpHeader::Vary(v) => write!(f, "Vary: {}", v),
            ResOnlyHttpHeader::WWWAuthenticate(v) => write!(f, "WWW-Authenticate: {}", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EntityHttpHeader {
    Allow(HttpHeaderValue),
    ContentEncoding(HttpHeaderValue),
    ContentLanguage(HttpHeaderValue),
    ContentLength(HttpHeaderValue),
    ContentLocation(HttpHeaderValue),
    ContentMD5(HttpHeaderValue),
    ContentRange(HttpHeaderValue),
    ContentType(HttpHeaderValue),
    Expires(HttpHeaderValue),
    LastModified(HttpHeaderValue),
}

impl Display for EntityHttpHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityHttpHeader::Allow(v) => write!(f, "Allow: {}", v),
            EntityHttpHeader::ContentEncoding(v) => write!(f, "Content-Encoding: {}", v),
            EntityHttpHeader::ContentLanguage(v) => write!(f, "Content-Language: {}", v),
            EntityHttpHeader::ContentLength(v) => write!(f, "Content-Length: {}", v),
            EntityHttpHeader::ContentLocation(v) => write!(f, "Content-Location: {}", v),
            EntityHttpHeader::ContentMD5(v) => write!(f, "Content-MD5: {}", v),
            EntityHttpHeader::ContentRange(v) => write!(f, "Content-Range: {}", v),
            EntityHttpHeader::ContentType(v) => write!(f, "Content-Type: {}", v),
            EntityHttpHeader::Expires(v) => write!(f, "Expires: {}", v),
            EntityHttpHeader::LastModified(v) => write!(f, "Last-Modified: {}", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HttpReqHeader {
    GeneralHeader(GeneralHttpHeader),
    ReqHeader(ReqOnlyHttpHeader),
    Other(String, String),
}

impl Display for HttpReqHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpReqHeader::GeneralHeader(h) => write!(f, "{}", h),
            HttpReqHeader::ReqHeader(h) => write!(f, "{}", h),
            HttpReqHeader::Other(name, value) => write!(f, "{}: {}", name, value),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HttpResHeader {
    GeneralHeader(GeneralHttpHeader),
    ResHeader(ResOnlyHttpHeader),
    Other(String, String),
}

impl Display for HttpResHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpResHeader::GeneralHeader(h) => write!(f, "{}", h),
            HttpResHeader::ResHeader(h) => write!(f, "{}", h),
            HttpResHeader::Other(name, value) => write!(f, "{}: {}", name, value),
        }
    }
}
