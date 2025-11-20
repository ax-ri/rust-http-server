use crate::http_req::{HttpReq, ReqTarget};
use crate::req_parser::ReqHeadParser;

use crate::http_res::HttpRes;
use crate::res_builder::ResBuilder;
use log::{debug, error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        TcpListener::bind(addr).map(|listener| Self { listener })
    }

    pub fn listen(&mut self) {
        // accept connections and process them serially
        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => handle_client(&mut stream),
                Err(err) => error!("Cannot accept TCP connection, {:?}", err),
            }
        }
    }
}

fn handle_client(stream: &mut TcpStream) {
    let peer_addr = stream.peer_addr().unwrap();
    info!("Connection received from: {}", peer_addr);

    // use a buffered reader to read the stream one line at a time
    let mut buffered_stream = BufReader::new(stream.try_clone().expect("Cannot clone stream"));

    let mut req_head_parser = ReqHeadParser::new();

    let mut connection_closed = false;
    let mut invalid_request = false;

    while !connection_closed {
        req_head_parser.reset();
        let mut buf = String::new();

        debug!("waiting for request head");
        while !req_head_parser.is_complete() {
            buf.clear();
            if let Err(e) = buffered_stream.read_line(&mut buf) {
                warn!("Cannot read line from buffered stream: {:?}", e);
                connection_closed = true;
                break;
            };

            if buf.is_empty() {
                connection_closed = true;
                break;
            }
            debug!("Received line: {:?}", buf);
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
            serve_error(stream, 400);
            continue;
        }
        debug!("done reading request head");
        debug!("parsing request head");
        match req_head_parser.do_parse() {
            Ok(parsed_head) => {
                debug!("request head parsing done");
                debug!("REQUEST HEAD:{:?}", parsed_head);

                let req = HttpReq::new(parsed_head);
                serve_req(req, &mut *stream);
            }
            Err(e) => {
                warn!("Error parsing request: {:?}", e);
                serve_error(stream, 400)
            }
        }
    }

    info!("Connection closed: {}", peer_addr);
}

fn serve_error(stream: &mut TcpStream, status_code: u16) {
    let mut res_builder = ResBuilder::new("1.1");
    send_response(stream, res_builder.build_error(status_code));
}

fn serve_req(req: HttpReq, stream: &mut TcpStream) {
    debug!("serving request");

    let doc_root = String::from("./htdocs");

    let mut res_builder = ResBuilder::new(req.version());
    let res = match req.verb() {
        "GET" => match req.target() {
            ReqTarget::Path(p) => {
                let full_path = doc_root + p;
                match res_builder.set_file_body(Path::new(&full_path)) {
                    Ok(()) => res_builder.do_build(),
                    Err(e) => {
                        debug!("error reading file: {:?}", e);
                        res_builder.build_error(404)
                    }
                }
            }
            _ => res_builder.build_error(400),
        },
        _ => res_builder.build_error(405),
    };

    send_response(stream, res);

    debug!("request served");
}

fn send_response(stream: &mut TcpStream, res: &HttpRes) {
    let (res_head, res_body) = res.to_bytes();
    stream
        .write_all(&res_head)
        .expect("Cannot write response head");
    if let Some(body) = res_body {
        stream.write_all(body).expect("Cannot write response body");
    }
}
