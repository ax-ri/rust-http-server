//! HTTP request parsing.

use crate::http_req::{HeaderValue, ReqHead, ReqHeader, ReqOnlyHeader, ReqTarget, ReqVerb};

use crate::http_header::GeneralHeader;
use ascii::{AsciiChar, AsciiStr, AsciiString};
use std::cmp::PartialEq;
use std::collections::HashMap;

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
    InvalidFirstLine(FirstLineParsingError),
    InvalidHeader(HeaderParsingError),
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

    pub fn process_line(&mut self, line: &AsciiStr) -> Result<(), ReqHeadParsingError> {
        match self.state {
            ReqHeadParserState::RequestLine => {
                if line.is_empty() {
                    Err(ReqHeadParsingError::InvalidFirstLine(
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
                            dbg!(name);

                            if let Some(AsciiChar::Space) = name.last() {
                                return Err(ReqHeadParsingError::InvalidHeader(
                                    HeaderParsingError::SpaceBeforeColon,
                                ));
                            }

                            // header names should be treated case-insensitive
                            let name = AsciiString::from(name.trim()).to_ascii_lowercase();
                            self.raw_req_head
                                .headers
                                .entry(name.clone())
                                .or_default()
                                .push_str(value.trim());
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
                                    .push_str(line.trim());
                                Ok(())
                            } else {
                                // error if there is no previous header
                                Err(ReqHeadParsingError::InvalidHeader(
                                    HeaderParsingError::NoColon,
                                ))
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
fn parse_first_line(
    line: &AsciiStr,
) -> Result<(ReqVerb, ReqTarget, AsciiString), ReqHeadParsingError> {
    match *line.split(AsciiChar::Space).collect::<Vec<_>>().as_slice() {
        [verb, target, version] => Ok((
            parse_http_verb(verb)?,
            parse_http_target(target)?,
            AsciiString::from(version),
        )),
        _ => Err(ReqHeadParsingError::InvalidFirstLine(
            FirstLineParsingError::InvalidFieldCount,
        )),
    }
}

fn parse_http_verb(verb: &AsciiStr) -> Result<ReqVerb, ReqHeadParsingError> {
    match verb.as_bytes() {
        b"GET" => Ok(ReqVerb::Get),
        _ => Err(ReqHeadParsingError::InvalidFirstLine(
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
                    ReqHeadParsingError::InvalidFirstLine(
                        FirstLineParsingError::InvalidTargetEncoding,
                    )
                })?
                .into_owned(),
            AsciiString::from(target),
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
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"connection", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Connection),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"date", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Date),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"pragma", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Pragma),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"trailer", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Trailer),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"transfer-encoding", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::TransferEncoding),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"upgrade", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Upgrade),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"via", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Via),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"warning", v) => Ok((
            ReqHeader::GeneralHeader(GeneralHeader::Warning),
            HeaderValue::Plain(AsciiString::from(v)),
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
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"expect", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Expect),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"from", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::From),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"host", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Host),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"if-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfMatch),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"if-modified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfModifiedSince),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"if-none-match", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfNoneMatch),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"if-range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfRange),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"if-unmodified-since", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::IfUnmodifiedSince),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"max-forwards", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::MaxForwards),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"proxy-authorization", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::ProxyAuthorization),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"range", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Range),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"referer", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::Referer),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"te", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::TE),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (b"user-agent", v) => Ok((
            ReqHeader::ReqOnly(ReqOnlyHeader::UserAgent),
            HeaderValue::Plain(AsciiString::from(v)),
        )),
        (name, value) => Ok((
            ReqHeader::Other(AsciiString::from_ascii(name).unwrap()),
            HeaderValue::Plain(AsciiString::from(value)),
        )),
    }
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value(value: &AsciiStr) -> Result<HeaderValue, ReqHeadParsingError> {
    let trimmed_value = value; //.replace(" ", "");
    let values = trimmed_value.split(AsciiChar::Comma).collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ReqHeadParsingError::InvalidHeader(
            HeaderParsingError::NoComponent,
        ));
    }
    let values: Vec<(AsciiString, Vec<(AsciiString, AsciiString)>)> =
        values.iter().map(|m| parse_value_and_attr(m)).collect();
    match values.len() {
        0 => Err(ReqHeadParsingError::InvalidHeader(
            HeaderParsingError::NoComponent,
        )),
        _ => Ok(HeaderValue::Parsed(values)),
    }
}

/// Parse a value that is made of a string and a list of attributes, separated by a semicolon.
fn parse_value_and_attr(value_str: &AsciiStr) -> (AsciiString, Vec<(AsciiString, AsciiString)>) {
    if value_str.chars().any(|c_| c_ == ';') {
        let mut values_it = value_str.split(AsciiChar::Semicolon);
        let main_value = values_it.next().unwrap();
        let attributes = values_it
            .filter_map(|s| {
                let split = s.split(AsciiChar::Equal).collect::<Vec<_>>();
                match split.len() {
                    1 => Some((AsciiString::from(split[0]), AsciiString::new())),
                    2 => Some((AsciiString::from(split[0]), AsciiString::from(split[1]))),
                    _ => None,
                }
            })
            .collect();
        (AsciiString::from(main_value), attributes)
    } else {
        (AsciiString::from(value_str), Vec::new())
    }
}
