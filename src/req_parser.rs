//! HTTP request parsing.

use crate::http_req::{HeaderValue, ReqHead, ReqHeader, ReqOnlyHeader, ReqTarget, ReqVerb};

use crate::http_header::{GeneralHeader, HeaderValueMemberName, HeaderValueMemberValue};
use crate::req_parser::ReqHeadParsingError::Ascii;
use ascii::{AsAsciiStrError, AsciiChar, AsciiStr, AsciiString};
use log::debug;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashMap};

struct RawReqHead {
    request_line: AsciiString,
    headers: HashMap<AsciiString, AsciiString>,
    last_header_name: Option<AsciiString>,
}

impl RawReqHead {
    fn new() -> Self {
        Self {
            request_line: AsciiString::new(),
            headers: HashMap::new(),
            last_header_name: None,
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
pub enum FirstLineParsingError {
    EmptyLine,
    InvalidFieldCount,
    InvalidVerb,
    InvalidTargetEncoding,
}

#[derive(Debug)]
pub enum HeaderParsingError {
    NoColon,
    SpaceBeforeColon,
    NoComponent,
}

#[derive(Debug)]
pub enum ReqHeadParsingError {
    Ascii(AsAsciiStrError),
    FirstLine(FirstLineParsingError),
    Header(HeaderParsingError),
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

    pub fn process_bytes(&mut self, bytes: Vec<u8>) -> Result<(), ReqHeadParsingError> {
        let line = AsciiString::from_ascii(bytes).map_err(|e| Ascii(e.ascii_error()))?;
        debug!("Received line: {:?}", line);
        let line = line.trim();
        match self.state {
            ReqHeadParserState::RequestLine => {
                if line.is_empty() {
                    Err(ReqHeadParsingError::FirstLine(
                        FirstLineParsingError::EmptyLine,
                    ))
                } else {
                    self.raw_req_head.request_line = AsciiString::from(line);
                    self.state = ReqHeadParserState::Headers;
                    Ok(())
                }
            }
            ReqHeadParserState::Headers => {
                if line.is_empty() {
                    self.state = ReqHeadParserState::Done;
                    Ok(())
                } else {
                    match line.chars().position(|c_| c_ == ':') {
                        // typical name: value header line
                        Some(colon_idx) => {
                            let (name, value) = (&line[..colon_idx], &line[colon_idx + 1..]);

                            if let Some(AsciiChar::Space) = name.last() {
                                return Err(ReqHeadParsingError::Header(
                                    HeaderParsingError::SpaceBeforeColon,
                                ));
                            }

                            // header names should be treated case-insensitive
                            let name = name.trim().to_ascii_lowercase();
                            self.raw_req_head
                                .headers
                                .entry(name.clone())
                                .or_default()
                                .push_str(value.trim_start());
                            self.raw_req_head.last_header_name = Some(name);
                            Ok(())
                        }
                        // if the line has no ':', then it may be the previous header line continued
                        None => {
                            if let Some(name) = self.raw_req_head.last_header_name.as_ref() {
                                self.raw_req_head
                                    .headers
                                    .entry(name.clone())
                                    .or_default()
                                    .push_str(line);
                                Ok(())
                            } else {
                                // error if there is no previous header
                                Err(ReqHeadParsingError::Header(HeaderParsingError::NoColon))
                            }
                        }
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
fn parse_first_line(line: &AsciiStr) -> Result<(ReqVerb, ReqTarget, String), ReqHeadParsingError> {
    match *line.split(AsciiChar::Space).collect::<Vec<_>>().as_slice() {
        [verb, target, version] => Ok((
            parse_http_verb(verb)?,
            parse_http_target(target)?,
            version.to_string(),
        )),
        _ => Err(ReqHeadParsingError::FirstLine(
            FirstLineParsingError::InvalidFieldCount,
        )),
    }
}

fn parse_http_verb(verb: &AsciiStr) -> Result<ReqVerb, ReqHeadParsingError> {
    match verb.as_bytes() {
        b"GET" => Ok(ReqVerb::Get),
        _ => Err(ReqHeadParsingError::FirstLine(
            FirstLineParsingError::InvalidVerb,
        )),
    }
}

fn parse_http_target(target: &AsciiStr) -> Result<ReqTarget, ReqHeadParsingError> {
    match target.as_bytes() {
        b"*" => Ok(ReqTarget::All),
        _ => Ok(ReqTarget::Path(
            urlencoding::decode(target.as_str())
                .map_err(|_| {
                    ReqHeadParsingError::FirstLine(FirstLineParsingError::InvalidTargetEncoding)
                })?
                .into_owned(),
            target.to_string(),
        )),
    }
}

fn parse_header(
    name: &AsciiStr,
    value: &AsciiStr,
) -> Result<(ReqHeader, HeaderValue), ReqHeadParsingError> {
    match (name.as_bytes(), value) {
        // general headers
        (b"cache-control", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::CacheControl),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"connection", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"date", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Date),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"pragma", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Pragma),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"trailer", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Trailer),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"transfer-encoding", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::TransferEncoding),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"upgrade", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Upgrade),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"via", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Via),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"warning", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Warning),
            HeaderValue::Plain(v.to_string()),
        )),
        // req only headers
        (b"accept", v) => {
            parse_header_value(v).map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::Accept), v))
        }
        (b"accept-charset", v) => {
            parse_header_value(v).map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptCharset), v))
        }
        (b"accept-encoding", value) => parse_header_value(value)
            .map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptEncoding), v)),
        (b"accept-language", value) => parse_header_value(value)
            .map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::AcceptLanguage), v)),
        (b"authorization", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Authorization),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"expect", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Expect),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"from", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::From),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"host", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Host),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"if-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfMatch),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"if-modified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfModifiedSince),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"if-none-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfNoneMatch),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"if-range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfRange),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"if-unmodified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfUnmodifiedSince),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"max-forwards", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::MaxForwards),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"proxy-authorization", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::ProxyAuthorization),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Range),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"referer", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Referer),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"te", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::TE),
            HeaderValue::Plain(v.to_string()),
        )),
        (b"user-agent", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::UserAgent),
            HeaderValue::Plain(v.to_string()),
        )),
        (name, v) => Ok((
            ReqHeader::Other(
                AsciiString::from_ascii(name)
                    .map_err(|e| Ascii(e.ascii_error()))?
                    .to_string(),
            ),
            HeaderValue::Plain(v.to_string()),
        )),
    }
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value(value: &AsciiStr) -> Result<HeaderValue, ReqHeadParsingError> {
    let values = value
        .split(AsciiChar::Comma)
        .map(|l| l.trim())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ReqHeadParsingError::Header(HeaderParsingError::NoComponent));
    }
    let values: Vec<(
        String,
        BTreeMap<HeaderValueMemberName, HeaderValueMemberValue>,
    )> = values.iter().map(|m| parse_value_and_attr(m)).collect();
    match values.len() {
        0 => Err(ReqHeadParsingError::Header(HeaderParsingError::NoComponent)),
        _ => Ok(HeaderValue::Parsed(values)),
    }
}

