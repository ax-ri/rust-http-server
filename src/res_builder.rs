use crate::http_header::{EntityHeader, GeneralHeader, HeaderValue, ResHeader, ResOnlyHeader};
use crate::http_res;
use crate::http_res::HttpRes;
use ascii::AsciiString;
use mime_guess::{MimeGuess, mime};
use std::cmp::Ordering;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;
use std::str::FromStr;

pub struct ResBuilder {
    res: HttpRes,
}

impl ResBuilder {
    pub fn new(version: &str) -> Self {
        Self {
            res: HttpRes::new(version),
        }
    }

    pub fn list_directory(
        &mut self,
        dir_path: &Path,
        rel_path: &str,
    ) -> Result<(), std::io::Error> {
        // set content type
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Plain(AsciiString::from_str(mime::TEXT_HTML.essence_str()).unwrap()),
        );

        let mut entries: Vec<DirEntry> = fs::read_dir(dir_path)?.map(|e| e.unwrap()).collect();
        entries.sort_by(|a, b| {
            match (
                a.metadata().unwrap().is_dir(),
                b.metadata().unwrap().is_dir(),
            ) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });
        let title = format!("Index of {}", rel_path);
        let html = format!(
            r##"<!DOCTYPE HTML>
                <html lang="en">
                    <head>
                        <meta charset="utf-8"/>
                        <title>{}</title>
                    </head>
                    <body>
                        <h1>{}</h1>
                        <hr/>
                        <ul>{}</ul>
                        <hr/>
                    </body>
                </html>
            "##,
            title,
            title,
            entries
                .iter()
                .map(|e| {
                    let file_name = e.file_name().into_string().unwrap();
                    let sep = if rel_path == "/" { "" } else { "/" };
                    format!(
                        r##"<li><a href="{}">{}{}</a></li>"##,
                        rel_path.to_owned() + sep + &file_name,
                        file_name,
                        if e.metadata().unwrap().is_dir() {
                            "/"
                        } else {
                            ""
                        },
                    )
                })
                .fold(String::new(), |acc, e| acc + e.as_str()),
        );

        // set content
        let content = Vec::from(html.as_bytes());
        self.res.set_body(Some(content));

        Ok(())
    }

    pub fn set_file_body(&mut self, file_path: &Path) -> Result<(), std::io::Error> {
        // set content type
        let mime_type = MimeGuess::from_path(file_path).first_or_octet_stream();
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Plain(AsciiString::from_str(mime_type.essence_str()).unwrap()),
        );

        // set content
        let content = fs::read(file_path)?;
        self.res.set_body(Some(content));

        Ok(())
    }

    pub fn build_error(&mut self, status_code: u16) -> &HttpRes {
        self.res.set_status(status_code);
        let title = format!(
            "{} {}",
            status_code,
            http_res::get_reason_phrase(status_code)
        );
        let message = format!(
            r##"<!DOCTYPE html>
                <html lang="en">
                    <head>
                        <meta charset="utf-8"/>
                        <title>{}</title>
                    </head>
                    <body>
                        <h1>{}</h1>
                    </body>
                </html>"##,
            title, title
        );
        self.res.set_body(Some(message.into_bytes()));
        self.do_build()
    }

    pub fn do_build(&mut self) -> &HttpRes {
        // set date if not already present
        if !self
            .res
            .has_header(ResHeader::GeneralHeader(GeneralHeader::Date))
        {
            self.res.set_header(
                ResHeader::GeneralHeader(GeneralHeader::Date),
                HeaderValue::Plain(
                    AsciiString::from_ascii(
                        chrono::Utc::now()
                            .format("%a, %d %b %Y %H:%M:%S GMT")
                            .to_string()
                            .as_bytes(),
                    )
                    .unwrap(),
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
                HeaderValue::Plain(AsciiString::from_str("rust-http-server").unwrap()),
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
