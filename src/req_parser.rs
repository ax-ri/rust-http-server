//! HTTP request parsing.

use crate::http_header::{
    GeneralHeader, HeaderValue, HeaderValueMemberName, HeaderValueMemberValue, ParsedHeaderValue,
    ReqHeader, ReqOnlyHeader, SimpleHeaderValue,
};
use crate::http_req::{ReqHead, ReqTarget, ReqVerb};

use std::{collections, str::FromStr};

use log::debug;

struct RawReqHead {
    request_line: ascii::AsciiString,
    headers: collections::HashMap<ascii::AsciiString, ascii::AsciiString>,
    last_header_name: Option<ascii::AsciiString>,
}

impl RawReqHead {
    fn new() -> Self {
        Self {
            request_line: ascii::AsciiString::new(),
            headers: collections::HashMap::new(),
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

#[derive(Debug, PartialEq)]
pub enum FirstLineParsingError {
    EmptyLine,
    InvalidFieldCount,
    InvalidVerb,
    InvalidTargetEncoding,
}

#[derive(Debug, PartialEq)]
pub enum HeaderParsingError {
    NoColon,
    SpaceBeforeColon,
    NoComponent,
    InvalidMime,
    InvalidFloat,
}

#[derive(Debug, PartialEq)]
pub enum ReqHeadParsingError {
    Ascii(ascii::AsAsciiStrError),
    FirstLine(FirstLineParsingError),
    Header(HeaderParsingError),
}

pub struct ReqHeadParser {
    state: ReqHeadParserState,
    raw_req_head: RawReqHead,
    parsed_req_head: Option<ReqHead>,
}

impl Default for ReqHeadParser {
    fn default() -> Self {
        Self::new()
    }
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
        let line = ascii::AsciiString::from_ascii(bytes)
            .map_err(|e| ReqHeadParsingError::Ascii(e.ascii_error()))?;
        debug!("Received line: {:?}", line);
        let line = line.trim();
        match self.state {
            ReqHeadParserState::RequestLine => {
                if line.is_empty() {
                    Err(ReqHeadParsingError::FirstLine(
                        FirstLineParsingError::EmptyLine,
                    ))
                } else {
                    self.raw_req_head.request_line = ascii::AsciiString::from(line);
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

                            if let Some(ascii::AsciiChar::Space) = name.last() {
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
        let mut headers = collections::HashMap::new();
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
    line: &ascii::AsciiStr,
) -> Result<(ReqVerb, ReqTarget, String), ReqHeadParsingError> {
    match *line
        .split(ascii::AsciiChar::Space)
        .collect::<Vec<_>>()
        .as_slice()
    {
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

fn parse_http_verb(verb: &ascii::AsciiStr) -> Result<ReqVerb, ReqHeadParsingError> {
    match verb.as_bytes() {
        b"GET" => Ok(ReqVerb::Get),
        _ => Err(ReqHeadParsingError::FirstLine(
            FirstLineParsingError::InvalidVerb,
        )),
    }
}

fn parse_http_target(target: &ascii::AsciiStr) -> Result<ReqTarget, ReqHeadParsingError> {
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
    name: &ascii::AsciiStr,
    value: &ascii::AsciiStr,
) -> Result<(ReqHeader, HeaderValue), ReqHeadParsingError> {
    // define some macros to make the match shorter

    // general header with simple plain value
    macro_rules! general_simple_plain {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::GeneralHeader($variant),
                HeaderValue::Simple(SimpleHeaderValue::Plain($v.to_string())),
            ))
        };
    }

    // request-only header with simple plain value
    macro_rules! req_only_simple_plain {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::ReqOnly($variant),
                HeaderValue::Simple(SimpleHeaderValue::Plain($v.to_string())),
            ))
        };
    }

    // request-only header with value parsed as plain
    macro_rules! req_only_parsed_plain {
        ($variant: expr, $v: ident) => {
            parse_header_value_plain($v).map(|v| (ReqHeader::ReqOnly($variant), v))
        };
    }

    match (name.as_bytes(), value) {
        // general headers
        (b"cache-control", v) => general_simple_plain!(GeneralHeader::CacheControl, v),
        (b"connection", v) => general_simple_plain!(GeneralHeader::Connection, v),
        (b"date", v) => general_simple_plain!(GeneralHeader::Pragma, v),
        (b"trailer", v) => general_simple_plain!(GeneralHeader::Trailer, v),
        (b"transfer-encoding", v) => general_simple_plain!(GeneralHeader::TransferEncoding, v),
        (b"upgrade", v) => general_simple_plain!(GeneralHeader::Upgrade, v),
        (b"via", v) => general_simple_plain!(GeneralHeader::Via, v),
        (b"warning", v) => general_simple_plain!(GeneralHeader::Warning, v),
        // req only headers
        (b"accept", v) => {
            parse_header_value_mime(v).map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::Accept), v))
        }
        (b"accept-charset", v) => req_only_parsed_plain!(ReqOnlyHeader::AcceptCharset, v),
        (b"accept-encoding", v) => req_only_parsed_plain!(ReqOnlyHeader::AcceptEncoding, v),
        (b"accept-language", v) => req_only_parsed_plain!(ReqOnlyHeader::AcceptLanguage, v),
        (b"authorization", v) => req_only_simple_plain!(ReqOnlyHeader::Authorization, v),
        (b"expect", v) => req_only_simple_plain!(ReqOnlyHeader::Expect, v),
        (b"from", v) => req_only_simple_plain!(ReqOnlyHeader::From, v),
        (b"host", v) => req_only_simple_plain!(ReqOnlyHeader::Host, v),
        (b"if-match", v) => req_only_simple_plain!(ReqOnlyHeader::IfMatch, v),
        (b"if-modified-since", v) => req_only_simple_plain!(ReqOnlyHeader::IfModifiedSince, v),
        (b"if-none-match", v) => req_only_simple_plain!(ReqOnlyHeader::IfNoneMatch, v),
        (b"if-range", v) => req_only_simple_plain!(ReqOnlyHeader::IfRange, v),
        (b"if-unmodified-since", v) => req_only_simple_plain!(ReqOnlyHeader::IfUnmodifiedSince, v),
        (b"max-forwards", v) => req_only_simple_plain!(ReqOnlyHeader::MaxForwards, v),
        (b"proxy-authorization", v) => req_only_simple_plain!(ReqOnlyHeader::ProxyAuthorization, v),
        (b"range", v) => req_only_simple_plain!(ReqOnlyHeader::Range, v),
        (b"referer", v) => req_only_simple_plain!(ReqOnlyHeader::Referer, v),
        (b"te", v) => req_only_simple_plain!(ReqOnlyHeader::TE, v),
        (b"user-agent", v) => req_only_simple_plain!(ReqOnlyHeader::UserAgent, v),
        // other
        (name, v) => Ok((
            ReqHeader::Other(
                ascii::AsciiString::from_ascii(name)
                    .map_err(|e| ReqHeadParsingError::Ascii(e.ascii_error()))?
                    .to_string(),
            ),
            HeaderValue::Simple(SimpleHeaderValue::Plain(v.to_string())),
        )),
    }
}

