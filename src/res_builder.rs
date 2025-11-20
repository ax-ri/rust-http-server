use crate::http_header::{EntityHeader, GeneralHeader, HeaderValue, ResHeader, ResOnlyHeader};
use crate::http_res::HttpRes;
use mime_guess::MimeGuess;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ResBuilder {
    res: HttpRes,
}

impl ResBuilder {
    pub fn new(version: &str) -> Self {
        Self {
            res: HttpRes::new(version),
        }
    }

    pub fn set_file_body(&mut self, file_path: &Path) -> Result<(), std::io::Error> {
        // set content type
        let mime_type = MimeGuess::from_path(file_path).first_or_text_plain();
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Plain(String::from(mime_type.essence_str())),
        );

        // set content
        let content = fs::read(file_path)?;
        self.res.set_body(Some(content));

        Ok(())
    }

    pub fn build_error(&mut self, status_code: u16) -> &HttpRes {
        self.res.set_status(status_code);
        self.res
            .set_body(Some(format!("Error {}", status_code).into_bytes()));
        self.do_build()
    }

    pub fn do_build(&mut self) -> &HttpRes {
        let mut res_headers = HashMap::new();
        res_headers.insert(
            ResHeader::ResOnlyHeader(ResOnlyHeader::Server),
            HeaderValue::Plain(String::from("Me")),
        );

        // set date if not already present
        if !self
            .res
            .has_header(ResHeader::GeneralHeader(GeneralHeader::Date))
        {
            self.res.set_header(
                ResHeader::GeneralHeader(GeneralHeader::Date),
                HeaderValue::Plain(
                    chrono::Utc::now()
                        .format("%a, %d %b %Y %H:%M:%S GMT")
                        .to_string(),
                ),
            );
        }

        // set server origin if not already present
        if !self
            .res
            .has_header(ResHeader::ResOnlyHeader(ResOnlyHeader::Server))
        {
            self.res.set_header(
                ResHeader::ResOnlyHeader(ResOnlyHeader::Server),
                HeaderValue::Plain(String::from("rust-http-server")),
            );
        }

        // set content-length
        if let Some(body) = self.res.body() {
            self.res.set_header(
                ResHeader::EntityHeader(EntityHeader::ContentLength),
                HeaderValue::Number(body.len() as i32),
            )
        }

        &self.res
    }
}
