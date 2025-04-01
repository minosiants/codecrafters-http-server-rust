use std::string::FromUtf8Error;
use bytes::BufMut;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, map_parser, map_res, rest, value};
use nom::{IResult, Parser};
use nom::bytes::{is_not, take_until, take_while};
use nom::character::complete::{crlf, space1};
use nom::character::streaming::anychar;
use nom::multi::many0;
use crate::{Error, Result};
#[derive(Debug, Copy, Clone, PartialEq)]
enum HttpVersion {
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
    SC404

}
impl From<StatusCode> for Vec<u8> {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::SC200 => b"200".to_vec(),
            StatusCode::SC404 => b"404".to_vec()
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub enum Reason {
    Ok,
    NotFound
}

impl From<Reason> for Vec<u8> {
    fn from(value: Reason) -> Self {
        match value {
            Reason::Ok => b"OK".to_vec(),
            Reason::NotFound => b"Not Found".to_vec()
        }
    }
}

#[derive(Debug, Clone)]
struct Host(String);
#[derive(Debug, Clone)]
struct UserAgent(String);
#[derive(Debug, Clone)]
struct Accept(String);

#[derive(Debug, Clone)]
pub enum Header {
    Host(Host),
    UserAgent(UserAgent),
    Accept(Accept),
}
#[derive(Debug, Clone, Copy)]
pub struct ResponseBody {}

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


const CRLF: &[u8; 2] = b"\r\n";
const SPACE: &[u8; 1] = b" ";
impl From<Response> for Vec<u8> {
    fn from(value: Response) -> Self {
        let Response(status_line, headers, body) = value;
        let mut result: Vec<u8> = vec![];
        result.extend::<Vec<u8>>(status_line.into());
        result.extend(CRLF);
        result.extend(CRLF);
        result
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    request_line: RequestLine,
    headers:Vec<Header>
}

impl Request {
    pub fn target(&self) -> RequestTarget {
        self.request_line.1.clone()
    }
    pub fn parse(input:&[u8]) -> Result<Self> {
        let res = map(
            (parse_request_line,
             crlf,
             parse_headers,
             crlf
            ),
            |(request_line, _, headers, _)|
            Request{request_line, headers}
        ).parse(input);
        match res {
            Ok((_,request)) => Ok(request),
            Err(e) => {
                println!("Error {:?}", e);
                Err(Error::GeneralError("Parser error".to_string()))
            }
        }

    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HttpMethod {
    Get,
    Post,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RequestTarget(pub String);
#[derive(Debug, Clone, PartialEq)]
struct RequestLine(HttpMethod, RequestTarget, HttpVersion);

fn parse_http_method(input: &[u8]) -> IResult<&[u8], HttpMethod> {
    alt((
        map(tag(&b"GET"[..]), |_| HttpMethod::Get),
        map(tag(&b"POST"[..]), |_| HttpMethod::Post)
    )).parse(input)
}

fn parse_target(input: &[u8]) -> IResult<&[u8], RequestTarget> {
    is_not(&b" "[..]).map_res(|v: &[u8]| String::from_utf8(v.to_vec())).map(|v| RequestTarget(v)).parse(input)
}
fn parse_http_version(input: &[u8]) -> IResult<&[u8], HttpVersion> {
    map(
        take_until(&b"\r\n"[..]),
        |_| HttpVersion::HttpOne,
    ).parse(input)
}
fn parse_request_line(input: &[u8]) -> IResult<&[u8], RequestLine> {
    (parse_http_method,
     space1,
     parse_target,
     space1,
     parse_http_version)
        .parse(input)
        .map(|(rest, (m, _, t, _, v))| (rest, RequestLine(m, t, v)))
}

fn parse_headers(input: &[u8]) -> IResult<&[u8], Vec<Header>> {
    let str = String::from_utf8(input.to_vec()).unwrap();
    println!("str: {:?}", str);
    fn to_string<T>(mut f: impl FnMut(String) -> T) -> impl FnMut((&[u8], &[u8])) -> core::result::Result<T, FromUtf8Error> {
        move |(_, b)|
        Ok(f(String::from_utf8(b.to_vec())?))
    }
    let host = (tag(&b"Host: "[..]), rest)
        .map_res(to_string(|v| Header::Host(Host(v))));

    let user_agent = (tag(&b"User-Agent: "[..]), rest)
        .map_res(to_string(|v| Header::UserAgent(UserAgent(v))));

    let accept = (tag(&b"Accept: "[..]), rest).map_res(to_string(|v| Header::Accept(Accept(v))));
    many0(
        map(
            (map_parser(
                take_until(&b"\r\n"[..]),
                alt((host,
                     user_agent,
                     accept)
                )),
             tag(&b"\r\n"[..])
            ),
            |(a, _)| a,
        )
    ).parse(input)
}


#[cfg(test)]
mod tests {
    use nom::character::complete::crlf;
    use super::*;


    #[test]
    fn test_decode_request() -> Result<()> {
        let req = b"GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let result =
            map(
                (parse_request_line,
             crlf,
             parse_headers,
             crlf
            ),
                |(request_line, _, headers, _)|
                Request{request_line, headers}
            ).parse(req);

        println!("resel {:?}", result);
        Ok(())
    }
}