/// Wrapper function to parse a header value that is a plain string.
fn parse_header_value_plain(value: &ascii::AsciiStr) -> Result<HeaderValue, ReqHeadParsingError> {
    parse_header_value(value, |v| Ok(SimpleHeaderValue::Plain(v.to_string())))
}

/// Wrapper function to parse a header value that is a mime-type.
fn parse_header_value_mime(value: &ascii::AsciiStr) -> Result<HeaderValue, ReqHeadParsingError> {
    parse_header_value(value, |v| {
        Ok(SimpleHeaderValue::Mime(
            mime_guess::Mime::from_str(v.as_str())
                .map_err(|_| ReqHeadParsingError::Header(HeaderParsingError::InvalidMime))?,
        ))
    })
}

/// Parse a header value that is made of a list of comma-separated values.
fn parse_header_value<F>(
    value: &ascii::AsciiStr,
    main_value_parser: F,
) -> Result<HeaderValue, ReqHeadParsingError>
where
    F: Fn(&ascii::AsciiStr) -> Result<SimpleHeaderValue, ReqHeadParsingError>,
{
    let values = value
        .split(ascii::AsciiChar::Comma)
        .map(|l| l.trim())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ReqHeadParsingError::Header(HeaderParsingError::NoComponent));
    }
    let mut parsed_values = Vec::new();
    for m in values {
        let v = parse_value_and_attr(m, |v| main_value_parser(v))?;
        parsed_values.push(v);
    }
    match parsed_values.len() {
        0 => Err(ReqHeadParsingError::Header(HeaderParsingError::NoComponent)),
        _ => Ok(HeaderValue::Parsed(ParsedHeaderValue(parsed_values))),
    }
}

/// Parse a value that is made of a string and a list of attributes, separated by a semicolon.
fn parse_value_and_attr<F>(
    value_str: &ascii::AsciiStr,
    main_value_parser: F,
) -> Result<
    (
        SimpleHeaderValue,
        collections::BTreeMap<HeaderValueMemberName, HeaderValueMemberValue>,
    ),
    ReqHeadParsingError,
