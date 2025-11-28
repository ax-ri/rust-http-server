use crate::http_header::{
    EntityHeader, GeneralHeader, HeaderValue, HeaderValueMemberName, HeaderValueMemberValue,
    ResHeader, ResOnlyHeader,
};
use crate::http_res;
use crate::http_res::{HttpRes, ResBody};
use mime_guess::{MimeGuess, mime};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::fs::DirEntry;
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

    fn set_default_content_type(&mut self) {
        // set content type
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Parsed(vec![(
                String::from(mime::TEXT_HTML.essence_str()),
                BTreeMap::from([(HeaderValueMemberName::Charset, HeaderValueMemberValue::UTF8)]),
            )]),
        );
    }

    pub fn list_directory(
        &mut self,
        dir_path: &Path,
        rel_path: &str,
    ) -> Result<(), std::io::Error> {
        self.set_default_content_type();

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
        self.res.set_body(Some(ResBody::Bytes(content)));

        Ok(())
    }

    pub fn set_file_body(&mut self, file_path: &Path) -> Result<(), std::io::Error> {
        // set content type
        let mime_type = MimeGuess::from_path(file_path).first_or_octet_stream();
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Plain(String::from(mime_type.essence_str())),
        );

        let metadata = fs::metadata(file_path)?;
        // set content
        if metadata.len() > 1024 * 1024 {
            let file = fs::File::open(file_path)?;
            let len = file.metadata()?.len();
            self.res.set_body(Some(ResBody::Stream(file, len)));
        } else {
            let content = fs::read(file_path)?;
            self.res.set_body(Some(ResBody::Bytes(content)));
        }
        Ok(())
    }

    pub fn build_error(&mut self, status_code: u16) -> &mut HttpRes {
        self.res.set_status(status_code);

        // set content type
        self.set_default_content_type();

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
        self.res
            .set_body(Some(ResBody::Bytes(message.into_bytes())));
        self.do_build()
    }

    pub fn do_build(&mut self) -> &mut HttpRes {
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
        if let Some(body) = self.res.body_ref()
            && body.len() > 0
        {
            self.res.set_header(
                ResHeader::EntityHeader(EntityHeader::ContentLength),
                HeaderValue::Number(body.len() as u64),
            )
        }

        &mut self.res
    }
}
