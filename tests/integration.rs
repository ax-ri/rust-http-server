//! Server integration tests.
//!
//! Ensure the server behaves correctly in terms of content serving and HTTP errors.

use rust_http_server::server;
use rustls::pki_types::pem::PemObject;
use std::{path, pin, sync};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Launch the HTTP server in an asynchronous task.
///
/// The returned tuple is useful to remotely terminate the server:
/// * the first element is a channel that terminates the task when send into
/// * the second element is a handle on the spawned task, to await for the termination.
///
/// It is meant to be used as so:
/// ```
/// tx.send(()).unwrap(); // send the termination signal
/// handle.await.unwrap(); // wait for the termination to finish
/// ```
async fn spawn_server(
    settings: server::Settings,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let mut server = server::Server::new(settings)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .unwrap();
    let handle = tokio::spawn(async move {
        tokio::select! {
            _ = rx => (),
            _ = server.listen() => ()
        }
    });
    (tx, handle)
}

/// Create a TCP stream with the server, using TLS if needed (for HTTPS).
///
/// Such a raw TCP stream is meant to be used to send arbitrary data to the server
/// (especially invalid HTTP request, that could not be constructed with an HTTP client).
///
/// The returned tuple contains a reader and a writer to that stream.
async fn create_raw_stream(
    use_tls: bool,
    addr: &str,
) -> (
    pin::Pin<Box<dyn AsyncBufRead>>,
    pin::Pin<Box<dyn AsyncWrite>>,
) {
    if use_tls {
        let mut certs = rustls::RootCertStore::empty();
        certs
            .add(rustls::pki_types::CertificateDer::from_pem_file("./ssl/root.crt").unwrap())
            .unwrap();
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(certs)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(sync::Arc::new(config));
        let domain =
            rustls::pki_types::ServerName::try_from(addr.split(':').next().unwrap_or(addr))
                .unwrap()
                .to_owned();
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let stream = connector.connect(domain, stream).await.unwrap();

        let (reader, writer) = tokio::io::split(stream);
        let reader = tokio::io::BufReader::new(reader);
        (Box::pin(reader), Box::pin(writer))
    } else {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();

        let (reader, writer) = tokio::io::split(stream);
        let reader = tokio::io::BufReader::new(reader);
        (Box::pin(reader), Box::pin(writer))
    }
}

/// Create an HTTP client to send requests to the server.
async fn create_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        // enable this to trust self-signed certificates
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
}

/// Send some bytes to the server, and assert that the response is the expected one.
async fn do_raw_request(
    reader: &mut (impl AsyncBufRead + Unpin),
    writer: &mut (impl AsyncWrite + Unpin),
    req: &[u8],
    expected: &[u8],
) {
    // write request to stream
    writer.write_all(req).await.unwrap();
    writer.flush().await.unwrap();

    // read the first line of the response
    let mut buffer = String::new();
    reader.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, String::from_utf8(Vec::from(expected)).unwrap());

    // empty the buffer in case the response returned more lines
    while !buffer.contains("</html>") {
        buffer.clear();
        reader.read_line(&mut buffer).await.unwrap();
    }
}

fn build_url(use_tls: bool, addr: &str, route: &str) -> String {
    format!(
        "{}://{}{}",
        if use_tls { "https" } else { "http" },
        addr,
        route
    )
}

/// Send a (well-formed) HTTP request to the server, and assert that the response is the expected one.
async fn do_request(
    client: &reqwest::Client,
    use_tls: bool,
    addr: &str,
    route: &str,
    status: reqwest::StatusCode,
) -> reqwest::Response {
    let res = client.get(build_url(use_tls, addr, route)).send().await;
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status(), status);
    res
}

async fn check_res_body(
    client: &reqwest::Client,
    use_tls: bool,
    addr: &str,
    route: &str,
    body: &str,
) {
    let res = do_request(client, use_tls, addr, route, reqwest::StatusCode::OK).await;
    assert_eq!(res.text().await.unwrap(), body);
}

