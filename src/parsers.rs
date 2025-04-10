use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::bytes::{is_not, take_until};
use nom::character::complete::{crlf, space0, space1};
use nom::combinator::{map, map_parser, rest};
use nom::multi::{many0, separated_list0};
use nom::sequence::{preceded, terminated};
use nom::{IResult, Parser};

use crate::{
    ContentType, Encoding, Error, Header, Headers, HttpMethod, Request, RequestBody, RequestLine,
    RequestTarget, Result,
};

fn parse_http_method(input: &[u8]) -> IResult<&[u8], crate::types::HttpMethod> {
    alt((
        map(tag(&b"GET"[..]), |_| HttpMethod::Get),
        map(tag(&b"POST"[..]), |_| HttpMethod::Post),
    ))
    .parse(input)
}

fn parse_target(input: &[u8]) -> IResult<&[u8], RequestTarget> {
    is_not(&b" "[..])
        .map_res(|v: &[u8]| String::from_utf8(v.to_vec()))
        .map(RequestTarget)
        .parse(input)
}
fn parse_http_version(input: &[u8]) -> IResult<&[u8], crate::types::HttpVersion> {
    map(take_until(&b"\r\n"[..]), |_| {
        crate::types::HttpVersion::HttpOne
    })
    .parse(input)
}
fn parse_request_line(input: &[u8]) -> IResult<&[u8], RequestLine> {
    (
        parse_http_method,
        space1,
        parse_target,
        space1,
        parse_http_version,
    )
        .parse(input)
        .map(|(rest, (m, _, t, _, v))| (rest, RequestLine(m, t, v)))
}

fn parse_headers(input: &[u8]) -> IResult<&[u8], Vec<Header>> {
    fn to_string(
        mut f: impl FnMut(String) -> Result<Option<Header>>,
    ) -> impl FnMut((&[u8], &[u8])) -> Result<Option<Header>> {
        move |(_, b)| f(String::from_utf8(b.to_vec())?)
    }
    let host =
        (tag(&b"Host: "[..]), rest).map_res(to_string(|v| Ok(Some(Header::host(v.as_str())))));

    let user_agent = (tag(&b"User-Agent: "[..]), rest)
        .map_res(to_string(|v| Ok(Some(Header::user_agent(v.as_str())))));

    let accept =
        (tag(&b"Accept: "[..]), rest).map_res(to_string(|v| Ok(Some(Header::accept(v.as_str())))));
    let content_type = (tag(&b"Content-Type: "[..]), rest).map_res(to_string(|v| {
        ContentType::from2(v.as_str()).map(|v| Some(Header::content_type(v)))
    }));
    let content_length = (tag(&b"Content-Length: "[..]), rest)
        .map_res(to_string(|v| Ok(Some(Header::content_length(v.parse()?)))));

    let content_encoding = (tag(&b"Content-Encoding: "[..]), rest).map_res(to_string(|v| {
        Ok(Some(Header::content_encoding(Encoding::from(v.as_str())?)))
    }));

    let encoding = map(
        separated_list0(
            tag(","),
            preceded(space0, terminated(take_while(|c: u8| c != b','), space0)),
        ),
        |v: Vec<&[u8]>| {
            v.into_iter()
                .flat_map(|enc| match String::from_utf8(enc.to_vec()) {
                    Ok(s) => Encoding::from(s.as_str()).ok(),
                    Err(_) => None,
                })
        },
    );

    let accept_encoding = (tag(&b"Accept-Encoding: "[..]), encoding).map_res(|(_, b)| {
        let bb: Vec<Encoding> = b.collect();
        if !bb.is_empty() {
            Ok::<Option<Header>, Error>(Some(Header::accept_encoding(&bb)))
        } else {
            Ok(None)
        }
    });

    let (input, headers) = many0(map(
        (
            map_parser(
                take_until(&b"\r\n"[..]),
                alt((
                    accept_encoding,
                    host,
                    user_agent,
                    accept,
                    content_type,
                    content_length,
                    content_encoding,
                )),
            ),
            tag(&b"\r\n"[..]),
        ),
        |(a, _)| a,
    ))
    .parse(input)?;
    Ok((input, headers.into_iter().flatten().collect()))
}

pub fn parse_request(input: &[u8]) -> Result<Request> {
    let res = map(
        (parse_request_line, crlf, parse_headers, crlf, rest),
        |(request_line, _, headers, _, body)| {
            let h: Headers = headers.into();
            Request {
                request_line,
                headers: h,
                body: Some(RequestBody(body.to_vec())),
            }
        },
    )
    .parse(input);
    match res {
        Ok((_, request)) => Ok(request),
        Err(e) => {
            println!("Error {:?}", e);
            Err(Error::GeneralError("Parser error".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use nom::character::complete::crlf;

    use super::*;

    #[test]
    fn test_decode_request_get() -> crate::Result<()> {
        let req = b"GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let result = map(
            (parse_request_line, crlf, parse_headers, crlf),
            |(request_line, _, headers, _)| Request {
                request_line,
                headers: headers.into(),
                body: None,
            },
        )
        .parse(req);

        println!("resel {:?}", result);
        Ok(())
    }

    #[test]
    fn test_decod_request_post() -> Result<()> {
        let req = b"POST /files/orange_pear_banana_strawberry HTTP/1.1\r\nHost: localhost:4221\r\nContent-Length: 63\r\nContent-Type: application/octet-stream\r\n\r\n";
        let result = map(
            (parse_request_line, crlf, parse_headers, crlf),
            |(request_line, _, headers, _)| Request {
                request_line,
                headers: headers.into(),
                body: None,
            },
        )
        .parse(req);
        println!("resel {:?}", result);
        Ok(())
    }
    #[test]
    fn test_accept_encoding() -> Result<()> {
        //  let req = b"GET /echo/grape HTTP/1.1\r\nHost: localhost:4221\r\nAccept-Encoding: invalid-encoding\r\n\r\n";
        let req = b"GET /echo/pineapple HTTP/1.1\r\nHost: localhost:4221\r\nAccept-Encoding: encoding-1, gzip, encoding-2\r\n\r\n";
        let result = map(
            (parse_request_line, crlf, parse_headers, crlf),
            |(request_line, _, headers, _)| Request {
                request_line,
                headers: headers.into(),
                body: None,
            },
        )
        .parse(req);
        println!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().1.get_route(), "/echo".to_string());

        Ok(())
    }
}
