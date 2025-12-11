#![feature(coverage_attribute)]

use rust_http_server::server::{Server, Settings};

use std::{env, net, path};

use log::{debug, info};

#[cfg_attr(coverage, coverage(off))]
fn parse_args() -> Result<Settings, String> {
    let mut arg_parser = argparse_rs::ArgParser::new(String::from("rust-http-server"));
    arg_parser.add_opt(
        "address",
        None,
        'a',
        true,
        "Socket address",
        argparse_rs::ArgType::Option,
    );
    arg_parser.add_opt(
        "doc-root",
        None,
        'r',
        true,
        "Directory root to serve resources from",
        argparse_rs::ArgType::Option,
    );
    arg_parser.add_opt(
        "dir-listing",
        Some("false"),
        'd',
        false,
        "Allow directory listing",
        argparse_rs::ArgType::Flag,
    );
    arg_parser.add_opt(
        "ssl-cert",
        None,
        'c',
        false,
        "SSL certificate for HTTPS",
        argparse_rs::ArgType::Option,
    );
    arg_parser.add_opt(
        "ssl-key",
        None,
        'k',
        false,
        "SSL key for HTTPS",
        argparse_rs::ArgType::Option,
    );
    let args = arg_parser.parse(env::args().collect::<Vec<String>>().iter())?;

    Ok(Settings {
        address: args
            .get::<net::SocketAddr>("address")
            .ok_or("invalid socket address")?,
        document_root: args
            .get::<path::PathBuf>("doc-root")
            .ok_or("invalid doc-root")?
            .canonicalize()
            .map_err(|_| "cannot canonicalize doc-root")?,
        allow_dir_listing: args
            .get::<bool>("dir-listing")
            .ok_or("invalid value for directory listing")?,
        ssl_cert_path: args.get::<path::PathBuf>("ssl-cert"),
        ssl_key_path: args.get::<path::PathBuf>("ssl-key"),
    })
}

#[tokio::main]
#[cfg_attr(coverage, coverage(off))]
async fn main() -> Result<(), String> {
    // set default log level to info
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info") }
    }
    env_logger::init();

    let server_settings = parse_args()?;
    debug!("server settings: {:?}", server_settings);
    info!("Starting server");
    let mut server = Server::new(server_settings)
        .await
        .map_err(|e| e.to_string())?;
    server.listen().await;
    Ok(())
}
