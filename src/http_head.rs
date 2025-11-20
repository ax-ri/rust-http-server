use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub struct HttpHeaderParsedValue {
    pub original: String,
    pub parsed: Vec<(String, Vec<(String, String)>)>,
}

#[derive(Debug)]
pub enum GeneralHttpHeader {
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

#[derive(Debug)]
pub enum ReqOnlyHttpHeader {
    Accept(HttpHeaderParsedValue),
    AcceptCharset(HttpHeaderParsedValue),
    AcceptEncoding(HttpHeaderParsedValue),
    AcceptLanguage(HttpHeaderParsedValue),
    Authorization,
    Expect,
    From,
    Host(String),
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
    UserAgent(String),
}

#[derive(Debug)]
pub enum ResOnlyHttpHeader {
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

#[derive(Debug)]
pub enum EntityHttpHeader {
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

#[derive(Debug)]
pub enum HttpReqHeader {
    GeneralHeader(GeneralHttpHeader),
    ReqHeader(ReqOnlyHttpHeader),
    Other(String, String),
}

impl Display for HttpReqHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpReqHeader::GeneralHeader(h) => match h {
                _ => todo!(),
            },
            HttpReqHeader::ReqHeader(h) => match h {
                ReqOnlyHttpHeader::Accept(value) => write!(f, "Accept: {:?}", value.parsed),
                ReqOnlyHttpHeader::AcceptCharset(value) => {
                    write!(f, "Accept-Charset: {:?}", value.parsed)
                }
                ReqOnlyHttpHeader::AcceptEncoding(value) => {
                    write!(f, "Accept-Encoding: {:?}", value.parsed)
                }
                ReqOnlyHttpHeader::AcceptLanguage(value) => {
                    write!(f, "Accept-Language: {:?}", value.parsed)
                }
                ReqOnlyHttpHeader::Host(host) => write!(f, "Host: {}", host),
                ReqOnlyHttpHeader::UserAgent(host) => write!(f, "User-Agent: {}", host),
                _ => todo!(),
            },
            HttpReqHeader::Other(name, value) => write!(f, "{}: {}", name, value),
        }
    }
}

#[derive(Debug)]
pub enum HttpReqTarget {
    Path(PathBuf),
    Other(String),
}

#[derive(Debug)]
pub struct HttpReqHead {
    verb: String,
    target: HttpReqTarget,
    version: String,
    headers: HashMap<String, HttpReqHeader>,
}

impl HttpReqHead {
    pub fn new(
        verb: String,
        target: HttpReqTarget,
        version: String,
        headers: HashMap<String, HttpReqHeader>,
    ) -> Self {
        Self {
            verb,
            target,
            version,
            headers,
        }
    }
}

impl Display for HttpReqHead {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?} HTTP/{}", self.verb, self.target, self.version).and(
            self.headers
                .iter()
                .try_for_each(|(name, value)| write!(f, "{}: {:?}", name, value)),
        )
    }
}
