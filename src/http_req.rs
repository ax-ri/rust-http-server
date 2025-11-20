//! Data structures for modeling an HTTP request.

pub(crate) use crate::http_header::{HeaderValue, ReqHeader, ReqOnlyHeader};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

////////////////////////////////////////////////////////////////////////////////////////////////////

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
    headers: HashMap<ReqHeader, HeaderValue>,
}

impl HttpReqHead {
    pub fn new(
        verb: String,
        target: HttpReqTarget,
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

impl Display for HttpReqHead {
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
    head: HttpReqHead,
}

impl HttpReq {
    pub fn new(head: HttpReqHead) -> Self {
        Self { head }
    }

    pub fn version(&self) -> &str {
        self.head.version.as_str()
    }
}
