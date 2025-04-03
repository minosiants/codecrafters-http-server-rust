use std::io::{BufReader, Read};
use std::net::TcpStream;
use derive_more::{Deref, From, Into};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::{is_not, take_until};
use nom::character::complete::{crlf, space1, u32};
use nom::combinator::{map, map_parser, rest};
use nom::{IResult, Parser};
use nom::multi::many0;
use nom::number::complete::be_u32;
use regex::Regex;
use crate::{Error, Header, ResponseBody, UserAgent, Result, ContentType, Headers, ContentLength, HttpMethod, Reason};


#[derive(Debug, Clone, PartialEq, From, Deref)]
pub struct RequestTarget(String);

impl RequestTarget {
    pub fn start_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}
#[derive(Debug, Clone, PartialEq)]
struct RequestLine(crate::types::HttpMethod, RequestTarget, crate::types::HttpVersion);

fn parse_http_method(input: &[u8]) -> IResult<&[u8], crate::types::HttpMethod> {
    alt((
        map(tag(&b"GET"[..]), |_| crate::types::HttpMethod::Get),
        map(tag(&b"POST"[..]), |_| crate::types::HttpMethod::Post)
    )).parse(input)
}

fn parse_target(input: &[u8]) -> IResult<&[u8], RequestTarget> {
    is_not(&b" "[..]).map_res(|v: &[u8]| String::from_utf8(v.to_vec())).map(|v| RequestTarget(v)).parse(input)
}
fn parse_http_version(input: &[u8]) -> IResult<&[u8], crate::types::HttpVersion> {
    map(
        take_until(&b"\r\n"[..]),
        |_| crate::types::HttpVersion::HttpOne,
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
    fn to_string(mut f: impl FnMut(String) -> Result<Header>) -> impl FnMut((&[u8], &[u8])) -> Result<Header> {
        move |(_, b)|
        f(String::from_utf8(b.to_vec())?)
    }
    let host = (tag(&b"Host: "[..]), rest)
        .map_res(to_string(|v| Result::Ok(Header::host(v.as_str()))));

    let user_agent = (tag(&b"User-Agent: "[..]), rest)
        .map_res(to_string(|v| Ok(Header::user_agent(v.as_str()))));

    let accept = (tag(&b"Accept: "[..]), rest).map_res(to_string(|v| Ok(Header::accept(v.as_str()))));
    let content_type = (tag(&b"Content-Type: "[..]),rest).map_res(
        to_string(|v| ContentType::from2(v.as_str()).map(Header::content_type))
    );
    let content_length = (tag(&b"Content-Length: "[..]), rest)
        .map_res(to_string(|v| Result::Ok(Header::content_length(v.parse().unwrap()))));


    many0(
        map(
            (map_parser(
                take_until(&b"\r\n"[..]),
                alt((host,
                     user_agent,
                     accept,
                     content_type,
                     content_length)
                )),
             tag(&b"\r\n"[..])
            ),
            |(a, _)| a,
        )
    ).parse(input)
}

#[derive(Debug, Clone, From, Deref, Into)]
pub struct RequestBody(Vec<u8>);

#[derive(Debug, Clone)]
pub struct Request {
    request_line: RequestLine,
    headers: Headers,
    body: Option<RequestBody>,
}

impl Request {
    pub fn http_method(&self) -> HttpMethod {
        self.request_line.0
    }
    pub fn target(&self) -> RequestTarget {
        self.request_line.1.clone()
    }
    pub fn user_agent(&self) -> Option<UserAgent> {
        self.headers.user_agent()
    }
    pub fn get_path(&self) -> String {
        let p = self.target().0.clone();
        let req_path = Regex::new(r"^/\w+/(?<path>\w+)").unwrap();

        let r = req_path.captures(p.as_str()).unwrap();
        r["path"].to_string()
    }
    pub fn body(&self) -> Option<RequestBody> {
        self.body.clone()
    }
    pub fn read(stream: &mut TcpStream) -> Result<Self> {
        let input = Self::read_request(stream)?;
        println!("input size {:?}", input.len());
        fn read_body(cl: Option<ContentLength>, s:&mut TcpStream) -> Option<RequestBody> {
            let content_length  = cl?;
            println!("content length: {:?}", content_length);
            let mut buf = vec![0; *content_length as usize];
            println!("buf size {:?}", buf.len());
            match s.read_exact(&mut buf) {
                Ok(_) => {

                    Some(RequestBody(buf))
                }
                Err(e) => {
                    println!("problem to read {:?}", e);
                    None
                }
            }

        }
        let res = map(
            (parse_request_line,
             crlf,
             parse_headers,
             crlf,
             rest
            ),
            |(request_line, _, headers, _, body)| {
                let h:Headers = headers.into();
                let cl = h.content_length();
                Request { request_line, headers:h, body: Some(RequestBody(body.to_vec())) }
            },
        ).parse(&input);
        match res {
            Ok((rest, request)) => Ok(request),
            Err(e) => {
                println!("Error {:?}", e);
                Err(Error::GeneralError("Parser error".to_string()))
            }
        }
    }
    fn read_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
        let mut reader = BufReader::new(stream);
        let mut req = Vec::new();
        let mut buffer = [0; 1024];
        let mut total_read = 0;

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break; // EOF
            }
            req.extend_from_slice(&buffer[..n]);
            total_read += n;

            // Check for end of headers
            if req.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        Ok(req)
    }
}




#[cfg(test)]
mod tests {
    use nom::character::complete::crlf;

    use super::*;

    #[test]
    fn test_decode_request_get() -> crate::Result<()> {
        let req = b"GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let result =
            map(
                (parse_request_line,
                 crlf,
                 parse_headers,
                 crlf
                ),
                |(request_line, _, headers, _)|
                Request { request_line, headers:headers.into(), body: None },
            ).parse(req);

        println!("resel {:?}", result);
        Ok(())
    }

    #[test]
    fn test_decod_request_post() -> Result<()> {

        let req = b"POST /files/orange_pear_banana_strawberry HTTP/1.1\r\nHost: localhost:4221\r\nContent-Length: 63\r\nContent-Type: application/octet-stream\r\n\r\n";
        let result =
            map(
                (parse_request_line,
                 crlf,
                 parse_headers,
                 crlf
                ),
                |(request_line, _, headers, _)|
                Request { request_line, headers:headers.into(), body: None },
            ).parse(req);
        println!("resel {:?}", result);
        Ok(())
    }
    #[test]
    fn bla() -> Result<()> {
        let l = b"POST /files/orange_pear_orange_mango HTTP/1.1\r\nHost: localhost:4221\r\nContent-Length: 62\r\nContent-Type: application/octet-stream\r\n\r\nstrawberry banana mango apple orange pineapple apple raspberry";

    }
}

