//! Data structures for modeling an HTTP request.

use crate::http_header::{GeneralHeader, HeaderValue, ReqHeader, SimpleHeaderValue};

use std::{collections, fmt};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum ReqVerb {
    Get,
}

impl fmt::Display for ReqVerb {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReqVerb::Get => f.write_str("GET"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ReqTarget {
    All,
    // path (url-decoded, original)
    Path(String, String),
}

impl fmt::Display for ReqTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    version: String,
    headers: collections::HashMap<ReqHeader, HeaderValue>,
    authentication_credentials: Option<(String, String)>,
}

impl ReqHead {
    pub fn new(
        verb: ReqVerb,
        target: ReqTarget,
        version: String,
        headers: collections::HashMap<ReqHeader, HeaderValue>,
        authentication_credentials: Option<(String, String)>,
    ) -> Self {
        Self {
            verb,
            target,
            version,
            headers,
            authentication_credentials,
        }
    }

    pub fn first_line(&self) -> String {
        format!("{} {} {}", self.verb, self.target, self.version)
    }

    pub fn should_close(&self) -> bool {
        self.headers
            .get(&ReqHeader::GeneralHeader(GeneralHeader::Connection))
            .is_some_and(|h| match h {
                HeaderValue::Simple(SimpleHeaderValue::Plain(v)) => v.eq("close"),
                _ => false,
            })
    }

    pub fn auth_creds(&self) -> Option<&(String, String)> {
        self.authentication_credentials.as_ref()
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

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HttpReq {
    date: chrono::DateTime<chrono::Utc>,
    head: ReqHead,
}

impl HttpReq {
    pub fn new(date: chrono::DateTime<chrono::Utc>, head: ReqHead) -> Self {
        Self { date, head }
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

    pub fn auth_creds(&self) -> Option<&(String, String)> {
        self.head.auth_creds()
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
                ReqTarget::Path(String::from(path), String::from(path)),
                String::from("HTTP/1.1"),
                collections::HashMap::new(),
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
                HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("foo"))),
            )]),
            None,
        );
        let mut req = HttpReq::new(now, req_head);

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
                HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("foo"))),
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
        );
        let mut req = HttpReq::new(chrono::Utc::now(), req_head);

        req.headers().insert(
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("close"))),
        );

        assert!(req.should_close());

        req.headers().insert(
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("keep-alive"))),
        );
        assert!(!req.should_close());
    }

    #[test]
    fn http_req_headers_test() {
        let mut headers = collections::HashMap::new();
        headers.insert(
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("close"))),
        );
        headers.insert(
            ReqHeader::ReqOnly(ReqOnlyHeader::Host),
            HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("rust-http-server"))),
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