async fn server_http_error_test(use_tls: bool, addr: &str) {
    let (mut reader, mut writer) = create_raw_stream(use_tls, addr).await;
    let client = create_http_client().await;

    // invalid verb
    do_raw_request(
        &mut reader,
        &mut writer,
        b"foo / HTTP/1.1\r\nHost: example.org\r\n\r\n",
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;

    // missing path
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET HTTP/1.1\r\nHost: example.org\r\n\r\n",
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;

    // non-existent file
    do_request(
        &client,
        use_tls,
        addr,
        "/non-existent",
        reqwest::StatusCode::NOT_FOUND,
    )
    .await;

    // empty first line
    do_raw_request(
        &mut reader,
        &mut writer,
        b"\r\n",
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;

    // invalid header
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nHost : example.org\r\n\r\n", // space before colon
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nHost example.org\r\n\r\n", // no colon
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;

    // reset stream because last test messed it up
    drop(reader);
    drop(writer);
    let (mut reader, mut writer) = create_raw_stream(use_tls, addr).await;

    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nHost: example\r\n.org\r\n\r\n", // continued line
        b"HTTP/1.1 200 OK\r\n",
    )
    .await;
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nAccept: \r\n\r\n", // no value
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nAccept: foo\r\n\r\n", // invalid mime
        b"HTTP/1.1 400 Bad Request\r\n",
    )
    .await;
}

async fn server_connection_test(use_tls: bool, addr: &str) {
    let (mut reader, mut writer) = create_raw_stream(use_tls, addr).await;

    // send a connection-close and check that the connection has indeed been closed
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /lipsum.html HTTP/1.1\r\nConnection: close\r\n\r\n",
        b"HTTP/1.1 200 OK\r\n",
    )
    .await;
    let mut buf = [0; 1];
    let result = reader.read(&mut buf).await;
    assert!(result.is_err() || result.unwrap() == 0);
    drop(reader);
    drop(writer);

    let (mut reader, mut writer) = create_raw_stream(use_tls, addr).await;

    writer
        .write_all(b"GET /lipsum.html HTTP/1.1\r\nConnection: keep-alive\r\n\r\n")
        .await
        .unwrap();
    writer.shutdown().await.unwrap();
    let mut buf = [0; 1];
    let result = reader.read(&mut buf).await;
    assert!(result.is_ok() && result.unwrap() == 1);
}

async fn server_dir_listing_test(use_tls: bool, addr: &str, allow_dir_listing: bool) {
    let (mut reader, mut writer) = create_raw_stream(use_tls, addr).await;
    let client = create_http_client().await;

    // non-root-path
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET non-root-path HTTP/1.1\r\nHost: example.com\r\n\r\n",
        b"HTTP/1.1 404 Not Found\r\n",
    )
    .await;

    // path-traversal
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /../ HTTP/1.1\r\nHost: example.com\r\n\r\n",
        b"HTTP/1.1 403 Forbidden\r\n",
    )
    .await;
    do_raw_request(
        &mut reader,
        &mut writer,
        b"GET /../README.md HTTP/1.1\r\nHost: example.com\r\n\r\n",
        b"HTTP/1.1 403 Forbidden\r\n",
    )
    .await;

    if allow_dir_listing {
        // root dir
        check_res_body(&client, use_tls, addr, "", "<!DOCTYPE HTML> <html lang=\"en\"> <head> <meta charset=\"utf-8\"/> <title>Index of /</title> </head> <body> <h1>Index of /</h1> <hr/> <ul><li><pre><a href=\"subdir\">subdir/</a></pre></li><li><pre><a href=\"fichier à caractères spéciaux français.txt\">fichier à caractères spéciaux français.txt</a></pre></li><li><pre><a href=\"lipsum.html\">lipsum.html</a></pre></li></ul> <hr/> </body> </html> \r\n").await;
        check_res_body(&client, use_tls, addr, "/", "<!DOCTYPE HTML> <html lang=\"en\"> <head> <meta charset=\"utf-8\"/> <title>Index of /</title> </head> <body> <h1>Index of /</h1> <hr/> <ul><li><pre><a href=\"subdir\">subdir/</a></pre></li><li><pre><a href=\"fichier à caractères spéciaux français.txt\">fichier à caractères spéciaux français.txt</a></pre></li><li><pre><a href=\"lipsum.html\">lipsum.html</a></pre></li></ul> <hr/> </body> </html> \r\n").await;

        // sub-dir
        check_res_body(&client, use_tls, addr, "/subdir", "<!DOCTYPE HTML> <html lang=\"en\"> <head> <meta charset=\"utf-8\"/> <title>Index of /subdir</title> </head> <body> <h1>Index of /subdir</h1> <hr/> <ul><li><pre><a href=\"/subdir/..\">../</a></pre></li><li><pre><a href=\"/subdir/lipsum-alt.txt\">lipsum-alt.txt</a></pre></li></ul> <hr/> </body> </html> \r\n").await;
        check_res_body(&client, use_tls, addr, "/subdir/", "<!DOCTYPE HTML> <html lang=\"en\"> <head> <meta charset=\"utf-8\"/> <title>Index of /subdir/</title> </head> <body> <h1>Index of /subdir/</h1> <hr/> <ul><li><pre><a href=\"/subdir/..\">../</a></pre></li><li><pre><a href=\"/subdir/lipsum-alt.txt\">lipsum-alt.txt</a></pre></li></ul> <hr/> </body> </html> \r\n").await;
    } else {
        // root dir
        do_request(&client, use_tls, addr, "", reqwest::StatusCode::FORBIDDEN).await;
        do_request(&client, use_tls, addr, "/", reqwest::StatusCode::FORBIDDEN).await;

        // sub-dir
        do_request(
            &client,
            use_tls,
            addr,
            "/subdir",
            reqwest::StatusCode::FORBIDDEN,
        )
        .await;
        do_request(
            &client,
            use_tls,
            addr,
            "/subdir/",
            reqwest::StatusCode::FORBIDDEN,
        )
        .await;
    }
}

