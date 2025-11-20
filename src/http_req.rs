//! Data structures for modeling an HTTP request.

pub(crate) use crate::http_header::{HeaderValue, ReqHeader, ReqOnlyHeader};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ReqTarget {
    All,
    Path(String),
}

#[derive(Debug)]
pub struct ReqHead {
    verb: String,
    target: ReqTarget,
    version: String,
    headers: HashMap<ReqHeader, HeaderValue>,
}

impl ReqHead {
    pub fn new(
        verb: String,
        target: ReqTarget,
        version: String,
        headers: HashMap<ReqHeader, HeaderValue>,
    ) -> Self {
        Self {
            verb,
            target,
            version,
            headers,
        }
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
    head: ReqHead,
}

impl HttpReq {
    pub fn new(head: ReqHead) -> Self {
        Self { head }
    }

    pub fn version(&self) -> &str {
        self.head.version.as_str()
    }

    pub fn verb(&self) -> &str {
        self.head.verb.as_str()
    }

    pub fn target(&self) -> &ReqTarget {
        &self.head.target
    }
}
