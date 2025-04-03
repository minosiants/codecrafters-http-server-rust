use std::string::FromUtf8Error;

use bytes::BufMut;
use nom::{AsBytes, IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::take_until;
use nom::character::complete::crlf;
use nom::combinator::{map, map_parser, rest};
use nom::multi::many0;
use derive_more::{Deref, From};
use crate::{Error, Result};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HttpVersion {
    HttpOne
}
impl From<HttpVersion> for Vec<u8> {
    fn from(value: HttpVersion) -> Self {
        match value {
            HttpVersion::HttpOne => b"HTTP/1.1".to_vec()
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum StatusCode {
    SC200,
    SC201,
    SC404

}
impl From<StatusCode> for Vec<u8> {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::SC200 => b"200".to_vec(),
            StatusCode::SC201 => b"201".to_vec(),
            StatusCode::SC404 => b"404".to_vec()
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub enum Reason {
    Ok,
    Created,
    NotFound
}

impl From<Reason> for Vec<u8> {
    fn from(value: Reason) -> Self {
        match value {
            Reason::Ok => b"OK".to_vec(),
            Reason::Created => b"Created".to_vec(),
            Reason::NotFound => b"Not Found".to_vec()
        }
    }
}

#[derive(Debug, Clone, From, Deref,PartialEq)]
struct Host(String);

#[derive(Debug, Clone, From, Deref,PartialEq)]
pub struct UserAgent(pub String);
#[derive(Debug, Clone,From, Deref,PartialEq)]
struct Accept(String);
#[derive(Debug, Clone,PartialEq)]
pub enum ContentType {
    TextPlain,
    OctetStream
}
impl ContentType {
    pub fn from2(s:&str) -> Result<ContentType> {
        match s {
            "text/plain" => Ok(ContentType::TextPlain),
            "application/octet-stream" =>    Ok(ContentType::OctetStream),
             ss => Err(Error::GeneralError(format!("not able to reate ContentType from {}", ss)))
        }
    }
}
impl From<ContentType> for Vec<u8> {
    fn from(value: ContentType) -> Self {
        match value {
            ContentType::TextPlain => b"text/plain".to_vec(),
            ContentType::OctetStream => b"application/octet-stream".to_vec()
        }
    }
}
#[derive(Debug, Clone,From, Deref, Copy, PartialEq)]
pub struct ContentLength(u32);

#[derive(Debug, Clone,From, PartialEq)]
#[from(forward)]
pub enum Header {
    Host(Host),
    UserAgent(UserAgent),
    Accept(Accept),
    ContentType(ContentType),
    ContentLength(ContentLength)
}
impl Header {
    pub fn host(value:&str) -> Self {
        Self::Host(Host(value.to_string()))
    }
    pub fn user_agent(value:&str) -> Self {
        Self::UserAgent(UserAgent(value.to_string()))
    }
    pub fn accept(value:&str) -> Self {
        Self::Accept(Accept(value.to_string()))
    }
    pub fn content_type(value:ContentType) -> Self {
        Self::ContentType(value)
    }
    pub fn content_length(value:u32) -> Self {
        Self::ContentLength(ContentLength(value))
    }
}
impl From<Header> for Vec<u8> {
    fn from(value: Header) -> Self {
        match value {
            Header::Host(host) => format!("Host: {:?}",host.0 ).as_bytes().to_vec(),
            Header::UserAgent(agent) => format!("User-Agent: {:?}", agent.0).as_bytes().to_vec(),
            Header::Accept(accept) => format!("Accept: {:?}", accept.0).as_bytes().to_vec(),
            Header::ContentType(ct) => {
                let mut r:Vec<u8> = b"Content-Type: ".to_vec();
                let ctv:Vec<u8> = ct.into();
                r.extend(ctv);
                r
            }
            Header::ContentLength(cl) => format!("Content-Length:{:?}", cl.0).as_bytes().to_vec()
        }
    }
}

#[derive(From, Debug, Clone, Deref)]
pub struct Headers(Vec<Header>);
impl Headers {
    pub fn content_length(&self) -> Option<ContentLength> {
        self.clone().0.into_iter().find_map(|v|
            match v {
               Header::ContentLength(cl) => Some(cl ),
                _ => None
            }
        )
    }
    pub fn user_agent(&self) -> Option<UserAgent> {
        self.clone().0.into_iter().find_map(|v| match v {
            Header::UserAgent(v) => Some(v.clone()),
            _ => None
        })
    }

}
#[derive(Debug, Clone)]
pub struct ResponseBody(pub String);

impl From<ResponseBody> for Vec<u8> {
    fn from(value: ResponseBody) -> Self {
        value.0.as_bytes().to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StatusLine(
    HttpVersion,
    StatusCode,
    Option<Reason>);

impl StatusLine {
    pub fn ok() -> StatusLine {
        Self(
            HttpVersion::HttpOne,
            StatusCode::SC200,
            Some(Reason::Ok),
        )
    }
    pub fn created() -> StatusLine {
        Self(
            HttpVersion::HttpOne,
            StatusCode::SC200,
            Some(Reason::Ok),
        )
    }
    pub fn not_found() -> StatusLine {
        Self(
            HttpVersion::HttpOne,
            StatusCode::SC404,
            Some(Reason::NotFound)
        )
    }
}

impl From<StatusLine> for Vec<u8> {
    fn from(value: StatusLine) -> Self {
        let StatusLine(http_version, status_code, reason) = value;
        let mut result: Vec<u8> = vec![];
        result.extend::<Vec<u8>>(http_version.into());
        result.extend(SPACE);
        result.extend::<Vec<u8>>(status_code.into());
        result.extend(SPACE);
        reason.into_iter().for_each(|r| result.extend::<Vec<u8>>(r.into()));
        result
    }
}


#[derive(Debug, Clone)]
pub struct Response(
    pub StatusLine,
    pub Vec<Header>,
    pub Option<ResponseBody>);

impl Response {
    pub fn ok(body:&str) -> Result<Self> {
        Ok(Response(StatusLine::ok(),
                 vec![Header::ContentType(ContentType::TextPlain),
                      Header::ContentLength(ContentLength(body.len() as u32))],
                 Some(ResponseBody(body.to_string()))))
    }
    pub fn ok_bin(body:&[u8]) -> Result<Self> {
        Ok(Response(StatusLine::ok(),
                    vec![Header::ContentType(ContentType::OctetStream),
                         Header::ContentLength(ContentLength(body.len() as u32))],
                    Some(ResponseBody(String::from_utf8(body.to_vec())?))))
    }
}
const CRLF: &[u8; 2] = b"\r\n";
const SPACE: &[u8; 1] = b" ";
impl From<Response> for Vec<u8> {
    fn from(value: Response) -> Self {
        let Response(status_line, headers, body) = value;
        let mut result: Vec<u8> = vec![];
        result.extend::<Vec<u8>>(status_line.into());
        result.extend(CRLF);

        let headers_b:Vec<u8>= headers.into_iter().rfold(vec![], |mut v:Vec<u8>,el|{
            let el:Vec<u8> = el.into();
            v.extend(el);
            v.extend(CRLF);
            v
        });
        if headers_b.is_empty(){
            result.extend(CRLF);
        } else {
            result.extend(headers_b);
        }
        result.extend(CRLF);
        body.into_iter().for_each(
            |b| {
                let v:Vec<u8> = b.into();
                result.extend(v)
            }
        );

        result
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
}



