//! Data structures for modeling an HTTP response.

use crate::http_header::ResHeader;
use crate::http_req::HeaderValue;
use std::collections::HashMap;

pub struct HttpRes {
    version: String,
    status_code: u16,
    headers: HashMap<ResHeader, HeaderValue>,
    body: Option<Vec<u8>>,
}

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

impl HttpRes {
    pub fn new(version: &str) -> Self {
        Self {
            version: String::from(version),
            status_code: 200,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn set_status(&mut self, status_code: u16) {
        self.status_code = status_code
    }

    pub fn body(&self) -> &Option<Vec<u8>> {
        &self.body
    }

    pub fn body_len(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.len())
    }

    pub fn set_body(&mut self, body: Option<Vec<u8>>) {
        if let Some(mut content) = body {
            content.push(b'\r');
            content.push(b'\n');
            self.body = Some(content);
        } else {
            self.body = None;
        }
    }

    pub fn set_header(&mut self, name: ResHeader, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn has_header(&mut self, name: ResHeader) -> bool {
        self.headers.contains_key(&name)
    }

    pub fn to_bytes(&self) -> (Vec<u8>, Option<&Vec<u8>>) {
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

        res_string.push_str("\r\n");

        (res_string.into_bytes(), Option::from(&self.body))
    }
}
