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

fn get_reason_phrase(status_code: u16) -> String {
    match status_code {
        200 => String::from("OK"),
        404 => String::from("Not Found"),
        500 => String::from("Internal Server Error"),
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

    pub fn set_status(&mut self, status_code: u16) {
        self.status_code = status_code
    }

    pub fn body(&self) -> &Option<Vec<u8>> {
        &self.body
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
            "HTTP/{} {} {}\r\n",
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
