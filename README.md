# `rust-http-server`

This project implements an HTTP web server from scratch in Rust. See the [
`project.md`](./project.md) file for more details.

## Self-signed certificates

For testing purposes, this project uses a self-signed TLS certificate and private key (to enable HTTPS). They are stored
in the [`ssl/`](./ssl) directory of this repository.

For reference, below are the commands used for the generation of the custom CA and certificate signature:

```bash
# based on https://stackoverflow.com/a/76385343

# create a CA
openssl req -x509 -noenc -subj '/CN=localhost' -newkey rsa -keyout ssl/root.key -out ssl/root.crt
# create a certificate signing request (CSR)
openssl req -noenc -newkey rsa -keyout ssl/server.key -out /tmp/client.csr -subj '/CN=localhost' -addext subjectAltName=DNS:localhost
# sign it using the CA
openssl x509 -req -in /tmp/client.csr -CA ssl/root.crt -CAkey ssl/root.key -days 365 -out ssl/server.crt -copy_extensions copy
```