use crate::{
    parse_request, Connection, Context, Headers, HttpMethod, HttpVersion, Result, UserAgent,
};
use bytes::{Bytes, BytesMut};
use derive_more::{Deref, From, Into};
use regex::Regex;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

#[derive(Debug, Clone, PartialEq, From, Deref)]
pub struct RequestTarget(pub String);

impl RequestTarget {
    pub fn start_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct RequestLine(pub HttpMethod, pub RequestTarget, pub HttpVersion);

#[derive(Debug, Clone, From, Deref, Into)]
pub struct RequestBody(pub Bytes);

#[derive(Debug, Clone)]
pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Option<RequestBody>,
}

impl Request
where
    Self: Sync + Send,
{
    pub fn http_method(&self) -> HttpMethod {
        self.request_line.0
    }
    pub fn target(&self) -> RequestTarget {
        self.request_line.1.clone()
    }
    pub fn user_agent(&self) -> Option<UserAgent> {
        self.headers.user_agent()
    }
    pub fn connection(&self) -> Option<Connection> {
        self.headers.connection()
    }

    fn split_target(&self) -> (String, String) {
        let p = self.target().0.clone();
        let req_path = Regex::new(r"^(?<route>/[^/]+)((?:/)(?<path>[^/]+))?$").unwrap();
        //   let req_path = Regex::new(r"^(?<route>/[^/]+)(?:/)(?<path>[^/]+)$").unwrap();

        match req_path.captures(p.as_str()) {
            None => ("/".to_string(), "".to_string()),
            Some(r) => (
                r.name("route")
                    .map(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                r.name("path").map(|v| v.as_str()).unwrap_or("").to_string(),
            ),
        }
    }
    pub fn get_path(&self) -> String {
        self.split_target().1
    }
    pub fn get_route(&self) -> String {
        self.split_target().0
    }
    pub fn body(&self) -> Option<RequestBody> {
        self.body.clone()
    }
    pub fn headers(&self) -> &Headers {
        &self.headers
    }
    pub async fn read(stream: &mut TcpStream) -> Result<Self> {
        let input = Self::read_request(stream).await.with_context(|| "");
        parse_request(&input?)
    }
    async fn read_request(stream: &mut TcpStream) -> std::io::Result<Bytes> {
        let mut req = BytesMut::new();
        let mut buffer = [0; 1024];
        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break; // EOF
            }
            req.extend_from_slice(&buffer[..n]);
            if req.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        Ok(req.freeze())
    }
}
