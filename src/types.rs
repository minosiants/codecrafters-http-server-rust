use std::string::FromUtf8Error;
use bytes::BufMut;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, map_parser, map_res, rest, value};
use nom::{AsBytes, IResult, Parser};
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
pub struct UserAgent(pub String);
#[derive(Debug, Clone)]
struct Accept(String);
#[derive(Debug, Clone)]
pub enum ContentType {
    TextPlain
}

impl From<ContentType> for Vec<u8> {
    fn from(value: ContentType) -> Self {
        match value {
            ContentType::TextPlain => b"text/plain".to_vec()
        }
    }
}
#[derive(Debug, Clone)]
pub struct ContentLength(pub u32);

#[derive(Debug, Clone)]
pub enum Header {
    Host(Host),
    UserAgent(UserAgent),
    Accept(Accept),
    ContentType(ContentType),
    ContentLength(ContentLength)
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

#[derive(Debug, Clone)]
pub struct Request {
    request_line: RequestLine,
    headers:Vec<Header>
}

impl Request {
    pub fn target(&self) -> RequestTarget {
        self.request_line.1.clone()
    }
    pub fn user_agent(&self) -> Option<UserAgent> {
        self.headers.clone().into_iter().find_map(|v| match v {
            Header::UserAgent(v) => Some(v.clone()),
            _ => None
        })
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