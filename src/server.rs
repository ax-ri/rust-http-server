use crate::http_req::HttpReq;
use crate::req_parser::HttpReqHeadParser;

use crate::res_builder::ResBuilder;
use log::{debug, error, info};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

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
    info!("Connection received from: {}", stream.peer_addr().unwrap());

    // use a buffered reader to read the stream one line at a time
    let mut buffered_stream = BufReader::new(stream.try_clone().expect("Cannot clone stream"));

    let mut req_head_parser = HttpReqHeadParser::new();

    let mut connection_closed = false;
    while !connection_closed {
        req_head_parser.reset();
        let mut buf = String::new();

        debug!("waiting for request head");
        while !req_head_parser.is_complete() {
            buf.clear();
            buffered_stream
                .read_line(&mut buf)
                .expect("Cannot read line from buffered stream");
            if buf.is_empty() {
                connection_closed = true;
                break;
            }
            debug!("Received line: {:?}", buf);
            req_head_parser
                .process_line(buf.trim())
                .expect("Cannot process line");
        }
        if connection_closed {
            break;
        }
        debug!("done reading request head");
        debug!("parsing request head");
        let parsed_head = req_head_parser
            .do_parse()
            .expect("Cannot parse request head");
        debug!("request head parsing done");
        debug!("REQUEST HEAD:{:?}", parsed_head);

        let req = HttpReq::new(parsed_head);
        serve_req(req, &mut *stream);
    }

    info!("Connection closed: {}", stream.peer_addr().unwrap());
}

fn serve_req(req: HttpReq, stream: &mut TcpStream) {
    debug!("handling request");

    let mut res_builder = ResBuilder::new(&req);
    let res = res_builder.do_build().expect("Cannot build response");

    println!("RESPONSE:\n{:?}", res.to_string());
    stream
        .write_all(res.to_string().as_bytes())
        .expect("Cannot write response");
}