async fn server_content_test(use_tls: bool, addr: &str) {
    let client = create_http_client().await;

    check_res_body(
        &client,
        use_tls,
        addr,
        "/lipsum.html",
        &tokio::fs::read_to_string("./test-root/lipsum.html")
            .await
            .unwrap(),
    )
    .await;

    check_res_body(
        &client,
        use_tls,
        addr,
        "/subdir/lipsum-alt.txt",
        &tokio::fs::read_to_string("./test-root/subdir/lipsum-alt.txt")
            .await
            .unwrap(),
    )
    .await;

    check_res_body(
        &client,
        use_tls,
        addr,
        "/fichier à caractères spéciaux français.txt",
        &tokio::fs::read_to_string("./test-root/fichier à caractères spéciaux français.txt")
            .await
            .unwrap(),
    )
    .await;

    let res = client
        .get(build_url(use_tls, addr, "/lipsum.html"))
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

async fn server_test(use_tls: bool, addr: &str, allow_dir_listing: bool) {
    server_http_error_test(use_tls, addr).await;
    server_connection_test(use_tls, addr).await;
    server_dir_listing_test(use_tls, addr, allow_dir_listing).await;
    server_content_test(use_tls, addr).await;
}

#[tokio::test]
async fn integration_test() {
    env_logger::builder().is_test(true).try_init().unwrap();

    let addr = "localhost:8080";
    let socket_addr = "0.0.0.0:8080".parse().unwrap();
    let document_root = path::PathBuf::from("./test-root").canonicalize().unwrap();
    let server_cert = path::PathBuf::from("./ssl/server.crt");
    let server_key = path::PathBuf::from("./ssl/server.key");

    let mut settings = server::Settings {
        address: socket_addr,
        document_root: document_root.clone(),
        allow_dir_listing: false,
        ssl_cert_path: None,
        ssl_key_path: None,
    };

    for allow_dir_listing in &[true, false] {
        settings.allow_dir_listing = *allow_dir_listing;

        // test with HTTP
        settings.ssl_cert_path = None;
        settings.ssl_key_path = None;
        let (tx, handle) = spawn_server(settings.clone()).await;
        server_test(false, addr, *allow_dir_listing).await;
        tx.send(()).unwrap();
        handle.await.unwrap();

        // test with HTTPS
        settings.ssl_cert_path = Some(server_cert.clone());
        settings.ssl_key_path = Some(server_key.clone());
        let (tx, handle) = spawn_server(settings.clone()).await;
        server_test(true, addr, *allow_dir_listing).await;
        tx.send(()).unwrap();
        handle.await.unwrap();
    }
}
