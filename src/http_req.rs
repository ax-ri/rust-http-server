//! Data structures for modeling an HTTP request.
//!
//! An HTTP request is represented as two parts: a head (first line and headers) and an optional body.

pub(crate) use crate::http_header::{
    EntityHeader, GeneralHeader, HeaderValue, ReqHeader, SimpleHeaderValue,
};
use crate::req_parser::SupportedEncoding;

use std::{collections, fmt};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// HTTP request verb.
#[derive(Debug, PartialEq)]
pub enum ReqVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl fmt::Display for ReqVerb {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => f.write_str("GET"),
            Self::Post => f.write_str("POST"),
            Self::Put => f.write_str("PUT"),
            Self::Patch => f.write_str("PATCH"),
            Self::Delete => f.write_str("DELETE"),
        }
    }
}

/// The path of an HTTP request. This path is URL-encoded, and can contain query params.
#[derive(Debug, PartialEq)]
pub struct ReqPath {
    // url-encoded path
    pub original: String,
    // url-decoded path
    pub decoded: String,
    // query params
    pub query: String,
}

/// The target can be either a path or a star '*' for OPTIONS requests.
#[derive(Debug, PartialEq)]
pub enum ReqTarget {
    All,
    Path(ReqPath),
}

impl fmt::Display for ReqTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "*"),
            Self::Path(ReqPath { original, .. }) => write!(f, "{}", original),
        }
    }
}

/// HTTP Request head: first line and headers.
#[derive(Debug)]
pub struct ReqHead {
    verb: ReqVerb,
    target: ReqTarget,
    version: String,
    headers: collections::HashMap<ReqHeader, HeaderValue>,
    authentication_credentials: Option<(String, String)>,
    encoding: Option<SupportedEncoding>,
}

impl ReqHead {
    pub fn new(
        verb: ReqVerb,
        target: ReqTarget,
        version: String,
        headers: collections::HashMap<ReqHeader, HeaderValue>,
        authentication_credentials: Option<(String, String)>,
        encoding: Option<SupportedEncoding>,
    ) -> Self {
        Self {
            verb,
            target,
            version,
            headers,
            authentication_credentials,
            encoding,
        }
    }

    pub fn first_line(&self) -> String {
        format!("{} {} {}", self.verb, self.target, self.version)
    }

    pub fn should_close(&self) -> bool {
        self.headers
            .get(&ReqHeader::General(GeneralHeader::Connection))
            .is_some_and(|h| match h {
                HeaderValue::Simple(SimpleHeaderValue::String(v)) => v.eq("close"),
                _ => false,
            })
    }

    pub fn accepted_encoding(&self) -> Option<&SupportedEncoding> {
        self.encoding.as_ref()
    }

    pub fn auth_creds(&self) -> Option<&(String, String)> {
        self.authentication_credentials.as_ref()
    }

    pub fn body_len(&self) -> usize {
        self.headers
            .get(&ReqHeader::Entity(EntityHeader::ContentLength))
            .map(|v| match *v {
                HeaderValue::Simple(SimpleHeaderValue::Number(n)) => n as usize,
                _ => 0,
            })
            .unwrap_or(0)
    }

    pub fn body_encoding(&self) -> Option<&HeaderValue> {
        self.headers
            .get(&ReqHeader::Entity(EntityHeader::ContentEncoding))
    }

    pub fn body_type(&self) -> Option<&str> {
        self.headers
            .get(&ReqHeader::Entity(EntityHeader::ContentType))
            .map(|v| match v {
                HeaderValue::Simple(SimpleHeaderValue::String(s)) => s,
                _ => "",
            })
    }
}

impl fmt::Display for ReqHead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\r\n", self.first_line()).and(
            self.headers
                .iter()
                .try_for_each(|(name, value)| write!(f, "{}: {}\r\n", name, value)),
        )
    }
}

/// HTTP request body
#[derive(Debug)]
pub struct ReqBody {
    bytes: Vec<u8>,
    content_type: String,
}

