use crate::http_req::{HttpReq, ReqTarget, ReqVerb};
use crate::req_parser::{ReqHeadParser, ReqHeadParsingError};
use std::io::Read;

use crate::http_res::{HttpRes, ResBody};
use crate::res_builder::ResBuilder;
use ascii::AsciiString;
use chrono::Utc;
use log::{debug, error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Settings {
    pub address: SocketAddr,
    pub document_root: PathBuf,
    pub allow_dir_listing: bool,
}

pub struct Server {
    listener: TcpListener,
    settings: Settings,
}

impl Server {
    pub fn new(settings: Settings) -> Result<Self, std::io::Error> {
        TcpListener::bind(settings.address).map(|listener| Self { listener, settings })
    }

    pub fn listen(&mut self) {
        // accept connections and process them serially
        loop {
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    let mut handler = ClientHandler::new(&self.settings, stream);
                    handler.handle();
                }
                Err(err) => error!("Cannot accept TCP connection, {:?}", err),
            }
        }
    }
}

struct ClientHandler<'a> {
    settings: &'a Settings,
    stream: TcpStream,
    peer_addr: String,
    current_req: Option<HttpReq>,
}

impl<'a> ClientHandler<'a> {
    fn new(settings: &'a Settings, stream: TcpStream) -> Self {
        let peer_addr = stream.peer_addr().unwrap().to_string();
        Self {
            settings,
            stream,
            peer_addr,
            current_req: None,
        }
    }

    fn handle(&mut self) {
        info!("Connection received from: {}", self.peer_addr);

        // use a buffered reader to read the stream one line at a time
        let mut buf_reader = BufReader::new(self.stream.try_clone().expect("Cannot clone stream"));

        let mut req_head_parser = ReqHeadParser::new();

        let mut connection_closed = false;
        let mut req_parsing_error = None;

        while !connection_closed {
            req_head_parser.reset();

            debug!("waiting for request head");
            while !req_head_parser.is_complete() {
                // read one line from the stream
                // with a maximum limit on bytes read (8000)
                let mut line: Vec<u8> = Vec::new();
                let mut handle = buf_reader.take(8000);
                let result = handle.read_until(b'\n', &mut line);
                buf_reader = handle.into_inner();

                // handle connection closing
                if let Err(e) = result {
                    warn!("Cannot read line from buffered stream: {:?}", e);
                    connection_closed = true;
                    break;
                };
                if line.is_empty() {
                    connection_closed = true;
                    break;
                }
                let ascii_line =
                    AsciiString::from_ascii(line).expect("Cannot convert to ascii string");
                debug!("Received line: {:?}", ascii_line);

                // parse line as part of HTTP request head
                if let Err(e) = req_head_parser.process_line(ascii_line.trim()) {
                    req_parsing_error = Some(e);
                    break;
                }
            }
            if connection_closed {
                break;
            }
            if let Some(e) = req_parsing_error.as_ref() {
                self.handle_req_parsing_error(e);
                continue;
            }

            debug!("done reading request head");
            debug!("parsing request head");
            match req_head_parser.do_parse() {
                Ok(parsed_head) => {
                    debug!("request head parsing done");
                    dbg!("REQUEST HEAD:{:?}", &parsed_head);

                    self.current_req = Some(HttpReq::new(Utc::now(), parsed_head));
                    self.serve_req();
                    if self.current_req.as_ref().unwrap().should_close() {
                        if let Err(e) = self.stream.shutdown(Shutdown::Both) {
                            warn!("Cannot close connection, {:?}", e);
                        };
                        info!("Closing connection");
                        connection_closed = true;
                    }
                }
                Err(e) => self.handle_req_parsing_error(&e),
            }
        }

        info!("Connection closed: {}", self.peer_addr);
    }

    fn handle_req_parsing_error(&mut self, error: &ReqHeadParsingError) {
        match error {
            ReqHeadParsingError::InvalidFirstLine(error) => {
                warn!("Error parsing request first line: {:?}", error)
            }
            ReqHeadParsingError::InvalidHeader(error) => {
                warn!("Error parsing request header: {:?}", error)
            }
        };
        self.serve_error(400);
    }

    fn serve_error(&mut self, status_code: u16) {
        let mut res_builder = ResBuilder::new("HTTP/1.1");
        let res = res_builder.build_error(status_code);
        self.send_response(res);
    }

    fn serve_req(&mut self) {
        debug!("serving request");

        match self.current_req.as_ref().unwrap().verb() {
            ReqVerb::Get => self.serve_static_resource(),
            //_ => self.serve_error(405),
        };

        debug!("request served");
    }

    fn serve_static_resource(&mut self) {
        let req = self.current_req.as_ref().unwrap();
        let mut res_builder = ResBuilder::new(req.version());
        match req.target() {
            // target '*' not supported for get resource
            ReqTarget::All => self.serve_error(400),
            // serve target from path
            ReqTarget::Path(path, _) => {
                // convert target resource path to ile system path
                let mut full_path = String::from(self.settings.document_root.to_str().unwrap());
                full_path.push_str(path);
                let full_path = Path::new(&full_path);

                // prevent path traversal: the resource path must be a sub-path of the doc root
                if !full_path.starts_with(self.settings.document_root.as_path()) {
                    self.serve_error(403);
                    return;
                }

                // prevent directory listing by default
                if full_path.is_dir() {
                    if self.settings.allow_dir_listing {
                        match res_builder.list_directory(full_path, path) {
                            Ok(()) => {
                                let res = res_builder.do_build();
                                self.send_response(res)
                            }
                            Err(err) => {
                                debug!("error reading directory: {:?}", err);
                                match err.kind() {
                                    std::io::ErrorKind::NotFound => self.serve_error(404),
                                    std::io::ErrorKind::PermissionDenied => self.serve_error(403),
                                    _ => self.serve_error(500),
                                }
                            }
                        }
                    } else {
                        self.serve_error(403);
                    }
                    return;
                }

                match res_builder.set_file_body(full_path) {
                    Ok(()) => {
                        let res = res_builder.do_build();
                        self.send_response(res)
                    }
                    Err(err) => {
                        debug!("error reading file: {:?}", err);
                        match err.kind() {
                            std::io::ErrorKind::NotFound => self.serve_error(404),
                            std::io::ErrorKind::PermissionDenied => self.serve_error(403),
                            _ => self.serve_error(500),
                        }
                    }
                }
            }
        }
    }

    fn send_response(&mut self, res: &mut HttpRes) {
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

        let res_head = res.head_bytes();
        if let Err(e) = self.stream.write_all(&res_head) {
            warn!("Cannot write response head: {:?}", e);
        }

        if let Err(e) = self.stream.flush() {
            warn!("Cannot flush response head: {:?}", e)
        }

        match res.body_ref() {
            Some(ResBody::Bytes(bytes)) => {
                debug!("sending {} bytes", bytes.len());
                if let Err(e) = self.stream.write_all(bytes) {
                    warn!("Cannot write response body bytes: {:?}", e);
                }
            }
            Some(ResBody::Stream(file, _)) => {
                let mut file = file;
                match std::io::copy(&mut file, &mut self.stream) {
                    Ok(n) => debug!("sent {} bytes", n),
                    Err(e) => {
                        warn!("Cannot write response body stream: {:?}", e);
                    }
                }
            }
            None => (),
        }
        if let Err(e) = self.stream.flush() {
            warn!("Cannot flush response body: {:?}", e)
        }
    }
}
