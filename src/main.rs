mod req_parser;
mod server;

use log::info;
use server::Server;

fn main() {
    env_logger::init();

    info!("Starting server");
    let mut server = Server::new("127.0.0.1:8080").unwrap();
    server.listen()
}
