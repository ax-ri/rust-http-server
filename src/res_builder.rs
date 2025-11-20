use crate::http_header::{EntityHeader, GeneralHeader, HeaderValue, ResHeader, ResOnlyHeader};
use crate::http_req::HttpReq;
use crate::http_res::HttpRes;
use std::collections::HashMap;

#[derive(Debug)]
pub enum ResBuildingError {
    Error,
}

pub struct ResBuilder<'a> {
    req: &'a HttpReq,
    res: HttpRes,
}

impl<'a> ResBuilder<'a> {
    pub fn new(req: &'a HttpReq) -> Self {
        Self {
            req,
            res: HttpRes::new(req.version()),
        }
    }

    pub fn do_build(&mut self) -> Result<&HttpRes, ResBuildingError> {
        let mut res_headers = HashMap::new();
        res_headers.insert(
            ResHeader::ResOnlyHeader(ResOnlyHeader::Server),
            HeaderValue::Plain(String::from("Me")),
        );

        // set date if not already present
        if !self
            .res
            .has_header(ResHeader::GeneralHeader(GeneralHeader::Date))
        {
            self.res.set_header(
                ResHeader::GeneralHeader(GeneralHeader::Date),
                HeaderValue::Plain(
                    chrono::Utc::now()
                        .format("%a, %d %b %Y %H:%M:%S GMT")
                        .to_string(),
                ),
            );
        }

        // set server origin if not already present
        if !self
            .res
            .has_header(ResHeader::ResOnlyHeader(ResOnlyHeader::Server))
        {
            self.res.set_header(
                ResHeader::ResOnlyHeader(ResOnlyHeader::Server),
                HeaderValue::Plain(String::from("rust-http-server")),
            );
        }

        self.res.set_body(Some(String::from("hello!")));

        // set content-length
        if let Some(body) = self.res.body() {
            self.res.set_header(
                ResHeader::EntityHeader(EntityHeader::ContentLength),
                HeaderValue::Number(body.len() as i32 + 2), // add 2 for the bytes of the CRLF
            )
        }

        Ok(&self.res)
    }
}
