#![cfg_attr(coverage, feature(coverage_attribute))]

use rust_http_server::server::{Server, Settings};

use log::{debug, info};
use std::{env, io, net, path};
use termion::input::TermRead;

fn parse_authentication_credentials(
    argument: Option<String>,
) -> Result<Option<Vec<(String, String)>>, String> {
    if let Some(creds_list) = argument {
        let mut auth_creds = Vec::new();
        for creds in creds_list.split(",") {
            match *creds.splitn(2, ':').collect::<Vec<&str>>().as_slice() {
                [username, password] if !username.is_empty() && !password.is_empty() => {
                    auth_creds.push((String::from(username), String::from(password)))
                }
                _ => return Err(format!("Invalid credential tuple: {}", creds)),
            }
        }
        if auth_creds.is_empty() {
            return Err(String::from("No valid credentials provided"));
        }
        Ok(Some(auth_creds))
    } else {
        Ok(None)
    }
}

#[cfg_attr(coverage, coverage(off))]
fn parse_args() -> Result<Settings, String> {
    let mut arg_parser = argparse_rs::ArgParser::new(String::from("rust-http-server"));
    arg_parser.add_opt(
        "address",
        None,
        'a',
        true,
        "Socket address to bind",
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
        "Allow directory listing (default: false)",
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
    arg_parser.add_opt(
        "auth-creds", None, 'p', false, "Comma-separated list of credentials (format: username:password). If provided, the server will only serve content to authenticated clients.",
        argparse_rs::ArgType::Option,
    );
    arg_parser.add_opt(
        "php-binary",
        Some("php-cgi"),
        'P',
        false,
        "Alternate path for php binary, used to process PHP scripts with CGI (default: php-cgi)",
        argparse_rs::ArgType::Option,
    );

    let args = match arg_parser.parse(env::args().collect::<Vec<String>>().iter()) {
        Ok(args) => args,
        Err(e) => {
            arg_parser.help();
            return Err(e.to_string());
        }
    };

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
        authentication_credentials: parse_authentication_credentials(
            args.get::<String>("auth-creds"),
        )?,
        php_cgi_binary: args
            .get::<String>("php-binary")
            .ok_or("invalid php binary path")?,
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

    // parse arguments
    let server_settings = parse_args()?;
    debug!("server settings: {:?}", server_settings);

    info!("Starting server on {}", server_settings.address);

    // create server
    let mut server = Server::new(server_settings)
        .await
        .map_err(|e| e.to_string())?;
    info!("Server listening");

    // setup keypress handling
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async {
        info!("Press <q> then <Enter> to stop the server");
        let stdin = io::stdin();
        // detecting keydown events
        for c in stdin.keys() {
            if let termion::event::Key::Char('q') = c.unwrap() {
                break;
            }
        }
        // send quit signal to main task
        tx.send(()).unwrap();
    });

    tokio::select! {
        _ = rx => {},
        _ = server.listen() => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_authentication_credentials_test() {
        assert_eq!(parse_authentication_credentials(None), Ok(None));
        assert!(parse_authentication_credentials(Some(String::new())).is_err());
        assert!(parse_authentication_credentials(Some(String::from("username"))).is_err());
        assert!(parse_authentication_credentials(Some(String::from("username:"))).is_err());
        assert!(parse_authentication_credentials(Some(String::from(":password"))).is_err());
        assert_eq!(
            parse_authentication_credentials(Some(String::from("username:password"))),
            Ok(Some(vec![(
                String::from("username"),
                String::from("password")
            )]))
        );
        assert_eq!(
            parse_authentication_credentials(Some(String::from("foo:bar,bar:foo"))),
            Ok(Some(vec![
                (String::from("foo"), String::from("bar")),
                (String::from("bar"), String::from("foo"))
            ]))
        );
    }
}
