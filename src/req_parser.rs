//! HTTP request parsing.

use crate::http_req::{HeaderValue, ReqHead, ReqHeader, ReqOnlyHeader, ReqTarget};

use regex::Regex;

use crate::http_header::GeneralHeader;
use std::cmp::PartialEq;
use std::collections::HashMap;

struct RawReqHead {
    request_line: String,
    headers: HashMap<String, String>,
    last_header_name: String,
}

impl RawReqHead {
    fn new() -> Self {
        Self {
            request_line: String::new(),
            headers: HashMap::new(),
            last_header_name: String::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum ReqHeadParserState {
    RequestLine,
    Headers,
    Done,
}

#[derive(Debug)]
pub enum ReqHeadParsingError {
    InvalidFirstLine,
    InvalidHeader,
}

pub struct ReqHeadParser {
    state: ReqHeadParserState,
    raw_req_head: RawReqHead,
    parsed_req_head: Option<ReqHead>,
}

impl ReqHeadParser {
    pub fn new() -> Self {
        Self {
            state: ReqHeadParserState::RequestLine,
            raw_req_head: RawReqHead::new(),
            parsed_req_head: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.state == ReqHeadParserState::Done
    }

    pub fn process_line(&mut self, line: &str) -> Result<(), ReqHeadParsingError> {
        match self.state {
            ReqHeadParserState::RequestLine => {
                if line.is_empty() {
                    Err(ReqHeadParsingError::InvalidFirstLine)
                } else {
                    self.raw_req_head.request_line = String::from(line);
                    self.state = ReqHeadParserState::Headers;
                    Ok(())
                }
            }
            ReqHeadParserState::Headers => {
                if line.is_empty() {
                    self.state = ReqHeadParserState::Done;
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
                                Err(ReqHeadParsingError::InvalidHeader)
                            } else {
                                self.raw_req_head
                                    .headers
                                    .entry(self.raw_req_head.last_header_name.clone())
                                    .or_default()
                                    .push_str(value);
                                Ok(())
                            }
                        }
                        _ => Err(ReqHeadParsingError::InvalidHeader),
                    }
                }
            }
            ReqHeadParserState::Done => {
                panic!("Head parser called when already done")
            }
        }
    }

    pub fn do_parse(&mut self) -> Result<ReqHead, ReqHeadParsingError> {
        let (verb, target, version) = parse_first_line(&self.raw_req_head.request_line)?;
        let mut headers = HashMap::new();
        for (name, value) in &self.raw_req_head.headers {
            let (name, value) = parse_header(name, value)?;
            headers.insert(name, value);
        }
        Ok(ReqHead::new(verb, target, version, headers))
    }

    pub fn reset(&mut self) {
        self.state = ReqHeadParserState::RequestLine;
        self.raw_req_head = RawReqHead::new();
        self.parsed_req_head = None;
    }
}

/// Parse the first line of an HTTP request (e.g. GET /foo/bar HTTP/1.1)
fn parse_first_line(line: &str) -> Result<(String, ReqTarget, String), ReqHeadParsingError> {
    let re = Regex::new(r"(?<verb>GET|HEAD|POST|PUT|DELETE|TRACE|CONNECT|OPTIONS) (?<target>.+) HTTP/(?<version>[\d.]+)")
        .expect("Cannot build request line parser");
    let caps = re
        .captures(line)
        .ok_or(ReqHeadParsingError::InvalidFirstLine)?;
    Ok((
        caps["verb"]
            .parse()
            .map_err(|_| ReqHeadParsingError::InvalidFirstLine)?,
        match &caps["target"] {
            "*" => ReqTarget::All,
            path => ReqTarget::Path(
                urlencoding::decode(path)
                    .map_err(|_| ReqHeadParsingError::InvalidFirstLine)?
                    .into_owned(),
                String::from(path),
            ),
        },
        caps["version"]
            .parse()
            .map_err(|_| ReqHeadParsingError::InvalidFirstLine)?,
    ))
}

fn parse_header(name: &str, value: &str) -> Result<(ReqHeader, HeaderValue), ReqHeadParsingError> {
    match (name, value) {
        // general headers
        ("cache-control", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::CacheControl),
            HeaderValue::Plain(String::from(v)),
        )),
        ("connection", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Plain(String::from(v)),
        )),
        ("date", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Date),
            HeaderValue::Plain(String::from(v)),
        )),
        ("pragma", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Pragma),
            HeaderValue::Plain(String::from(v)),
        )),
        ("trailer", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Trailer),
            HeaderValue::Plain(String::from(v)),
        )),
        ("transfer-encoding", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::TransferEncoding),
            HeaderValue::Plain(String::from(v)),
        )),
        ("upgrade", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Upgrade),
            HeaderValue::Plain(String::from(v)),
        )),
        ("via", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Via),
            HeaderValue::Plain(String::from(v)),
        )),
        ("warning", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Warning),
            HeaderValue::Plain(String::from(v)),
        )),
        // req only headers
        ("accept", v) => {
            parse_header_value(v).map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::Accept), v))
        }
        ("accept-charset", v) => {
            parse_header_value(v).map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptCharset), v))
        }
        ("accept-encoding", value) => parse_header_value(value)
            .map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptEncoding), v)),
        ("accept-language", value) => parse_header_value(value)
            .map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptLanguage), v)),
        ("authorization", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Authorization),
            HeaderValue::Plain(String::from(v)),
        )),
        ("expect", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Expect),
            HeaderValue::Plain(String::from(v)),
        )),
        ("from", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::From),
            HeaderValue::Plain(String::from(v)),
        )),
        ("host", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Host),
            HeaderValue::Plain(String::from(v)),
        )),
        ("if-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfMatch),
            HeaderValue::Plain(String::from(v)),
        )),
        ("if-modified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfModifiedSince),
            HeaderValue::Plain(String::from(v)),
        )),
        ("if-none-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfNoneMatch),
            HeaderValue::Plain(String::from(v)),
        )),
        ("if-range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfRange),
            HeaderValue::Plain(String::from(v)),
        )),
        ("if-unmodified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfUnmodifiedSince),
            HeaderValue::Plain(String::from(v)),
        )),
        ("max-forwards", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::MaxForwards),
            HeaderValue::Plain(String::from(v)),
        )),
        ("proxy-authorization", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::ProxyAuthorization),
            HeaderValue::Plain(String::from(v)),
        )),
        ("range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Range),
            HeaderValue::Plain(String::from(v)),
        )),
        ("referer", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Referer),
            HeaderValue::Plain(String::from(v)),
        )),
        ("te", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::TE),
            HeaderValue::Plain(String::from(v)),
        )),
        ("user-agent", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::UserAgent),
            HeaderValue::Plain(String::from(v)),
        )),
        (name, value) => Ok((
            ReqHeader::Other(String::from(name)),
            HeaderValue::Plain(String::from(value)),
        )),
    }
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value(value: &str) -> Result<HeaderValue, ReqHeadParsingError> {
    let trimmed_value = value.replace(" ", "");
    let values = trimmed_value.split(",").collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ReqHeadParsingError::InvalidHeader);
    }
    let values: Vec<(String, Vec<(String, String)>)> =
        values.iter().map(|m| parse_value_and_attr(m)).collect();
    match values.len() {
        0 => Err(ReqHeadParsingError::InvalidHeader),
        _ => Ok(HeaderValue::Parsed(values)),
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
