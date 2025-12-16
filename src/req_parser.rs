//! HTTP request parsing.
//!
//! Parsing is done in two steps: first parse the request head (first line and headers) to determine
//! if a body is expected, and then decode the body (if compressed).

mod utils;
pub use utils::decode_req_body;

use crate::http_header::{HeaderValue, ParsedHeaderValue, ReqHeader, ReqOnlyHeader};
use crate::http_req::ReqHead;

use log::debug;
use std::collections;

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
    InvalidTargetQuery,
    InvalidTargetEncoding,
}

#[derive(Debug, PartialEq)]
pub enum HeaderParsingError {
    NoColon,
    SpaceBeforeColon,
    NoComponent,
    InvalidMime,
    InvalidFloat,
    InvalidBasicCredentials,
    NumberParsing,
}

#[derive(Debug, PartialEq)]
pub enum ReqHeadParsingError {
    Ascii(ascii::AsAsciiStrError),
    FirstLine(FirstLineParsingError),
    Header(HeaderParsingError),
    NoSupportedEncoding,
    BodyDecoding,
}

#[derive(Debug)]
pub enum SupportedEncoding {
    Gzip,
    Deflate,
    Zstd,
    Br,
}

#[cfg_attr(coverage, coverage(off))]
impl From<&SupportedEncoding> for String {
    fn from(value: &SupportedEncoding) -> Self {
        match value {
            SupportedEncoding::Gzip => "gzip".to_string(),
            SupportedEncoding::Deflate => "deflate".to_string(),
            SupportedEncoding::Zstd => "zstd".to_string(),
            SupportedEncoding::Br => "br".to_string(),
        }
    }
}

pub struct ReqHeadParser {
    state: ReqHeadParserState,
    raw_req_head: RawReqHead,
    parsed_req_head: Option<ReqHead>,
}

#[cfg_attr(coverage, coverage(off))]
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

    /// Tell whether the parsing of the head is done or not. Call do_parse() when this returns true.
    pub fn is_complete(&self) -> bool {
        self.state == ReqHeadParserState::Done
    }

    /// Process a line of HTTP request head
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

    /// Return the parsed request head once the line processing is complete.
    pub fn do_parse(&mut self) -> Result<ReqHead, ReqHeadParsingError> {
        let (verb, target, version) = utils::parse_first_line(&self.raw_req_head.request_line)?;

        let mut headers = collections::HashMap::new();
        for (name, value) in &self.raw_req_head.headers {
            let (name, value) = utils::parse_header(name, value)?;
            headers.insert(name, value);
        }

        // select supported encoding if present
        let encoding = if let Some(HeaderValue::Parsed(ParsedHeaderValue(v))) =
            headers.get(&ReqHeader::ReqOnly(ReqOnlyHeader::AcceptEncoding))
        {
            Some(utils::extract_supported_encoding(v)?)
        } else {
            None
        };

        // authentication
        let authentication_credentials = if let Some(HeaderValue::Credentials(username, password)) =
            headers.get(&ReqHeader::ReqOnly(ReqOnlyHeader::Authorization))
        {
            Some((username.clone(), password.clone()))
        } else {
            None
        };

        Ok(ReqHead::new(
            verb,
            target,
            version,
            headers,
            authentication_credentials,
            encoding,
        ))
    }

    /// Reset the parser to parse a new request head
    pub fn reset(&mut self) {
        self.state = ReqHeadParserState::RequestLine;
        self.raw_req_head = RawReqHead::new();
        self.parsed_req_head = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_header::{HeaderValueMemberName, HeaderValueMemberValue};
    use crate::http_req::{ReqPath, ReqTarget, ReqVerb, SimpleHeaderValue};
    use std::str::FromStr;

    fn ascii(s: &str) -> &ascii::AsciiStr {
        ascii::AsciiStr::from_ascii(s).unwrap()
    }

    #[test]
    fn parse_first_line_test() {
        assert_eq!(
            utils::parse_first_line(ascii("GET / HTTP/1.1")),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(ReqPath {
                    decoded: String::from("/"),
                    original: String::from("/"),
                    query: String::new()
                }),
                String::from("HTTP/1.1")
            ))
        );

        assert_eq!(
            utils::parse_first_line(ascii("GET /dir/page.html HTTP/2.0")),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(ReqPath {
                    decoded: String::from("/dir/page.html"),
                    original: String::from("/dir/page.html"),
                    query: String::new()
                }),
                String::from("HTTP/2.0")
            ))
        );

        assert_eq!(
            utils::parse_first_line(ascii(
                "GET %2Fr%C3%A9pertoire%20sp%C3%A9cial%2Ffichier%20%C3%A0%20tester HTTP/1.1"
            )),
            Ok((
                ReqVerb::Get,
                ReqTarget::Path(ReqPath {
                    decoded: String::from("/répertoire spécial/fichier à tester"),
                    original: String::from(
                        "%2Fr%C3%A9pertoire%20sp%C3%A9cial%2Ffichier%20%C3%A0%20tester"
                    ),
                    query: String::new()
                }),
                String::from("HTTP/1.1")
            ))
        );
    }

    #[test]
    fn parse_header_test() {}

    #[test]
    fn parse_header_value_test() {
        assert_eq!(
            utils::parse_header_value_plain(ascii("keep-alive")),
            Ok(HeaderValue::Parsed(ParsedHeaderValue(vec![(
                SimpleHeaderValue::String(String::from("keep-alive")),
                collections::BTreeMap::new()
            )])))
        );

        assert_eq!(
            utils::parse_header_value_plain(ascii(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
            )),
            Ok(HeaderValue::Parsed(ParsedHeaderValue(vec![
                (
                    SimpleHeaderValue::String(String::from("text/html")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::String(String::from("application/xhtml+xml")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::String(String::from("application/xml")),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.9)
                    )])
                ),
                (
                    SimpleHeaderValue::String(String::from("image/avif")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::String(String::from("image/webp")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::String(String::from("image/apng")),
                    collections::BTreeMap::new()
                ),
                (
                    SimpleHeaderValue::String(String::from("*/*")),
                    collections::BTreeMap::from([(
                        HeaderValueMemberName::Quality,
                        HeaderValueMemberValue::new_float(0.8)
                    )])
                ),
                (
                    SimpleHeaderValue::String(String::from("application/signed-exchange")),
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
            utils::parse_header_value_mime(ascii(
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
