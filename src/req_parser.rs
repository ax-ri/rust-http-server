use crate::req_parser::ParserState::RequestLine;
use log::error;
use regex::Regex;
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

struct HttpHeaderParsedValue {
    original: String,
    parsed: Vec<(String, Vec<(String, String)>)>,
}

enum HttpHeader {
    Accept(HttpHeaderParsedValue),
    AcceptCharset(HttpHeaderParsedValue),
    AcceptEncoding(HttpHeaderParsedValue),
    AcceptLanguage(HttpHeaderParsedValue),
    // Authorization,
    // Expect,
    // From,
    Host(String),
    // IfMatch,
    // IfModifiedSince,
    // If-None-Match,
    // If-Range,
    // If-Unmodified-Since,
    // Max-Forwards,
    // Proxy-Authorization,
    // Range,
    // Referer,
    // TE
    UserAgent(String),
    Other(String, String),
}

impl Display for HttpHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpHeader::Accept(value) => write!(f, "Accept: {:?}", value.parsed),
            HttpHeader::AcceptCharset(value) => write!(f, "Accept-Charset: {:?}", value.parsed),
            HttpHeader::AcceptEncoding(value) => write!(f, "Accept-Encoding: {:?}", value.parsed),
            HttpHeader::AcceptLanguage(value) => write!(f, "Accept-Language: {:?}", value.parsed),
            HttpHeader::Host(host) => write!(f, "Host: {}", host),
            HttpHeader::UserAgent(host) => write!(f, "User-Agent: {}", host),
            HttpHeader::Other(name, value) => write!(f, "{}: {}", name, value),
        }
    }
}

struct HttpRequest {
    verb: String,
    path: PathBuf,
    version: String,
    headers: Vec<HttpHeader>,
}

impl HttpRequest {
    fn new() -> Self {
        HttpRequest {
            verb: String::new(),
            path: PathBuf::new(),
            version: String::new(),
            headers: Vec::new(),
        }
    }
}

impl Display for HttpRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut res = write!(
            f,
            "{} {} HTTP/{}\n",
            self.verb,
            self.path.to_str().unwrap(),
            self.version,
        );
        self.headers.iter().for_each(|h| {
            res = res.and(match h {
                HttpHeader::Other(_, _) => write!(f, "(Other) {}\n", h),
                _ => write!(f, "{}\n", h),
            })
        });
        res
    }
}

#[derive(Debug, PartialEq)]
enum ParserState {
    RequestLine,
    Headers,
}

#[derive(Debug)]
enum ParsingError {
    InvalidFirstLine,
    InvalidHeader,
}

pub struct ReqParser {
    state: ParserState,
    req: HttpRequest,
}

impl ReqParser {
    pub fn new() -> Self {
        Self {
            state: RequestLine,
            req: HttpRequest::new(),
        }
    }

    pub fn parse_line(&mut self, line: &str) -> bool {
        match self.state {
            ParserState::RequestLine => {
                match parse_first_line(line) {
                    Ok((verb, path, version)) => {
                        self.req.verb = verb;
                        self.req.path = path;
                        self.req.version = version;
                        self.state = ParserState::Headers;
                    }
                    Err(e) => error!("parse_line: error {:?}", e),
                };
                false
            }
            ParserState::Headers => match line {
                "\n" | "\r\n" => {
                    println!("REQUEST\n{}", self.req);
                    true
                }
                _ => {
                    match parse_header(line) {
                        Ok(header) => self.req.headers.push(header),
                        Err(e) => error!("parse_line: error {:?}", e),
                    };
                    false
                }
            },
        }
    }

    pub fn reset(&mut self) {
        self.state = ParserState::RequestLine;
        self.req = HttpRequest::new();
    }
}

/// Parse the first line of an HTTP request (e.g. GET /foo/bar HTTP/1.1)
fn parse_first_line(line: &str) -> Result<(String, PathBuf, String), ParsingError> {
    let re = Regex::new(r"(?<verb>GET|HEAD|POST|PUT|DELETE|TRACE|CONNECT|OPTIONS) (?<path>.+) HTTP/(?<version>[\d.]+)")
        .unwrap();
    re.captures(line)
        .map(|caps| {
            (
                caps["verb"].parse().expect("Failed to parse HTTP verb"),
                PathBuf::from(
                    caps["path"]
                        .parse::<String>()
                        .expect("Failed to parse HTTP request path"),
                ),
                caps["version"]
                    .parse()
                    .expect("Failed to parse HTTP version"),
            )
        })
        .ok_or(ParsingError::InvalidFirstLine)
}

fn parse_header(line: &str) -> Result<HttpHeader, ParsingError> {
    match *line.trim().split(": ").collect::<Vec<_>>() {
        ["Accept", value] => parse_header_value(value).map(|v| HttpHeader::Accept(v)),
        ["Accept-Charset", value] => {
            parse_header_value(value).map(|v| HttpHeader::AcceptCharset(v))
        }
        ["Accept-Encoding", value] => {
            parse_header_value(value).map(|v| HttpHeader::AcceptEncoding(v))
        }
        ["Accept-Language", value] => {
            parse_header_value(value).map(|v| HttpHeader::AcceptLanguage(v))
        }
        ["Host", value] => Ok(HttpHeader::Host(String::from(value))),
        ["User-Agent", value] => Ok(HttpHeader::UserAgent(String::from(value))),
        [name, value] => Ok(HttpHeader::Other(String::from(name), String::from(value))),
        _ => Err(ParsingError::InvalidHeader),
    }
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value(value: &str) -> Result<HttpHeaderParsedValue, ParsingError> {
    let trimmed_value = value.replace(" ", "");
    let values = trimmed_value.split(",").collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ParsingError::InvalidHeader);
    }
    println!("values {:?}", values);
    let values: Vec<(String, Vec<(String, String)>)> =
        values.iter().map(|m| parse_value_and_attr(m)).collect();
    match values.len() {
        0 => Err(ParsingError::InvalidHeader),
        _ => Ok(HttpHeaderParsedValue {
            original: String::from(value),
            parsed: values,
        }),
    }
}

/// Parse a value that is made of a string and a list of attributes, separated by a semicolon.
fn parse_value_and_attr(value_str: &str) -> (String, Vec<(String, String)>) {
    if value_str.contains(";") {
        let mut values_it = value_str.split(";");
        let main_value = values_it.next().unwrap();
        let attributes = values_it
            .filter_map(|s| {
                let split = s.split('=').collect::<Vec<_>>();
                match split.len() {
                    1 => Some((String::from(split[0]), String::new())),
                    2 => Some((String::from(split[0]), String::from(split[1]))),
                    _ => None,
                }
            })
            .collect();
        (String::from(main_value), attributes)
    } else {
        (String::from(value_str), Vec::new())
    }
}