>
where
    F: Fn(&ascii::AsciiStr) -> Result<SimpleHeaderValue, ReqHeadParsingError>,
{
    if value_str.chars().any(|c_| c_ == ';') {
        let mut values_it = value_str.split(ascii::AsciiChar::Semicolon);
        let main_value = values_it.next().unwrap();
        let mut attributes = collections::BTreeMap::new();

        for s in values_it {
            let split = s.split(ascii::AsciiChar::Equal).collect::<Vec<_>>();
            let (member_name, member_value) = match split.len() {
                1 => (split[0], ascii::AsciiStr::from_ascii(&[]).unwrap()),
                2 => (split[0], split[1]),
                _ => return Err(ReqHeadParsingError::Header(HeaderParsingError::InvalidMime)),
            };

            let (member_name, member_value) = match (
                member_name.to_ascii_lowercase().as_bytes(),
                member_value.to_ascii_lowercase().as_bytes(),
            ) {
                (b"q", _) => (
                    HeaderValueMemberName::Quality,
                    HeaderValueMemberValue::Float(
                        ordered_float::NotNan::new(member_value.as_str().parse::<f32>().map_err(
                            |_| ReqHeadParsingError::Header(HeaderParsingError::InvalidFloat),
                        )?)
                        .map_err(|_| {
                            ReqHeadParsingError::Header(HeaderParsingError::InvalidFloat)
                        })?,
                    ),
                ),
                _ => (
                    HeaderValueMemberName::Other(member_name.to_string()),
                    HeaderValueMemberValue::Other(member_value.to_string()),
                ),
            };

            attributes.insert(member_name, member_value);
        }
        Ok((main_value_parser(main_value)?, attributes))
    } else {
        Ok((main_value_parser(value_str)?, collections::BTreeMap::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ascii(s: &str) -> &ascii::AsciiStr {
        ascii::AsciiStr::from_ascii(s).unwrap()
    }

    #[test]
    fn parse_first_line_test() {
        assert_eq!(
            parse_first_line(ascii("GET / HTTP/1.1")),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(String::from("/"), String::from("/")),
                String::from("HTTP/1.1")
            ))
        );

        assert_eq!(
            parse_first_line(ascii("GET /dir/page.html HTTP/2.0")),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(
                    String::from("/dir/page.html"),
                    String::from("/dir/page.html")
                ),
                String::from("HTTP/2.0")
            ))
        );

        assert_eq!(
            parse_first_line(ascii(
                "GET %2Fr%C3%A9pertoire%20sp%C3%A9cial%2Ffichier%20%C3%A0%20tester HTTP/1.1"
            )),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(
                    String::from("/répertoire spécial/fichier à tester"),
                    String::from("%2Fr%C3%A9pertoire%20sp%C3%A9cial%2Ffichier%20%C3%A0%20tester")
                ),
                String::from("HTTP/1.1")
            ))
        );
    }

    #[test]
    fn parse_header_test() {}

    #[test]
    fn parse_header_value_test() {
        assert_eq!(
            parse_header_value_plain(ascii("keep-alive")),
            Ok(HeaderValue::Parsed(ParsedHeaderValue(vec![(
                SimpleHeaderValue::Plain(String::from("keep-alive")),
                collections::BTreeMap::new()
            )])))
        );

        assert_eq!(
            parse_header_value_plain(ascii(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
            )),
            Ok(HeaderValue::Parsed(ParsedHeaderValue(vec![
                (
                    SimpleHeaderValue::Plain(String::from("text/html")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Plain(String::from("application/xhtml+xml")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Plain(String::from("application/xml")),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.9)
                    )])
                ),
                (
                    SimpleHeaderValue::Plain(String::from("image/avif")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Plain(String::from("image/webp")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Plain(String::from("image/apng")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Plain(String::from("*/*")),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.8)
                    )])
                ),
                (
                    SimpleHeaderValue::Plain(String::from("application/signed-exchange")),
                    collections::BTreeMap::from([
                        (
                            HeaderValueMemberName::new_other("v"),
                            HeaderValueMemberValue::new_other("b3")
                        ),
                        (
                            HeaderValueMemberName::Quality,
                            HeaderValueMemberValue::new_float(0.7)
                        )
                    ])
                )
            ])))
        );

        assert_eq!(
            parse_header_value_mime(ascii(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
            )),
            Ok(HeaderValue::Parsed(ParsedHeaderValue(vec![
                (
                    SimpleHeaderValue::Mime(mime_guess::mime::TEXT_HTML),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("application/xhtml+xml").unwrap()
                    ),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("application/xml").unwrap()
                    ),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.9)
                    )])
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("image/avif").unwrap()
                    ),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("image/webp").unwrap()
                    ),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("image/apng").unwrap()
                    ),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::Mime(mime_guess::mime::STAR_STAR),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.8)
                    )])
                ),
                (
                    SimpleHeaderValue::Mime(
                        mime_guess::mime::Mime::from_str("application/signed-exchange").unwrap()
                    ),
                    collections::BTreeMap::from([
                        (
                            HeaderValueMemberName::new_other("v"),
                            HeaderValueMemberValue::new_other("b3")
                        ),
                        (
                            HeaderValueMemberName::Quality,
                            HeaderValueMemberValue::new_float(0.7)
                        )
                    ])
                )
            ])))
        );
    }

    #[test]
    fn parse_value_and_attr_test() {}
}
