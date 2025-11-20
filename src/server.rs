use crate::req_parser::HttpReqHeadParser;
use log::{debug, error, info};
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        match TcpListener::bind(addr) {
            Ok(listener) => Ok(Self { listener }),
            Err(err) => Err(err),
        }
    }

    pub fn listen(&mut self) {
        // accept connections and process them serially
        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => self.handle_client(&mut stream),
                Err(err) => error!("Cannot accept TCP connection, {:?}", err),
            }
        }
    }

    fn handle_client(&mut self, stream: &mut TcpStream) {
        info!("Connection received from: {}", stream.peer_addr().unwrap());

        let mut buffered_stream = BufReader::new(&mut *stream);
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
            debug!("done reading request head");
            debug!("parsing request head");
            let parsed_head = req_head_parser
                .do_parse()
                .expect("Cannot parse request head");
            debug!("request head parsing done");
            println!("REQUEST HEAD:{:?}", parsed_head);
        }

        info!("Connection closed: {}", stream.peer_addr().unwrap());
    }
}
