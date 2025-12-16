use crate::http_header::{
    EntityHeader, GeneralHeader, HeaderValue, HeaderValueMemberName, HeaderValueMemberValue,
    ParsedHeaderValue, ReqHeader, ReqOnlyHeader, SimpleHeaderValue,
};
use crate::http_req::{ReqHead, ReqPath, ReqTarget, ReqVerb};
use crate::req_parser::{
    FirstLineParsingError, HeaderParsingError, ReqHeadParsingError, SupportedEncoding,
};
use base64::Engine;
use std::{collections, str::FromStr};

/// Parse the first line of an HTTP request (e.g. GET /foo/bar HTTP/1.1)
pub fn parse_first_line(
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

pub fn parse_http_verb(verb: &ascii::AsciiStr) -> Result<ReqVerb, ReqHeadParsingError> {
    match verb.as_bytes() {
        b"GET" => Ok(ReqVerb::Get),
        b"POST" => Ok(ReqVerb::Post),
        b"PUT" => Ok(ReqVerb::Put),
        b"PATCH" => Ok(ReqVerb::Patch),
        b"DELETE" => Ok(ReqVerb::Delete),
        _ => Err(ReqHeadParsingError::FirstLine(
            FirstLineParsingError::InvalidVerb,
        )),
    }
}

pub fn parse_http_target(target: &ascii::AsciiStr) -> Result<ReqTarget, ReqHeadParsingError> {
    match target.as_bytes() {
        b"*" => Ok(ReqTarget::All),
        _ => {
            let (encoded_path, query) = match *target
                .split(ascii::AsciiChar::Question)
                .take(2)
                .collect::<Vec<_>>()
                .as_slice()
            {
                [target] => (target, String::new()),
                [target, params] => (
                    target,
                    String::from(
                        params
                            .split(ascii::AsciiChar::Hash)
                            .next()
                            .unwrap()
                            .as_str(),
                    ),
                ),
                _ => {
                    return Err(ReqHeadParsingError::FirstLine(
                        FirstLineParsingError::InvalidTargetQuery,
                    ));
                }
            };

            Ok(ReqTarget::Path(ReqPath {
                decoded: urlencoding::decode(encoded_path.as_str())
                    .map_err(|_| {
                        ReqHeadParsingError::FirstLine(FirstLineParsingError::InvalidTargetEncoding)
                    })?
                    .into_owned(),
                original: target.to_string(),
                query,
            }))
        }
    }
}

// because many headers are not used, exclude this function from coverage
#[cfg_attr(coverage, coverage(off))]
pub fn parse_header(
    name: &ascii::AsciiStr,
    value: &ascii::AsciiStr,
) -> Result<(ReqHeader, HeaderValue), ReqHeadParsingError> {
    // define some macros to make the match shorter

    // general header with simple plain value
    macro_rules! general_simple_plain {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::General($variant),
                HeaderValue::Simple(SimpleHeaderValue::String($v.to_string())),
            ))
        };
    }

    // request-only header with simple plain value
    macro_rules! req_only_simple_plain {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::ReqOnly($variant),
                HeaderValue::Simple(SimpleHeaderValue::String($v.to_string())),
            ))
        };
    }

    // request-only header with value parsed as plain
    macro_rules! req_only_parsed_plain {
        ($variant: expr, $v: ident) => {
            parse_header_value_plain($v).map(|v| (ReqHeader::ReqOnly($variant), v))
        };
    }

    // entity header with simple plain value
    macro_rules! entity_simple_plain {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::Entity($variant),
                HeaderValue::Simple(SimpleHeaderValue::String($v.to_string())),
            ))
        };
    }

    // entity header with simple number value
    macro_rules! entity_simple_number {
        ($variant: expr, $v: ident) => {
            Ok((
                ReqHeader::Entity($variant),
                HeaderValue::Simple(SimpleHeaderValue::Number($v.as_str().parse().map_err(
                    |_| ReqHeadParsingError::Header(HeaderParsingError::NumberParsing),
                )?)),
            ))
        };
    }

    // entity header with value parsed as plain
    macro_rules! entity_parsed_plain {
        ($variant: expr, $v: ident) => {
            parse_header_value_plain($v).map(|v| (ReqHeader::Entity($variant), v))
        };
    }

    match (name.as_bytes(), value) {
        // general headers
        (b"cache-control", v) => general_simple_plain!(GeneralHeader::CacheControl, v),
        (b"connection", v) => general_simple_plain!(GeneralHeader::Connection, v),
        (b"date", v) => general_simple_plain!(GeneralHeader::Date, v),
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
        (b"authorization", v) => parse_authorization_header(v)
            .map(|v| (ReqHeader::ReqOnly(ReqOnlyHeader::Authorization), v)),
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
        // entity header
        (b"allow", v) => entity_simple_plain!(EntityHeader::Allow, v),
        (b"content-encoding", v) => entity_parsed_plain!(EntityHeader::ContentEncoding, v),
        (b"content-language", v) => entity_parsed_plain!(EntityHeader::ContentLanguage, v),
        (b"content-length", v) => entity_simple_number!(EntityHeader::ContentLength, v),
        (b"content-location", v) => entity_simple_plain!(EntityHeader::ContentLocation, v),
        (b"content-md5", v) => entity_simple_plain!(EntityHeader::ContentMD5, v),
        (b"content-range", v) => entity_simple_plain!(EntityHeader::ContentRange, v),
        (b"content-type", v) => entity_simple_plain!(EntityHeader::ContentType, v),
        (b"expires", v) => entity_simple_plain!(EntityHeader::Expires, v),
        (b"last-modified", v) => entity_simple_plain!(EntityHeader::LastModified, v),
        // other
        (name, v) => Ok((
            ReqHeader::Other(
                ascii::AsciiString::from_ascii(name)
                    .map_err(|e| ReqHeadParsingError::Ascii(e.ascii_error()))?
                    .to_string(),
            ),
            HeaderValue::Simple(SimpleHeaderValue::String(v.to_string())),
        )),
    }
}

