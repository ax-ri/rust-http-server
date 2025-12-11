use crate::http_header::{
    EntityHeader, GeneralHeader, HeaderValue, ResHeader, ResOnlyHeader, SimpleHeaderValue,
};
use crate::http_res::{self, HttpRes, ResBody};

use std::{cmp, fs, path, str::FromStr};

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
            HeaderValue::Simple(SimpleHeaderValue::Mime(
                mime_guess::Mime::from_str("text/html; charset=utf-8").unwrap(),
            )),
        );
    }

    pub fn list_directory(
        &mut self,
        dir_path: &path::Path,
        rel_path: &str,
    ) -> Result<(), std::io::Error> {
        self.set_default_content_type();

        let mut entries = if rel_path == "/" {
            Vec::new()
        } else {
            vec![(String::from(".."), true)]
        };
        for e in fs::read_dir(dir_path)? {
            let e = e
                .as_ref()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, ""))?;
            entries.push((
                e.file_name()
                    .into_string()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidFilename, ""))?,
                e.metadata()?.is_dir(),
            ))
        }

        entries.sort_by(
            |(a_name, a_is_dir), (b_name, b_is_dir)| match (a_is_dir, b_is_dir) {
                (true, false) => cmp::Ordering::Less,
                (false, true) => cmp::Ordering::Greater,
                _ => a_name.cmp(b_name),
            },
        );
        let title = format!("Index of {}", rel_path);
        let html = format!(
            "<!DOCTYPE HTML> \
             <html lang=\"en\"> \
                <head> \
                    <meta charset=\"utf-8\"/> \
                    <title>{}</title> \
                </head> \
                <body> \
                    <h1>{}</h1> \
                    <hr/> \
                        <ul>{}</ul> \
                    <hr/> \
                </body> \
             </html> \
             \r\n",
            title,
            title,
            entries
                .iter()
                .map(|(file_name, is_dir)| {
                    let sep = if rel_path == "/" { "" } else { "/" };
                    format!(
                        r##"<li><pre><a href="{}">{}{}</a></pre></li>"##,
                        rel_path.trim_end_matches("/").to_owned() + sep + file_name,
                        file_name,
                        if *is_dir { "/" } else { "" },
                    )
                })
                .fold(String::new(), |acc, e| format!("{}{}", acc, e)),
        );

        // set content
        let content = Vec::from(html.as_bytes());
        self.res.set_body(Some(ResBody::Bytes(content)));

        Ok(())
    }

    pub async fn set_file_body(&mut self, file_path: &path::Path) -> Result<(), std::io::Error> {
        // set content type
        let mime_type = mime_guess::MimeGuess::from_path(file_path).first_or_octet_stream();
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Simple(SimpleHeaderValue::Mime(mime_type)),
        );

        let metadata = fs::metadata(file_path)?;
        // set content
        if metadata.len() > 1024 * 1024 {
            let file = tokio::fs::File::open(file_path).await?;
            let len = file.metadata().await?.len();
            self.res.set_body(Some(ResBody::Stream(file, len)));
        } else {
            let content = fs::read(file_path)?;
            self.res.set_body(Some(ResBody::Bytes(content)));
        }
        Ok(())
    }

    pub fn build_error(&mut self, status_code: u16, with_body: bool) -> &mut HttpRes {
        self.res.set_status(status_code);

        // set content type
        if with_body {
            self.set_default_content_type();

            let title = format!(
                "{} {}",
                status_code,
                http_res::get_reason_phrase(status_code)
            );
            let message = format!(
                "<!DOCTYPE html> \
                 <html lang=\"en\"> \
                    <head> \
                        <meta charset=\"utf-8\"/> \
                        <title>{}</title> \
                    </head> \
                    <body> \
                        <h1>{}</h1> \
                    </body> \
                 </html> \
                 \r\n",
                title, title
            );
            self.res
                .set_body(Some(ResBody::Bytes(message.into_bytes())));
        } else {
            self.res.set_header(
                ResHeader::EntityHeader(EntityHeader::ContentLength),
                HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("0"))),
            );
        }
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
                HeaderValue::Simple(SimpleHeaderValue::Plain(
                    chrono::Utc::now()
                        .format("%a, %d %b %Y %H:%M:%S GMT")
                        .to_string(),
                )),
            );
        }

        // set server origin if not already present
        if !self
            .res
            .has_header(ResHeader::ResOnlyHeader(ResOnlyHeader::Server))
        {
            self.res.set_header(
                ResHeader::ResOnlyHeader(ResOnlyHeader::Server),
                HeaderValue::Simple(SimpleHeaderValue::Plain(String::from("rust-http-server"))),
            );
        }

        // set content-length
        if let Some(body) = self.res.body_ref()
            && !body.is_empty()
        {
            self.res.set_header(
                ResHeader::EntityHeader(EntityHeader::ContentLength),
                HeaderValue::Simple(SimpleHeaderValue::Number(body.len() as u64)),
            )
        }

        &mut self.res
    }
}