impl ReqBody {
    pub fn new(bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            bytes,
            content_type,
        }
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn content_type(&self) -> &str {
        self.content_type.as_str()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// An HTTP request.
pub struct HttpReq {
    date: chrono::DateTime<chrono::Utc>,
    head: ReqHead,
    body: Option<ReqBody>,
}

impl HttpReq {
    pub fn new(date: chrono::DateTime<chrono::Utc>, head: ReqHead, body: Option<ReqBody>) -> Self {
        Self { date, head, body }
    }

    pub fn date(&self) -> &chrono::DateTime<chrono::Utc> {
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
        self.head.first_line()
    }

    pub fn should_close(&self) -> bool {
        self.head.should_close()
    }

    pub fn headers(&mut self) -> &mut collections::HashMap<ReqHeader, HeaderValue> {
        &mut self.head.headers
    }

    pub fn accepted_encoding(&self) -> Option<&SupportedEncoding> {
        self.head.accepted_encoding()
    }

    pub fn auth_creds(&self) -> Option<&(String, String)> {
        self.head.auth_creds()
    }

    pub fn body(&self) -> Option<&ReqBody> {
        self.body.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_header::ReqOnlyHeader;

    #[test]
    fn http_req_head_path_test() {
        let paths = vec!["/", "/file.html", "/dir/file.html"];
        for path in paths {
            let req_head = ReqHead::new(
                ReqVerb::Get,
                ReqTarget::Path(ReqPath {
                    original: String::from(path),
                    decoded: String::from(path),
                    query: String::new(),
                }),
                String::from("HTTP/1.1"),
                collections::HashMap::new(),
                None,
                None,
            );
            assert_eq!(
                format!("{}", req_head),
                format!("GET {} HTTP/1.1\r\n", path.to_string())
            );
        }
    }

    #[test]
    fn http_req_simple_test() {
        let now = chrono::Utc::now();
        let req_head = ReqHead::new(
            ReqVerb::Get,
            ReqTarget::All,
            String::from("HTTP/1.1"),
            collections::HashMap::from([(
                ReqHeader::ReqOnly(ReqOnlyHeader::Host),
                HeaderValue::Simple(SimpleHeaderValue::String(String::from("foo"))),
            )]),
            None,
            None,
        );
        let mut req = HttpReq::new(now, req_head, None);

        assert_eq!(*req.date(), now);
        assert_eq!(req.version(), "HTTP/1.1");
        assert_eq!(*req.verb(), ReqVerb::Get);
        assert_eq!(*req.target(), ReqTarget::All);

        assert_eq!(req.first_line(), "GET * HTTP/1.1");
        assert!(!req.should_close());
        assert_eq!(
            req.headers(),
            &mut collections::HashMap::from([(
                ReqHeader::ReqOnly(ReqOnlyHeader::Host),
                HeaderValue::Simple(SimpleHeaderValue::String(String::from("foo"))),
            )])
        );
    }

    #[test]
    fn http_req_should_close_test() {
        let req_head = ReqHead::new(
            ReqVerb::Get,
            ReqTarget::All,
            String::from("HTTP/1.1"),
            collections::HashMap::new(),
            None,
            None,
        );
        let mut req = HttpReq::new(chrono::Utc::now(), req_head, None);

        req.headers().insert(
            ReqHeader::General(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("close"))),
        );

        assert!(req.should_close());

        req.headers().insert(
            ReqHeader::General(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("keep-alive"))),
        );
        assert!(!req.should_close());
    }

    #[test]
    fn http_req_headers_test() {
        let mut headers = collections::HashMap::new();
        headers.insert(
            ReqHeader::General(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("close"))),
        );
        headers.insert(
            ReqHeader::ReqOnly(ReqOnlyHeader::Host),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("rust-http-server"))),
        );
        headers.insert(
            ReqHeader::ReqOnly(ReqOnlyHeader::Accept),
            HeaderValue::Simple(SimpleHeaderValue::Mime(
                mime_guess::mime::APPLICATION_OCTET_STREAM,
            )),
        );
        headers.insert(
            ReqHeader::Other(String::from("X-My-Header")),
            HeaderValue::Simple(SimpleHeaderValue::Number(1234)),
        );

        let req_head = ReqHead::new(
            ReqVerb::Get,
            ReqTarget::All,
            String::from("HTTP/1.1"),
            headers,
            None,
            None,
        );
        let fmt = format!("{}", req_head);
        assert!(fmt.starts_with("GET * HTTP/1.1\r\n"));
        // the header order is non-deterministic (hashmap.iter())
        assert!(fmt.contains("X-My-Header: 1234\r\n"));
        assert!(fmt.contains("Connection: close\r\n"));
        assert!(fmt.contains("Accept: application/octet-stream\r\n"));
        assert!(fmt.contains("Host: rust-http-server\r\n"));
    }
}
