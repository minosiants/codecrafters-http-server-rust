use bytes::BufMut;

#[derive(Debug, Copy, Clone)]
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
    SC200
}
impl From<StatusCode> for Vec<u8> {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::SC200 => b"200".to_vec()
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub enum Reason {
    Ok
}

impl From<Reason> for Vec<u8> {
    fn from(value: Reason) -> Self {
        match value {
            Reason::Ok => b"OK".to_vec()
        }

    }
}

#[derive(Debug, Clone, Copy)]
pub enum Header {

}
#[derive(Debug, Clone, Copy)]
pub struct ResponseBody{

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
            Some(Reason::Ok)
        )
    }
}

impl From<StatusLine> for Vec<u8> {
    fn from(value: StatusLine) -> Self {
        let StatusLine(http_version, status_code, reason) = value;
        let mut result:Vec<u8> = vec![];
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


const  CRLF: &[u8; 2] = b"\r\n";
const SPACE: &[u8; 1] = b" ";
impl From<Response> for Vec<u8> {
    fn from(value: Response) -> Self {
        let Response(status_line, headers, body) = value;
        let mut result:Vec<u8> = vec![];
        result.extend::<Vec<u8>>(status_line.into());
        result.extend(CRLF);
        result.extend(CRLF);
        result
    }
}
