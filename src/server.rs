use crate::http_header::{
    EntityHeader, HeaderValue, ReqHeader, ReqOnlyHeader, ResHeader, SimpleHeaderValue,
};
use crate::http_req::{HttpReq, ReqTarget, ReqVerb};
use crate::http_res::{HttpRes, ResBody};
use crate::req_parser::{ReqHeadParser, ReqHeadParsingError};
use crate::res_builder::ResBuilder;
use crate::utils;

use std::{collections, fmt, io, net, path, sync};

use log::{debug, error, info, warn};
use rustls::pki_types::pem::PemObject;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Settings {
    pub address: net::SocketAddr,
    pub document_root: path::PathBuf,
    pub allow_dir_listing: bool,
    pub ssl_cert_path: Option<path::PathBuf>,
    pub ssl_key_path: Option<path::PathBuf>,
}

pub struct Server {
    listener: tokio::net::TcpListener,
    settings: Settings,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
}

pub enum Error {
    Io(io::Error),
    Tls(rustls::Error),
    TlsPem(rustls::pki_types::pem::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Tls(e) => write!(f, "TLS error: {}", e),
            Self::TlsPem(e) => write!(f, "TLS Pem error: {}", e),
        }
    }
}

impl Server {
    pub async fn new(settings: Settings) -> Result<Self, Error> {
        let listener = tokio::net::TcpListener::bind(settings.address)
            .await
            .map_err(Error::Io)?;
        if let Some(cert_path) = settings.ssl_cert_path.as_ref()
            && let Some(key_path) = settings.ssl_key_path.as_ref()
        {
            let certs = rustls::pki_types::CertificateDer::pem_file_iter(cert_path)
                .map_err(Error::TlsPem)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::TlsPem)?;
            let key =
                rustls::pki_types::PrivateKeyDer::from_pem_file(key_path).map_err(Error::TlsPem)?;
            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(Error::Tls)?;
            Ok(Self {
                listener,
                settings,
                tls_acceptor: Some(tokio_rustls::TlsAcceptor::from(sync::Arc::new(config))),
            })
        } else {
            Ok(Self {
                listener,
                settings,
                tls_acceptor: None,
            })
        }
    }

    pub async fn listen(&mut self) {
        // accept connections and process them concurrently
        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let settings = self.settings.clone();

                    if let Some(acceptor) = self.tls_acceptor.as_ref() {
                        match acceptor.accept(stream).await {
                            Ok(stream) => {
                                tokio::spawn(async move {
                                    let mut handler =
                                        ClientHandler::new(settings, peer_addr.to_string(), stream);
                                    handler.handle().await;
                                });
                            }
                            Err(err) => {
                                warn!("Cannot accept TLS connection: {}", err);
                                continue;
                            }
                        }
                    } else {
                        tokio::spawn(async move {
                            let mut handler =
                                ClientHandler::new(settings, peer_addr.to_string(), stream);
                            handler.handle().await;
                        });
                    }
                }
                Err(err) => error!("Cannot accept TCP connection, {:?}", err),
            }
        }
    }
}

trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin {}
impl AsyncStream for tokio::net::TcpStream {}
impl AsyncStream for tokio_rustls::server::TlsStream<tokio::net::TcpStream> {}

struct ClientHandler<S: AsyncStream> {
    settings: Settings,
    stream: S,
    peer_addr: String,
    current_req: Option<HttpReq>,
}

impl<S: AsyncStream> ClientHandler<S> {
    fn new(settings: Settings, peer_addr: String, stream: S) -> Self {
        Self {
            settings,
            stream,
            peer_addr,
            current_req: None,
        }
    }

    async fn handle(&mut self) {
        info!("Connection received from: {}", self.peer_addr);

        let mut req_head_parser = ReqHeadParser::new();

        let mut connection_closed = false;
        let mut req_parsing_error = Ok(());

        while !connection_closed {
            req_head_parser.reset();

            // use a buffered reader to read the stream one line at a time
            let mut buf_reader = tokio::io::BufReader::new(&mut self.stream);

            debug!("waiting for request head");
            while !req_head_parser.is_complete() {
                // read one line from the stream
                // with a maximum limit on bytes read (8000)
                let mut line: Vec<u8> = Vec::new();
                let mut handle = buf_reader.take(8000);
                let result = handle.read_until(b'\n', &mut line).await;
                buf_reader = handle.into_inner();

                // handle connection closing
                if let Err(err) = result {
                    warn!("Cannot read line from buffered stream: {:?}", err);
                    connection_closed = true;
                    break;
                };
                if line.is_empty() {
                    connection_closed = true;
                    break;
                }
                // parse line as part of HTTP request head
                if let Err(err) = req_head_parser.process_bytes(line) {
                    req_parsing_error = Err(err);
                    break;
                }
            }
            if connection_closed {
                break;
            }
            if let Err(err) = req_parsing_error.as_ref() {
                self.handle_req_parsing_error(err).await;
                continue;
            }

            debug!("done reading request head");
            debug!("parsing request head");
            match req_head_parser.do_parse() {
                Ok(parsed_head) => {
                    debug!("request head parsing done");

                    self.current_req = Some(HttpReq::new(chrono::Utc::now(), parsed_head));
                    self.serve_req().await;
                    if self.current_req.as_ref().unwrap().should_close() {
                        if let Err(err) = self.stream.shutdown().await {
                            warn!("Cannot close connection, {:?}", err);
                        };
                        info!("Closing connection");
                        connection_closed = true;
                    }
                }
                Err(err) => self.handle_req_parsing_error(&err).await,
            }
        }

        info!("Connection closed: {}", self.peer_addr);
    }

