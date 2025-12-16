//! A library crate to model the HTTP protocol concepts (request, response etc.).

#![cfg_attr(coverage, feature(coverage_attribute))]

mod http_header;
pub mod http_req;
pub mod http_res;
pub mod req_parser;
pub mod res_builder;
pub mod server;
pub mod utils;
