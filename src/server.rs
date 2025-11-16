use crate::req_parser::ReqParser;
use log::{debug, error, info};
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

pub struct Server {
    listener: TcpListener,
    req_parser: ReqParser,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        match TcpListener::bind(addr) {
            Ok(listener) => Ok(Self {
                listener,
                req_parser: ReqParser::new(),
            }),
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

        let mut closed = false;
        while !closed {
            self.req_parser.reset();
            let mut buf = String::new();
            let mut is_done = false;

            debug!("read start");
            while !is_done {
                buf.clear();
                buffered_stream.read_line(&mut buf).unwrap();
                if buf.is_empty() {
                    closed = true;
                    break;
                }
                debug!("Received line: {:?}", buf);
                is_done = self.req_parser.parse_line(&buf);
            }
            debug!("read end");
        }

        info!("Connection closed: {}", stream.peer_addr().unwrap());
    }
}
