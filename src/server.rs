use crate::http_req::{HttpReq, ReqTarget};
use crate::req_parser::ReqHeadParser;

use crate::http_res::HttpRes;
use crate::res_builder::ResBuilder;
use chrono::Utc;
use log::{debug, error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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
        let mut invalid_request = false;

        while !connection_closed {
            req_head_parser.reset();
            let mut buf = String::new();

            debug!("waiting for request head");
            while !req_head_parser.is_complete() {
                buf.clear();

                // read one line from the stream
                let result = buf_reader.read_line(&mut buf);
                // handle connection closing
                if let Err(e) = result {
                    warn!("Cannot read line from buffered stream: {:?}", e);
                    connection_closed = true;
                    break;
                };
                if buf.is_empty() {
                    connection_closed = true;
                    break;
                }

                debug!("Received line: {:?}", buf);

                // parse line as part of HTTP request head
                if let Err(e) = req_head_parser.process_line(buf.trim()) {
                    warn!("Cannot process line: {:?}", e);
                    invalid_request = true;
                    break;
                }
            }
            if connection_closed {
                break;
            }
            if invalid_request {
                self.serve_error(400);
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
                }
                Err(e) => {
                    warn!("Error parsing request: {:?}", e);
                    self.serve_error(400)
                }
            }
        }

        info!("Connection closed: {}", self.peer_addr);
    }

    fn serve_error(&mut self, status_code: u16) {
        let mut res_builder = ResBuilder::new("1.1");
        let res = res_builder.build_error(status_code);
        self.send_response(res);
    }

    fn serve_req(&mut self) {
        debug!("serving request");

        match self.current_req.as_ref().unwrap().verb() {
            "GET" => self.serve_static_resource(),
            _ => self.serve_error(405),
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

    fn send_response(&mut self, res: &HttpRes) {
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

        let (res_head, res_body) = res.to_bytes();
        if let Err(e) = self.stream.write_all(&res_head) {
            warn!("Cannot write response head: {:?}", e);
        }
        if let Some(body) = res_body {
            #[allow(clippy::collapsible_if)]
            if let Err(e) = self.stream.write_all(body) {
                warn!("Cannot write response body: {:?}", e);
            }
        }
    }
}
