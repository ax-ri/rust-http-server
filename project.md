## HTTP Web Server from scratch

### Student

- Axel Richard ([axel.richard@ensta.fr](mailto:axel.richard@ensta.fr))

### Description

The goal of this project is to implement an HTTP web server (as
defined [here](https://en.wikipedia.org/wiki/Web_server)) from scratch . This application will be accepting TCP
connections and respond to HTTP requests, following the behaviour described
in [RFC 9112](https://datatracker.ietf.org/doc/html/rfc9112#section-1).

The server will support the version 1.1 of HTTP (as
described [here](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Evolution_of_HTTP)).

The idea is to start from a very basic server application and then gradually add more advanced features, as listed
below.

- [ ] Basic features
    - [x] Listening for and accepting one TCP connection
    - [x] Static content support: serving resources (GET method) of different types (text and binary files, i.e. HTML,
      images, PDF files etc.)
    - [ ] Support for [basic authentication](https://www.rfc-editor.org/rfc/rfc7617)
- [ ] Medium features
    - [x] Handling several concurrent connections
    - [ ] Support of usual [request headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers) (
      *accept* headers, CORS, web socket upgrade request etc.)
    - [x] TLS encryption support (HTTPS)
- [ ] Advanced (or optional) features
    - [ ] Dynamic content support
    - [ ] handling other HTTP methods (POST, PUT, PATCH, DELETE) and request body processing
    - [ ] interfacing with PHP language to handle dynamic HTML pages
    - [ ] Support for caching
    - [ ] Support for configuration file (like Apache httpd or nginx) to define virtual hosts etc.

At the time of the deadline, I expect basic and medium features to be implemented, and at least one of the advanced
features.
