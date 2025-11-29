mod http_header;
mod http_req;
mod http_res;
mod req_parser;
mod res_builder;
mod server;
mod utils;

use crate::server::Settings;
use argparse_rs::{ArgParser, ArgType};
use log::{debug, info};
use server::Server;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

fn parse_args() -> Result<Settings, String> {
    let mut arg_parser = ArgParser::new(String::from("rust-http-server"));
    arg_parser.add_opt(
        "address",
        None,
        'a',
        true,
        "Socket address",
        ArgType::Option,
    );
    arg_parser.add_opt(
        "doc-root",
        None,
        'r',
        true,
        "Directory root to serve resources from",
        ArgType::Option,
    );
    arg_parser.add_opt(
        "dir-listing",
        Some("false"),
        'd',
        false,
        "Allow directory listing",
        ArgType::Flag,
    );
    let args = arg_parser.parse(env::args().collect::<Vec<String>>().iter())?;

    Ok(Settings {
        address: args
            .get::<SocketAddr>("address")
            .ok_or("invalid socket address")?,
        document_root: args
            .get::<PathBuf>("doc-root")
            .ok_or("invalid document root")?,
        allow_dir_listing: args
            .get::<bool>("dir-listing")
            .ok_or("invalid value for directory listing")?,
    })
}

fn main() -> Result<(), String> {
    // set default log level to info
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info") }
    }
    env_logger::init();

    let server_settings = parse_args()?;
    debug!("server settings: {:?}", server_settings);
    info!("Starting server");
    let mut server = Server::new(server_settings).map_err(|e| e.to_string())?;
    server.listen();
    Ok(())
}
