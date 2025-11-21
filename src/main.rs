mod http_header;
mod http_req;
mod http_res;
mod req_parser;
mod res_builder;
mod server;

use crate::server::Settings;
use clap::Parser;
use log::{debug, info};
use server::Server;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'a', long, help = "socket address")]
    address: SocketAddr,
    #[arg(short = 'r', long = "root", help = "directory to serve resources from")]
    document_root: PathBuf,
    #[arg(short = 'd', long, default_value = "false")]
    allow_dir_listing: bool,
}

impl From<Args> for Settings {
    fn from(value: Args) -> Self {
        Self {
            address: value.address,
            document_root: value.document_root,
            allow_dir_listing: value.allow_dir_listing,
        }
    }
}

fn main() {
    // set default log level to info
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info") }
    }
    env_logger::init();
    let args = Args::parse();
    let server_settings = args.into();
    debug!("server settings: {:?}", server_settings);
    info!("Starting server");
    let mut server = Server::new(server_settings).unwrap();
    server.listen()
}
