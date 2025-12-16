//! Data structures for modeling an HTTP response.
//!
//! An HTTP response is made of a first line, some headers, and a body.

use crate::http_header::{HeaderValue, ResHeader};

use std::collections;

#[inline(always)]
#[cfg_attr(coverage, coverage(off))]
pub fn get_reason_phrase(status_code: u16) -> String {
    match status_code {
        100 => String::from("Continue"),
        101 => String::from("Switching Protocols"),
        200 => String::from("OK"),
        201 => String::from("Created"),
        202 => String::from("Accepted"),
        203 => String::from("Non-Authoritative Information"),
        204 => String::from("No Content"),
        205 => String::from("Reset Content"),
        206 => String::from("Partial Content"),
        300 => String::from("Multiple Choices"),
        301 => String::from("Moved Permanently"),
        302 => String::from("Found"),
        303 => String::from("See Other"),
        304 => String::from("Not Modified"),
        305 => String::from("Use Proxy"),
        307 => String::from("Temporary Redirect"),
        400 => String::from("Bad Request"),
        401 => String::from("Unauthorized"),
        402 => String::from("Payment Required"),
        403 => String::from("Forbidden"),
        404 => String::from("Not Found"),
        405 => String::from("Method Not Allowed"),
        406 => String::from("Not Acceptable"),
        407 => String::from("Proxy Authentication Required"),
        408 => String::from("Request Timeout"),
        409 => String::from("Conflict"),
        410 => String::from("Gone"),
        411 => String::from("Length Required"),
        412 => String::from("Precondition Failed"),
        413 => String::from("Payload Too Large"),
        414 => String::from("URI Too Long"),
        415 => String::from("Unsupported Media Type"),
        416 => String::from("Range Not Satisfiable"),
        417 => String::from("Expectation Failed"),
        426 => String::from("Upgrade Required"),
        500 => String::from("Internal Server Error"),
        501 => String::from("Not Implemented"),
        502 => String::from("Bad Gateway"),
        503 => String::from("Service Unavailable"),
        504 => String::from("Gateway Timeout"),
        505 => String::from("HTTP Version Not Supported"),
        _ => String::from("Unknown Error"),
    }
}

/// HTTP response body
pub enum ResBody {
    Bytes(Vec<u8>),
    Stream(tokio::fs::File, u64),
}

impl ResBody {
    pub fn len(&self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes.len(),
            Self::Stream(_, len) => *len as usize,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Bytes(bytes) => bytes.is_empty(),
            Self::Stream(_, len) => *len == 0,
        }
    }
}

pub struct HttpRes {
    version: String,
    status_code: u16,
    headers: collections::HashMap<ResHeader, HeaderValue>,
    body: Option<ResBody>,
    raw_headers: Option<String>,
}

impl HttpRes {
    pub fn new(version: &str) -> Self {
        Self {
            version: String::from(version),
            status_code: 200,
            headers: collections::HashMap::new(),
            body: None,
            raw_headers: None,
        }
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn set_status(&mut self, status_code: u16) {
        self.status_code = status_code
    }

    pub fn has_header(&mut self, name: ResHeader) -> bool {
        self.headers.contains_key(&name)
    }

    pub fn set_header(&mut self, name: ResHeader, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn set_raw_headers(&mut self, headers: String) {
        self.raw_headers = Some(headers);
    }

    /// Generate the bytes corresponding to the response head (first line and headers)
    /// These bytes must be dynamically generated, contrary to the response body that can be read
    /// from a stream (typically, a static file on the filesystem).
    pub fn head_bytes(&self) -> Vec<u8> {
        let mut res_string = String::new();
        res_string.push_str(&format!(
            "{} {} {}\r\n",
            self.version,
            self.status_code,
            get_reason_phrase(self.status_code)
        ));

        self.headers
            .iter()
            .for_each(|(name, value)| res_string.push_str(&format!("{}: {}\r\n", name, value)));

        if let Some(h) = self.raw_headers.as_ref() {
            res_string.push_str(h);
        }

        res_string.push_str("\r\n");

        res_string.into_bytes()
    }

    pub fn body_ref(&self) -> Option<&ResBody> {
        self.body.as_ref()
    }

    pub fn body_mut(&mut self) -> Option<&mut ResBody> {
        self.body.as_mut()
    }

    pub fn body_len(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.len())
    }

    pub fn set_body(&mut self, body: Option<ResBody>) {
        self.body = body;
    }

    pub fn headers(&mut self) -> &mut collections::HashMap<ResHeader, HeaderValue> {
        &mut self.headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_header::{GeneralHeader, ResOnlyHeader, SimpleHeaderValue};

    #[test]
    fn http_res_test() {
        let mut res = HttpRes::new("HTTP/1.1");
        res.set_status(200);
        res.set_header(
            ResHeader::General(GeneralHeader::Connection),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("keep-alive"))),
        );
        res.set_header(
            ResHeader::ResOnly(ResOnlyHeader::Server),
            HeaderValue::Simple(SimpleHeaderValue::String(String::from("rust-http-server"))),
        );

        assert_eq!(res.status_code(), 200);
        assert!(res.has_header(ResHeader::General(GeneralHeader::Connection)));
        assert!(res.has_header(ResHeader::ResOnly(ResOnlyHeader::Server)));
        assert_eq!(
            res.headers,
            collections::HashMap::from([
                (
                    ResHeader::General(GeneralHeader::Connection),
                    HeaderValue::Simple(SimpleHeaderValue::String(String::from("keep-alive")))
                ),
                (
                    ResHeader::ResOnly(ResOnlyHeader::Server),
                    HeaderValue::Simple(SimpleHeaderValue::String(String::from(
                        "rust-http-server"
                    ))),
                )
            ])
        );

        let bytes = res.head_bytes();
        assert!(bytes.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(
            b"Connection: keep-alive\r\n"
                .iter()
                .all(|b| bytes.contains(b))
        );
        assert!(
            b"Server: rust-http-server\r\n"
                .iter()
                .all(|b| bytes.contains(b))
        );
    }

    #[test]
    fn http_res_body_test() {
        let mut res = HttpRes::new("HTTP/1.1");
        assert_eq!(res.body_len(), 0);
        assert!(res.body_ref().is_none());
        assert!(res.body_mut().is_none());

        res.set_body(Some(ResBody::Bytes(vec![0, 1, 2, 3, 4])));
        assert_eq!(res.body_len(), 5);
        assert!(res.body_ref().is_some());
        assert!(res.body_mut().is_some());
        assert_eq!(res.body_ref().unwrap().len(), 5);
        assert_eq!(res.body_mut().unwrap().len(), 5);
        assert!(!res.body_ref().unwrap().is_empty());
        assert!(!res.body_mut().unwrap().is_empty());
    }
}