/// Parse a value that is made of a string and a list of attributes, separated by a semicolon.
fn parse_value_and_attr(
    value_str: &AsciiStr,
) -> (
    String,
    BTreeMap<HeaderValueMemberName, HeaderValueMemberValue>,
) {
    if value_str.chars().any(|c_| c_ == ';') {
        let mut values_it = value_str.split(AsciiChar::Semicolon);
        let main_value = values_it.next().unwrap();
        let attributes = values_it
            .filter_map(|s| {
                let split = s.split(AsciiChar::Equal).collect::<Vec<_>>();
                match split.len() {
                    1 => Some((split[0], AsciiStr::from_ascii(&[]).unwrap())),
                    2 => Some((split[0], split[1])),
                    _ => None,
                }
            })
            .map(|(attr_name, attr_value)| {
                match (
                    attr_name.to_ascii_lowercase().as_bytes(),
                    attr_value.to_ascii_lowercase().as_bytes(),
                ) {
                    (b"charset", b"utf-8") => {
                        (HeaderValueMemberName::Charset, HeaderValueMemberValue::UTF8)
                    }
                    _ => (
                        HeaderValueMemberName::Other(attr_name.to_string()),
                        HeaderValueMemberValue::Other(attr_value.to_string()),
                    ),
                }
            })
            .collect();
        (main_value.to_string(), attributes)
    } else {
        (value_str.to_string(), BTreeMap::new())
    }
}
