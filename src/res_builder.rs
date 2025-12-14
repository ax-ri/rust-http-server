use crate::http_header::{
    EntityHeader, GeneralHeader, HeaderValue, ResHeader, ResOnlyHeader, SimpleHeaderValue,
};
use crate::http_res::{self, HttpRes, ResBody};
use crate::req_parser::SupportedEncoding;

use crate::http_req::ReqVerb;
use crate::res_builder::ResBuildingError::PhpError;
use log::debug;
use std::{cmp, fmt, fs, io, net, path, process, str::FromStr};

pub struct ResBuilder {
    res: HttpRes,
}

#[derive(Debug)]
pub enum ResBuildingError {
    PhpError(String),
}

impl fmt::Display for ResBuildingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhpError(e) => write!(f, "PHP error: {}", e),
        }
    }
}

pub struct PhpScriptParams<'a> {
    pub script_path: &'a str,
    pub script_query: &'a str,
    pub used_credentials: Option<(String, String)>,
    pub client_ip: &'a str,
    pub verb: &'a ReqVerb,
    pub address: &'a net::SocketAddr,
    pub version: &'a str,
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
    ) -> Result<(), io::Error> {
        self.set_default_content_type();

        let mut entries = if rel_path == "/" {
            Vec::new()
        } else {
            vec![(String::from(".."), true)]
        };
        for e in fs::read_dir(dir_path)? {
            let e = e
                .as_ref()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, ""))?;
            entries.push((
                e.file_name()
                    .into_string()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidFilename, ""))?,
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

    pub async fn set_file_body(
        &mut self,
        mut file_path: &path::Path,
        encoding: Option<&SupportedEncoding>,
    ) -> Result<(), io::Error> {
        // set content type
        let mime_type = mime_guess::MimeGuess::from_path(file_path).first_or_octet_stream();
        self.res.set_header(
            ResHeader::EntityHeader(EntityHeader::ContentType),
            HeaderValue::Simple(SimpleHeaderValue::Mime(mime_type.clone())),
        );

        let mut real_file_path = None;
        if let Some(encoding) = encoding {
            // set encoding if file is not already a compressed format
            if mime_type.type_() == mime_guess::mime::TEXT {
                debug!("using compression");

                // create a temporary file to store the compressed version
                let tmp_file = async_tempfile::TempFile::new()
                    .await
                    .map_err(io::Error::other)?;
                // store the temp path for later use
                real_file_path = Some(tmp_file.try_clone().await.map_err(io::Error::other)?);

                // copy the real file to the compression encoder
                let mut file = tokio::fs::File::open(file_path).await?;
                use async_compression::tokio::write;
                macro_rules! copy_with {
                    ($e: path) => {{
                        let mut encoder = $e(tmp_file);
                        tokio::io::copy(&mut file, &mut encoder).await?;
                    }};
                }
                match encoding {
                    SupportedEncoding::Gzip => copy_with!(write::GzipEncoder::new),
                    SupportedEncoding::Deflate => copy_with!(write::DeflateEncoder::new),
                    SupportedEncoding::Zstd => copy_with!(write::ZstdEncoder::new),
                    SupportedEncoding::Br => copy_with!(write::BrotliEncoder::new),
                };

                // set the used encoding in the response header
                self.res.set_header(
                    ResHeader::EntityHeader(EntityHeader::ContentEncoding),
                    HeaderValue::Simple(SimpleHeaderValue::Plain(String::from(encoding))),
                );

                let metadata = fs::metadata(file_path)?;
                debug!("real file length: {} bytes", metadata.len());
            }
        }
        // if some compression was done, use the temporary file path instead
        // to serve the compressed content
        if let Some(tmp_file) = real_file_path.as_ref() {
            file_path = tmp_file.file_path().as_path();
        }

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

    pub async fn run_php_script(
        &mut self,
        params: PhpScriptParams<'_>,
    ) -> Result<(), ResBuildingError> {
        let output = process::Command::new("php-cgi")
            .env_clear()
            .env(
                "AUTH_TYPE",
                if params.used_credentials.is_some() {
                    "Basic"
                } else {
                    ""
                },
            )
            .env("CONTENT_LENGTH", "")
            .env("CONTENT_TYPE", "")
            .env("GATEWAY_INTERFACE", "1.1")
            .env("PATH_INFO", "")
            .env("PATH_TRANSLATED", "")
            .env("QUERY_STRING", params.script_query)
            .env("REMOTE_ADDR", params.client_ip)
            .env("REMOTE_HOST", params.client_ip)
            .env("REMOTE_IDENT", "")
            .env(
                "REMOTE_USER",
                if let Some((username, _)) = params.used_credentials {
                    username
                } else {
                    String::new()
                },
            )
            .env("REQUEST_METHOD", format!("{}", params.verb))
            .env("SCRIPT_NAME", "")
            .env(
                "SCRIPT_PATH",
                path::PathBuf::from(params.script_path)
                    .file_name()
                    .ok_or(PhpError(String::from("Invalid script path")))?,
            )
            .env("SCRIPT_FILENAME", params.script_path)
            .env("SERVER_NAME", params.address.ip().to_string())
            .env("SERVER_PORT", params.address.port().to_string())
            .env("SERVER_PROTOCOL", params.version)
            .env("SERVER_SOFTWARE", "rust-http-server")
            .env("REDIRECT_STATUS", "200")
            .output()
            .map_err(|e| PhpError(e.to_string()))?;

        if !output.status.success() {
            return Err(PhpError(format!(
                "php-cgi error ({}): {}",
                output.status,
                String::from_utf8(output.stderr).ok().unwrap_or_default()
            )));
        }

        let output = String::from_utf8(output.stdout).map_err(|e| PhpError(e.to_string()))?;
        let mut split = output.splitn(2, "\r\n\r\n");
        let (headers, body) = (
            split
                .next()
                .ok_or(PhpError(String::from("Invalid script output")))?,
            split
                .next()
                .ok_or(PhpError(String::from("Invalid script output")))?,
        );
        // let header_names = headers
        //     .split("\r\n")
        //     .filter_map(|h| h.split(':').next())
        //     .map(str::to_lowercase)
        //     .collect::<collections::HashSet<_>>();

        self.res.set_raw_headers(String::from(headers) + "\r\n");
        self.res
            .set_body(Some(ResBody::Bytes(Vec::from(body.as_bytes()))));

        Ok(())
    }

    pub fn build_error(&mut self, status_code: u16, with_body: bool) -> &mut HttpRes {
        self.res.set_status(status_code);

        // 401 unauthorized error: add authentication request header
        if status_code == 401 {
            self.res.set_header(
                ResHeader::ResOnlyHeader(ResOnlyHeader::WWWAuthenticate),
                HeaderValue::Simple(SimpleHeaderValue::Plain(String::from(
                    "Basic realm=\"simple\"",
                ))),
            )
        }

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