    async fn handle_req_parsing_error(&mut self, error: &ReqHeadParsingError) {
        match error {
            ReqHeadParsingError::Ascii(error) => {
                warn!("Error parsing request as ASCII: {:?}", error)
            }
            ReqHeadParsingError::FirstLine(error) => {
                warn!("Error parsing request first line: {:?}", error)
            }
            ReqHeadParsingError::Header(error) => {
                warn!("Error parsing request header: {:?}", error)
            }
        };
        self.serve_error(400, true).await;
    }

    async fn serve_error(&mut self, status_code: u16, with_body: bool) {
        let mut res_builder = ResBuilder::new("HTTP/1.1");
        let res = res_builder.build_error(status_code, with_body);
        Box::pin(self.send_response(res)).await;
    }

    async fn serve_io_error(&mut self, err: &io::Error) {
        match err.kind() {
            io::ErrorKind::NotFound => self.serve_error(404, true).await,
            io::ErrorKind::PermissionDenied => self.serve_error(403, true).await,
            _ => self.serve_error(500, true).await,
        }
    }

    async fn serve_req(&mut self) {
        debug!("serving request");

        match self.current_req.as_ref().unwrap().verb() {
            ReqVerb::Get => self.serve_static_resource().await,
            //_ => self.serve_error(405),
        };

        debug!("request served");
    }

    async fn serve_static_resource(&mut self) {
        let req = self.current_req.as_ref().unwrap();
        let mut res_builder = ResBuilder::new(req.version());
        match req.target() {
            // target '*' not supported for get resource
            ReqTarget::All => self.serve_error(400, true).await,
            // serve target from path
            ReqTarget::Path(path, _) => {
                // convert target resource path to ile system path
                let mut full_path = String::from(self.settings.document_root.to_str().unwrap());
                full_path.push_str(path);
                let full_path = path::Path::new(&full_path);

                // prevent path traversal: the resource path must be a sub-path of the doc root
                if !full_path.starts_with(self.settings.document_root.as_path()) {
                    self.serve_error(403, true).await;
                    return;
                }

                // prevent directory listing by default
                if full_path.is_dir() {
                    if self.settings.allow_dir_listing {
                        match res_builder.list_directory(full_path, path) {
                            Ok(()) => {
                                let res = res_builder.do_build();
                                self.send_response(res).await
                            }
                            Err(err) => {
                                debug!("error reading directory: {:?}", err);
                                self.serve_io_error(&err).await;
                            }
                        }
                    } else {
                        self.serve_error(403, true).await;
                    }
                    return;
                }

                match res_builder.set_file_body(full_path).await {
                    Ok(()) => {
                        let res = res_builder.do_build();
                        self.send_response(res).await
                    }
                    Err(err) => {
                        debug!("error reading file: {:?}", err);
                        self.serve_io_error(&err).await;
                    }
                }
            }
        }
    }

    async fn send_response(&mut self, res: &mut HttpRes) {
        // check whether the response content-type is accepted by the sender
        if let Some(req) = self.current_req.as_mut()
            && let collections::hash_map::Entry::Occupied(accepted) = req
                .headers()
                .entry(ReqHeader::ReqOnly(ReqOnlyHeader::Accept))
            && let collections::hash_map::Entry::Occupied(actual) = res
                .headers()
                .entry(ResHeader::EntityHeader(EntityHeader::ContentType))
            && let HeaderValue::Simple(SimpleHeaderValue::Mime(actual)) = actual.get()
        {
            match accepted.get() {
                HeaderValue::Simple(SimpleHeaderValue::Mime(accepted)) => {
                    dbg!(&actual);
                    if !utils::are_mime_compatible(accepted, actual) {
                        self.serve_error(415, false).await;
                        return;
                    }
                }
                HeaderValue::Parsed(v) => {
                    if v.0.iter().all(|(v, _)| match v {
                        SimpleHeaderValue::Mime(accepted) => {
                            !utils::are_mime_compatible(accepted, actual)
                        }
                        _ => true,
                    }) {
                        self.serve_error(415, false).await;
                        return;
                    }
                }
                _ => (),
            }
        }

        // log request and response
        info!(
            "{} - - {} {} {} {}",
            self.peer_addr,
            self.current_req.as_ref().map_or(String::from("-"), |r| r
                .date()
                .format("[%d/%b/%Y:%H:%M:%S %z]")
                .to_string()),
            self.current_req
                .as_ref()
                .map_or(String::from("-"), |r| format!(r#""{}""#, r.first_line())),
            res.status_code(),
            res.body_len()
        );

        // write response head to socket
        let res_head = res.head_bytes();
        if let Err(err) = self.stream.write_all(&res_head).await {
            warn!("Cannot write response head: {:?}", err);
        }
        if let Err(err) = self.stream.flush().await {
            warn!("Cannot flush response head: {:?}", err)
        }

        // write response body (if any) to socket
        match res.body_mut() {
            Some(ResBody::Bytes(bytes)) => {
                debug!("sending {} bytes", bytes.len());
                if let Err(err) = self.stream.write_all(bytes).await {
                    warn!("Cannot write response body bytes: {:?}", err);
                }
            }
            Some(ResBody::Stream(file, _)) => match tokio::io::copy(file, &mut self.stream).await {
                Ok(n) => debug!("sent {} bytes", n),
                Err(err) => {
                    warn!("Cannot write response body stream: {:?}", err);
                }
            },
            None => (),
        }
        if let Err(err) = self.stream.flush().await {
            warn!("Cannot flush response body: {:?}", err)
        }
    }
}
