//! Data structures for modeling an HTTP response.

use crate::http_header::ResHeader;
use crate::http_req::HeaderValue;
use std::collections::HashMap;
use std::fmt::Display;

pub struct HttpRes {
    version: String,
    status_code: u16,
    headers: HashMap<ResHeader, HeaderValue>,
    body: Option<String>,
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

    pub fn set_status(&mut self, status_code: u16) {
        self.status_code = status_code
    }

    pub fn body(&self) -> &Option<String> {
        &self.body
    }

    pub fn set_body(&mut self, body: Option<String>) {
        self.body = body;
    }

    pub fn set_header(&mut self, name: ResHeader, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn has_header(&mut self, name: ResHeader) -> bool {
        self.headers.contains_key(&name)
    }
}

fn get_reason_phrase(status_code: u16) -> String {
    match status_code {
        200 => String::from("OK"),
        404 => String::from("Not Found"),
        500 => String::from("Internal Server Error"),
        _ => String::from("Unknown Error"),
    }
}

impl Display for HttpRes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = write!(
            f,
            "HTTP/{} {} {}\r\n",
            self.version,
            self.status_code,
            get_reason_phrase(self.status_code)
        )
        .and(
            self.headers
                .iter()
                .try_for_each(|(name, value)| write!(f, "{}: {}\r\n", name, value)),
        )
        .and(write!(f, "\r\n"));

        if let Some(body) = self.body.as_ref() {
            res = res.and(write!(f, "{}\r\n", body))
        }
        res
    }
}