/// Wrapper function to parse a header value that is a plain string.
pub(crate) fn parse_header_value_plain(
    value: &ascii::AsciiStr,
) -> Result<HeaderValue, ReqHeadParsingError> {
    parse_header_value(value, |v| Ok(SimpleHeaderValue::String(v.to_string())))
}

/// Wrapper function to parse a header value that is a mime-type.
pub(crate) fn parse_header_value_mime(
    value: &ascii::AsciiStr,
) -> Result<HeaderValue, ReqHeadParsingError> {
    parse_header_value(value, |v| {
        Ok(SimpleHeaderValue::Mime(
            mime_guess::Mime::from_str(v.as_str())
                .map_err(|_| ReqHeadParsingError::Header(HeaderParsingError::InvalidMime))?,
        ))
    })
}

/// Parse a header value that is made of a list of comma-separated values.
pub fn parse_header_value<F>(
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

pub fn parse_authorization_header(
    value_str: &ascii::AsciiStr,
) -> Result<HeaderValue, ReqHeadParsingError> {
    if value_str.len() > 6 && value_str[0..=5] == *"Basic " {
        let encoded_creds =
            value_str
                .split(ascii::AsciiChar::Space)
                .nth(1)
                .ok_or(ReqHeadParsingError::Header(
                    HeaderParsingError::InvalidBasicCredentials,
                ))?;
        let decoded_creds = base64::prelude::BASE64_STANDARD
            .decode(encoded_creds.as_bytes())
            .map_err(|_e| {
                ReqHeadParsingError::Header(HeaderParsingError::InvalidBasicCredentials)
            })?;
        let decoded_creds = String::from_utf8(decoded_creds).map_err(|_e| {
            ReqHeadParsingError::Header(HeaderParsingError::InvalidBasicCredentials)
        })?;
        match *decoded_creds.splitn(2, ":").collect::<Vec<_>>().as_slice() {
            [username, password] => Ok(HeaderValue::Credentials(
                String::from(username),
                String::from(password),
            )),
            _ => Err(ReqHeadParsingError::Header(
                HeaderParsingError::InvalidBasicCredentials,
            )),
        }
    } else {
        Err(ReqHeadParsingError::Header(
            HeaderParsingError::InvalidBasicCredentials,
        ))
    }
}

pub(crate) fn extract_supported_encoding(
    v: &[(
        SimpleHeaderValue,
        collections::BTreeMap<HeaderValueMemberName, HeaderValueMemberValue>,
    )],
) -> Result<SupportedEncoding, ReqHeadParsingError> {
    let supported = v
        .iter()
        .filter_map(|(s, _)| match s {
            SimpleHeaderValue::String(s) => match s.as_str() {
                "gzip" => Some(SupportedEncoding::Gzip),
                "deflate" => Some(SupportedEncoding::Deflate),
                "zstd" => Some(SupportedEncoding::Zstd),
                "br" => Some(SupportedEncoding::Br),
                _ => None,
            },
            _ => None,
        })
        .next();
    if supported.is_none() && !v.is_empty() {
        return Err(ReqHeadParsingError::NoSupportedEncoding);
    };
    Ok(supported.unwrap())
}

/// Decode a request body using the encoding specified in the request head.
pub fn decode_req_body(req_head: &ReqHead, body: Vec<u8>) -> Result<Vec<u8>, ReqHeadParsingError> {
    if let Some(HeaderValue::Parsed(ParsedHeaderValue(v))) = req_head.body_encoding() {
        use compression::prelude::*;
        macro_rules! decode_with {
            ($d: path) => {{
                body.iter()
                    .cloned()
                    .decode(&mut $d())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| ReqHeadParsingError::BodyDecoding)
            }};
        }

        match extract_supported_encoding(v)? {
            SupportedEncoding::Gzip => decode_with!(GZipDecoder::new),
            SupportedEncoding::Deflate => decode_with!(Deflater::new),
            _ => Err(ReqHeadParsingError::NoSupportedEncoding),
        }
    } else {
        Ok(body)
    }
}
