//! HTTP request parsing.

use crate::http_req::{
    HttpHeaderValue, HttpReqHead, HttpReqHeader, HttpReqTarget, ReqOnlyHttpHeader,
};

use regex::Regex;

use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

struct RawHttpReqHead {
    request_line: String,
    headers: HashMap<String, String>,
    last_header_name: String,
}

impl RawHttpReqHead {
    fn new() -> Self {
        Self {
            request_line: String::new(),
            headers: HashMap::new(),
            last_header_name: String::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum HttpReqHeadParserState {
    RequestLine,
    Headers,
    Done,
}

#[derive(Debug)]
pub enum HttpReqHeadParsingError {
    InvalidFirstLine,
    InvalidHeader,
}

pub struct HttpReqHeadParser {
    state: HttpReqHeadParserState,
    raw_req_head: RawHttpReqHead,
    parsed_req_head: Option<HttpReqHead>,
}

impl HttpReqHeadParser {
    pub fn new() -> Self {
        Self {
            state: HttpReqHeadParserState::RequestLine,
            raw_req_head: RawHttpReqHead::new(),
            parsed_req_head: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.state == HttpReqHeadParserState::Done
    }

    pub fn process_line(&mut self, line: &str) -> Result<(), HttpReqHeadParsingError> {
        match self.state {
            HttpReqHeadParserState::RequestLine => {
                if line.is_empty() {
                    Err(HttpReqHeadParsingError::InvalidFirstLine)
                } else {
                    self.raw_req_head.request_line = String::from(line);
                    self.state = HttpReqHeadParserState::Headers;
                    Ok(())
                }
            }
            HttpReqHeadParserState::Headers => {
                if line.is_empty() {
                    self.state = HttpReqHeadParserState::Done;
                    Ok(())
                } else {
                    match *line.splitn(2, ":").collect::<Vec<_>>() {
                        // typical name: value header line
                        [name, value] => {
                            // header names should be treated case-insensitive
                            let name = String::from(name).to_ascii_lowercase();
                            self.raw_req_head
                                .headers
                                .entry(name.clone())
                                .or_default()
                                .push_str(value.trim_start());
                            self.raw_req_head.last_header_name = name;
                            Ok(())
                        }
                        // if the line has not ':', then it may be the previous header line continued
                        [value] => {
                            // error if there is no previous header
                            if self.raw_req_head.last_header_name.is_empty() {
                                Err(HttpReqHeadParsingError::InvalidHeader)
                            } else {
                                self.raw_req_head
                                    .headers
                                    .entry(self.raw_req_head.last_header_name.clone())
                                    .or_default()
                                    .push_str(value);
                                Ok(())
                            }
                        }
                        _ => Err(HttpReqHeadParsingError::InvalidHeader),
                    }
                }
            }
            HttpReqHeadParserState::Done => {
                panic!("Head parser called when already done")
            }
        }
    }

    pub fn do_parse(&mut self) -> Result<HttpReqHead, HttpReqHeadParsingError> {
        let (verb, target, version) = parse_first_line(&self.raw_req_head.request_line)?;
        let mut headers = HashSet::new();
        for (name, value) in &self.raw_req_head.headers {
            headers.insert(parse_header(name, value)?);
        }
        Ok(HttpReqHead::new(verb, target, version, headers))
    }

    pub fn reset(&mut self) {
        self.state = HttpReqHeadParserState::RequestLine;
        self.raw_req_head = RawHttpReqHead::new();
        self.parsed_req_head = None;
    }
}

/// Parse the first line of an HTTP request (e.g. GET /foo/bar HTTP/1.1)
fn parse_first_line(
    line: &str,
) -> Result<(String, HttpReqTarget, String), HttpReqHeadParsingError> {
    let re = Regex::new(r"(?<verb>GET|HEAD|POST|PUT|DELETE|TRACE|CONNECT|OPTIONS) (?<target>.+) HTTP/(?<version>[\d.]+)")
        .expect("Cannot build request line parser");
    let caps = re
        .captures(line)
        .ok_or(HttpReqHeadParsingError::InvalidFirstLine)?;
    Ok((
        caps["verb"]
            .parse()
            .map_err(|_| HttpReqHeadParsingError::InvalidFirstLine)?,
        match &caps["target"] {
            "*" => HttpReqTarget::Other(String::from("*")),
            path => HttpReqTarget::Path(PathBuf::from(path)),
        },
        caps["version"]
            .parse()
            .map_err(|_| HttpReqHeadParsingError::InvalidFirstLine)?,
    ))
}

fn parse_header(name: &str, value: &str) -> Result<HttpReqHeader, HttpReqHeadParsingError> {
    match (name, value) {
        ("accept", value) => parse_header_value(value)
            .map(|v| HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::Accept(v))),
        ("accept-charset", value) => parse_header_value(value)
            .map(|v| HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::AcceptCharset(v))),
        ("accept-encoding", value) => parse_header_value(value)
            .map(|v| HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::AcceptEncoding(v))),
        ("accept-language", value) => parse_header_value(value)
            .map(|v| HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::AcceptLanguage(v))),
        ("host", value) => Ok(HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::Host(
            HttpHeaderValue::Plain(String::from(value)),
        ))),
        ("user-agent", value) => Ok(HttpReqHeader::ReqHeader(ReqOnlyHttpHeader::UserAgent(
            HttpHeaderValue::Plain(String::from(value)),
        ))),
        (name, value) => Ok(HttpReqHeader::Other(
            String::from(name),
            String::from(value),
        )),
    }
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value(value: &str) -> Result<HttpHeaderValue, HttpReqHeadParsingError> {
    let trimmed_value = value.replace(" ", "");
    let values = trimmed_value.split(",").collect::<Vec<_>>();
    if values.is_empty() {
        return Err(HttpReqHeadParsingError::InvalidHeader);
    }
    println!("values {:?}", values);
    let values: Vec<(String, Vec<(String, String)>)> =
        values.iter().map(|m| parse_value_and_attr(m)).collect();
    match values.len() {
        0 => Err(HttpReqHeadParsingError::InvalidHeader),
        _ => Ok(HttpHeaderValue::Parsed(values)),
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
