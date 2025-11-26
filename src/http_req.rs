//! Data structures for modeling an HTTP request.

use crate::http_header::GeneralHeader;
pub(crate) use crate::http_header::{HeaderValue, ReqHeader, ReqOnlyHeader};
use ascii::AsciiString;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ReqVerb {
    Get,
}

impl Display for ReqVerb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReqVerb::Get => f.write_str("GET"),
        }
    }
}

#[derive(Debug)]
pub enum ReqTarget {
    All,
    // path (url-decoded, original)
    Path(String, AsciiString),
}

impl Display for ReqTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReqTarget::All => write!(f, "*"),
            ReqTarget::Path(_, original) => write!(f, "{}", original),
        }
    }
}

#[derive(Debug)]
pub struct ReqHead {
    verb: ReqVerb,
    target: ReqTarget,
    version: AsciiString,
    headers: HashMap<ReqHeader, HeaderValue>,
}

impl ReqHead {
    pub fn new(
        verb: ReqVerb,
        target: ReqTarget,
        version: AsciiString,
        headers: HashMap<ReqHeader, HeaderValue>,
    ) -> Self {
        Self {
            verb,
            target,
            version,
            headers,
        }
    }

    pub fn should_close(&self) -> bool {
        self.headers
            .get(&ReqHeader::GeneralHeader(GeneralHeader::Connection))
            .is_some_and(|h| match h {
                HeaderValue::Plain(v) => v.eq("close"),
                _ => false,
            })
    }
}

impl Display for ReqHead {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} HTTP/{}\r\n",
            self.verb, self.target, self.version
        )
        .and(
            self.headers
                .iter()
                .try_for_each(|(name, value)| write!(f, "{}: {}\r\n", name, value)),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HttpReq {
    date: DateTime<Utc>,
    head: ReqHead,
}

impl HttpReq {
    pub fn new(date: DateTime<Utc>, head: ReqHead) -> Self {
        Self { date, head }
    }

    pub fn date(&self) -> &DateTime<Utc> {
        &self.date
    }

    pub fn version(&self) -> &str {
        self.head.version.as_str()
    }

    pub fn verb(&self) -> &ReqVerb {
        &self.head.verb
    }

    pub fn target(&self) -> &ReqTarget {
        &self.head.target
    }

    pub fn first_line(&self) -> String {
        format!(
            "{} {} HTTP/{}",
            self.head.verb, self.head.target, self.head.version
        )
    }

    pub fn should_close(&self) -> bool {
        self.head.should_close()
    }
}
