//! Data structures for modeling an HTTP Request.

pub(crate) use crate::http_header::{HttpHeaderValue, HttpReqHeader, ReqOnlyHttpHeader};
use std::collections::HashSet;
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
    headers: HashSet<HttpReqHeader>,
}

impl HttpReqHead {
    pub fn new(
        verb: String,
        target: HttpReqTarget,
        version: String,
        headers: HashSet<HttpReqHeader>,
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
                .try_for_each(|value| write!(f, "{:?}\r\n", value)),
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
