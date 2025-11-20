use crate::http_header::HttpResHeader;
use std::collections::HashSet;
use std::fmt::Display;

pub struct HttpRes {
    version: String,
    status_code: u16,
    headers: HashSet<HttpResHeader>,
    body: Option<String>,
}

impl HttpRes {
    pub fn new(
        version: &str,
        status_code: u16,
        headers: HashSet<HttpResHeader>,
        body: Option<String>,
    ) -> Self {
        Self {
            version: String::from(version),
            status_code,
            headers,
            body,
        }
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
        .and(self.headers.iter().try_for_each(|h| write!(f, "{}\r\n", h)))
        .and(write!(f, "\r\n"));

        if let Some(body) = self.body.as_ref() {
            res = res.and(write!(f, "{}\r\n", body))
        }
        res
    }
}
