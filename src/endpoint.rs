use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt::Debug;
use std::io::Write;


use crate::{
    AcceptEncoding, Connection, ContentType, Context, Encoding, Error,
    Header, Headers, HttpMethod, Request, RequestBody, Response, ResponseBody, Result, StatusCode,
    StatusLine, UserAgent,
};

#[derive(Debug, Clone)]
pub
enum Body<'a> {
    Text(&'a str),
    Bin(&'a [u8]),
    Empty,
}

impl<'a> Length for Body<'a> {
    fn len(&self) -> u32 {
        match self {
            Body::Text(v) => v.len() as u32,
            Body::Bin(v) => v.len() as u32,
            Body::Empty => 0,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct UnitT;

trait Length {
    fn len(&self) -> u32;
}

pub trait AsBody {
    fn body(&self) -> Body;
}

impl AsBody for &str {
    fn body(&self) -> Body {
        if self.len() != 0 {
            Body::Text(self)
        } else {
            Body::Empty
        }
    }
}
impl<'a> AsBody for String {
    fn body(&self) -> Body {
        if self.len() != 0 {
            Body::Text(self.as_str())
        } else {
            Body::Empty
        }
    }
}
impl AsBody for Vec<u8> {
    fn body(&self) -> Body {
        if self.len() != 0 {
            Body::Bin(self)
        } else {
            Body::Empty
        }
    }
}
impl<'a> Body<'a> {
    fn to_bin(self) -> Vec<u8> {
        match self {
            Body::Text(str) => str.as_bytes().to_vec(),
            Body::Bin(b) => b.to_vec(),
            Body::Empty => vec![],
        }
    }
}
#[derive(Debug, Clone)]
pub struct New();
#[derive(Debug, Clone)]
pub struct Incomplete(Request);
#[derive(Debug, Clone)]
pub struct Complete(pub Request, pub Response);
#[derive(Debug, Clone)]
pub enum State {
    //New(New),
    Incomplete(Incomplete),
    Complete(Complete),
}

impl State {
    pub fn incomplete(r: Request) -> State {
        Self::Incomplete(Incomplete(r))
    }
    pub fn complete(req: Request, resp: Response) -> State {
        Self::Complete(Complete(req, resp))
    }
    pub fn request(&self) -> &Request {
        match self {
            State::Incomplete(Incomplete(r)) => r,
            State::Complete(Complete(r, _)) => r,
        }
    }
    fn set_response(&self, resp: &Response) -> State {
        State::complete(self.request().clone(), resp.clone())
    }
}

pub trait Endpoint {
    type Output: Debug + Clone;
    fn handle(&self, r: State) -> Result<(State, Self::Output)>;
    fn map<F, O2>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> O2,
        Self: Sized,
    {
        Map { g: self, f }
    }
    fn map_res<F, O2>(self, f: F) -> MapRes<Self, F>
    where
        F: Fn(Self::Output) -> Result<O2>,
        Self: Sized,
    {
        MapRes { g: self, f }
    }
    fn map_op<F, O1, O2>(self, f: F) -> MapOp<Self, F>
    where
        F: Fn(O1) -> O2,
        Self: Sized,
    {
        MapOp { g: self, f }
    }
    fn flat_map<F, HH, O>(self, f: F) -> FlatMap<Self, F>
    where
        O: Debug + Clone,
        F: Fn(Self::Output) -> HH,
        HH: Endpoint<Output = O>,
        Self: Sized,
    {
        FlatMap { h: self, f }
    }
    fn flat_map_res<F, HH, O>(self, f: F) -> FlatMapRes<Self, F>
    where
        O: Debug + Clone,
        F: Fn(Self::Output) -> HH,
        HH: Endpoint<Output = O>,
        Self: Sized,
    {
        FlatMapRes { h: self, f }
    }
    fn flat_map_op<F, HH, O1, O2>(self, f: F) -> FlatMapOp<Self, F>
    where
        O1: Debug + Clone,
        O2: Debug + Clone,
        F: Fn(O1) -> HH,
        HH: Endpoint<Output = O2>,
        Self: Sized,
    {
        FlatMapOp { h: self, f }
    }
    fn stop_if<P>(self, p: P) -> StopIf<Self, P>
    where
        P: Fn(Self::Output) -> bool,
        Self: Sized,
    {
        StopIf { h: self, p }
    }

    fn and<G, O>(self, g: G) -> And<Self, G>
    where
        G: Endpoint<Output = O>,
        Self: Sized,
    {
        And { h: self, g }
    }
    fn or<G, O>(self, g: G) -> Or<Self, G>
    where
        G: Endpoint<Output = O>,
        Self: Sized,
    {
        Or { h: self, g }
    }
    fn value<O>(self, o: O) -> Value<Self, O>
    where
        Self: Sized,
    {
        Value { h: self, o }
    }
    fn unit(self) -> Unit<Self>
    where
        Self: Sized,
    {
        Unit { h: self }
    }
    fn set_response<G>(self, g: G) -> SetResponse<Self, G>
    where
        G: Endpoint<Output = Response>,
        Self: Sized,
    {
        SetResponse { h: self, g }
    }
    fn modify_response<F>(self, f: F) -> ModifyResponse<Self, F>
    where
        F: Fn(Response) -> Result<Response>,
        Self: Sized,
    {
        ModifyResponse { h: self, f }
    }
}

pub struct Map<G, F> {
    g: G,
    f: F,
}

impl<G, F, O2> Endpoint for Map<G, F>
where
    O2: Debug + Clone,
    G: Endpoint,
    F: Fn(G::Output) -> O2,
    // for <'b> F::Output<'b>:Debug
{
    type Output = O2;
    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.g.handle(r)?;
        Ok((state, (self.f)(o)))
    }
}

pub struct MapRes<G, F> {
    g: G,
    f: F,
}

impl<G, F, O2> Endpoint for MapRes<G, F>
where
    O2: Debug + Clone,
    G: Endpoint,
    F: Fn(G::Output) -> Result<O2>,
{
    type Output = O2;
    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.g.handle(r)?;
        Ok((state, (self.f)(o)?))
    }
}

pub struct MapOp<G, F> {
    g: G,
    f: F,
}

impl<G, F, O1, O2> Endpoint for MapOp<G, F>
where
    O1: Debug + Clone,
    O2: Debug + Clone,
    G: Endpoint<Output = Option<O1>>,
    F: Fn(O1) -> O2,
{
    type Output = Option<O2>;
    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.g.handle(r)?;
        match o {
            None => Ok((state, Option::<O2>::None)),
            Some(v) => Ok((state, Option::<O2>::Some((self.f)(v)))),
        }
    }
}

pub struct FlatMap<H, F> {
    h: H,
    f: F,
}

impl<H, F, HH, O, O2> Endpoint for FlatMap<H, F>
where
    O: Debug + Clone,
    O2: Debug + Clone,
    H: Endpoint<Output = O>,
    HH: Endpoint,
    F: Fn(H::Output) -> HH,
    HH: Endpoint<Output = O2>,
{
    type Output = HH::Output;

    fn handle<'a>(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.h.handle(r)?;
        let hh = (self.f)(o);
        hh.handle(state)
    }
}
pub struct FlatMapRes<H, F> {
    h: H,
    f: F,
}

impl<H, F, HH, O1, O2> Endpoint for FlatMapRes<H, F>
where
    O1: Debug + Clone,
    O2: Debug + Clone,
    H: Endpoint<Output = Result<O1>>,
    F: Fn(O1) -> HH,
    HH: Endpoint<Output = O2>,
{
    type Output = O2;

    fn handle<'a>(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.h.handle(r)?;
        let hh = (self.f)(o?);
        hh.handle(state)
    }
}

pub struct FlatMapOp<H, F> {
    h: H,
    f: F,
}

impl<H, F, HH, O, O2> Endpoint for FlatMapOp<H, F>
where
    O: Debug + Clone,
    O2: Debug + Clone,
    H: Endpoint<Output = Option<O>>,
    F: Fn(O) -> HH,
    HH: Endpoint<Output = O2>,
{
    type Output = Option<HH::Output>;

    fn handle<'a>(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.h.handle(r)?;
        match o {
            None => Ok((state, None)),
            Some(v) => {
                let (ss, o2) = (self.f)(v).handle(state)?;
                Ok((ss, Some(o2)))
            }
        }
    }
}

pub struct And<H, G> {
    h: H,
    g: G,
}

impl<H, G, O1, O2> Endpoint for And<H, G>
where
    O1: Debug + Clone,
    O2: Debug + Clone,
    H: Endpoint<Output = O1>,
    G: Endpoint<Output = O2>,
{
    type Output = (O1, O2);

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, o1) = self.h.handle(r)?;
        let (ss, o2) = self.g.handle(s)?;
        println!("and {:?}", ss);
        Ok((ss, (o1, o2)))
    }
}
pub struct Or<H, G> {
    h: H,
    g: G,
}

impl<H, G, O> Endpoint for Or<H, G>
where
    O: Debug + Clone,
    H: Endpoint<Output = O>,
    G: Endpoint<Output = O>,
{
    type Output = O;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        match self.h.handle(r.clone()) {
            Ok((s, v)) => Ok((s, v)),
            Err(_) => self.g.handle(r),
        }
    }
}
pub struct StopIf<H, P> {
    h: H,
    p: P,
}

impl<H, P> Endpoint for StopIf<H, P>
where
    H: Endpoint,
    P: Fn(H::Output) -> bool,
{
    type Output = H::Output;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, out) = self.h.handle(r)?;
        if (self.p)(out.clone()) {
            Err(Error::CantHandle)
        } else {
            Ok((s, out.clone()))
        }
    }
}
pub struct Value<H, O> {
    h: H,
    o: O,
}

impl<H, O> Endpoint for Value<H, O>
where
    H: Endpoint,
    O: Debug + Clone,
{
    type Output = O;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, _) = self.h.handle(r)?;
        Ok((s, self.o.clone()))
    }
}

pub struct Unit<H> {
    h: H,
}

impl<H, O> Endpoint for Unit<H>
where
    O: Debug + Clone,

    H: Endpoint<Output = O>,
{
    type Output = UnitT;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, _) = self.h.handle(r)?;
        Ok((s, UnitT))
    }
}

pub struct SetResponse<H, G> {
    h: H,
    g: G,
}

impl<H, G, O1> Endpoint for SetResponse<H, G>
where
    O1: Debug + Clone,
    H: Endpoint<Output = O1>,
    G: Endpoint<Output = Response>,
{
    type Output = UnitT;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, _) = self.h.handle(r)?;
        let (ss, resp) = self.g.handle(s)?;
        Ok((ss.set_response(&resp), UnitT))
    }
}

pub struct ModifyResponse<H, F> {
    h: H,
    f: F,
}

impl<H, O, F> Endpoint for ModifyResponse<H, F>
where
    O: Debug + Clone,
    H: Endpoint<Output = O>,
    F: Fn(Response) -> Result<Response>,
{
    type Output = O;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (s, o) = self.h.handle(r)?;
        let ss = match s {
            State::Incomplete(Incomplete(req)) => State::incomplete(req.clone()),
            State::Complete(Complete(req, res)) => {
                State::complete(req.clone(), (self.f)(res.clone())?)
            }
        };
        Ok((ss, o))
    }
}

pub struct S {}

impl Endpoint for S {
    type Output = State;

    fn handle(&self, s: State) -> Result<(State, Self::Output)> {
        Ok((s.clone(), s))
    }
}

pub fn state() -> impl Endpoint<Output = State> {
    S {}
}
pub fn request() -> impl Endpoint<Output = Request> {
    state().map(|v| v.request().clone())
}
pub fn req_body() -> impl Endpoint<Output = Option<RequestBody>> {
    request().map(|v| v.body)
}
pub fn user_agent() -> impl Endpoint<Output = Option<UserAgent>> {
    state().map(|v| v.request().user_agent())
}
pub fn path() -> impl Endpoint<Output = String> {
    request().map(|v| v.get_path())
}
pub fn route() -> impl Endpoint<Output = String> {
    request().map(|v| v.get_route())
}
pub fn http_method() -> impl Endpoint<Output = HttpMethod> {
    request().map(|v| v.http_method())
}
pub fn req_headers() -> impl Endpoint<Output = Headers> {
    request().map(|v| v.headers)
}

pub fn route_for(path: &str) -> impl Endpoint<Output = String> + use<'_> {
    route().stop_if(move |v| path.to_string() != v)
}
pub fn modify_response<F>(f: F) -> impl Endpoint<Output = UnitT>
where
    F: Fn(Response) -> Result<Response> + 'static,
{
    state().modify_response(move |r| f(r)).unit()
}
pub fn get() -> impl Endpoint<Output = HttpMethod> {
    http_method().stop_if(|v| !HttpMethod::is_get(&v))
}
pub fn post() -> impl Endpoint<Output = HttpMethod> {
    http_method().stop_if(|v| !HttpMethod::is_post(&v))
}
pub fn ok<'a, T>(body: T) -> impl Endpoint<Output = Response> + use<'a, T>
where
    T: AsBody,
{
    lift(mk_response(body, StatusCode::SC200))
}
pub fn not_found<'a, T>(body: T) -> impl Endpoint<Output = Response> + use<'a, T>
where
    T: AsBody,
{
    lift(mk_response(body, StatusCode::SC404))
}
pub fn mk_response<'a, H>(body: H, code: StatusCode) -> Response
where
    H: AsBody,
{
    let body = body.body();
    let ct = match body {
        Body::Text(_) | Body::Empty => ContentType::TextPlain,
        Body::Bin(_) => ContentType::OctetStream,
    };
    // let b = body.to_bin();
    let sl = match code {
        StatusCode::SC200 => StatusLine::ok(),
        StatusCode::SC201 => StatusLine::created(),
        StatusCode::SC404 => StatusLine::not_found(),
    };
    let len = body.len();
    let body = match body {
        Body::Text(_) | Body::Bin(_) => Some(ResponseBody(body.to_bin())),
        Body::Empty => None,
    };
    Response(
        sl,
        vec![Header::content_type(ct), Header::content_length(len)],
        body,
    )
}

struct Lift<T> {
    t: T,
}
impl<T> Endpoint for Lift<T>
where
    T: Debug + Clone,
{
    type Output = T;

    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        Ok((r, self.t.clone()))
    }
}
pub fn lift<T>(t: T) -> impl Endpoint<Output = T>
where
    T: Debug + Clone,
{
    Lift { t }
}

fn accept_encoding() -> impl Endpoint<Output = Option<AcceptEncoding>> {
    request().map(|v| v.headers.accept_encoding())
}
fn gzip_header() -> impl Endpoint<Output = Option<Encoding>> {
    accept_encoding().map(|v| v.and_then(|v| v.gzip()))
}
pub fn gzip() -> impl Endpoint<Output = Option<UnitT>> {
    gzip_header().flat_map_op(|_| {
        modify_response(|r| {
            r.clone()
                .add_header(Header::content_encoding(Encoding::Gzip))
                .with_body(|rb| Ok(ResponseBody(gzip_encode(rb.into())?)))
        })
    })
}
fn connection() -> impl Endpoint<Output = Option<Connection>> {
    req_headers().map(|h| h.connection())
}
pub fn close_connection() -> impl Endpoint<Output = Option<UnitT>> {
    connection().flat_map_op(|_| {
        modify_response(|r| {
            Ok(r.clone()
                .add_header(Header::connection(Connection::Close))
                .clone())
        })
    })
}
fn gzip_encode(b: &[u8]) -> Result<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(b).with_context(|| "")?;
    let res = enc.finish().with_context(|| "")?;
    Ok(res)
}

pub mod route {
    use super::*;

    pub fn get(path: &str) -> impl Endpoint<Output = UnitT> + use<'_> {
        super::get().and(route_for(path)).unit()
    }
    pub fn post(path: &str) -> impl Endpoint<Output = UnitT> + use<'_> {
        super::post().and(route_for(path)).unit()
    }
}
#[cfg(test)]
mod test {
    use crate::parse_request;

    use super::*;

    #[test]
    fn test_get() -> Result<()> {
        let req = b"GET /user-agent HTTP/1.1\r\nHost: localhost:4221\r\nConnection: close\r\n\r\n";

        let req = parse_request(req)?;
        let state = State::incomplete(req);
        let g = route::get("/user-agent")
            .set_response(ok("okkkkkk"))
            .or(route::get("/").set_response(ok("ok")))
            .and(close_connection());

        let res = g.handle(state)?;
        println!("{:?}", res.0);
        Ok(())
    }
    #[test]
    fn test_get_user_agent() -> Result<()> {
        let req = b"GET /user-agent HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let req = parse_request(req)?;
        let state = State::incomplete(req);
        let response = |v: Option<UserAgent>| ok(v.map(|v| v.0).unwrap_or("".to_string()));
        let (state, r) = route::get("/user-agent")
            .set_response(user_agent().flat_map(response))
            .handle(state)?;

        println!("{:?}", state);

        Ok(())
    }

    #[test]
    fn test_get_echo() -> Result<()> {
        let req = b"GET /echo/hello HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let req = parse_request(req)?;
        let state = State::incomplete(req);

        let (state, r) = route::get("/echo")
            .set_response(path().flat_map(|v| ok(v)))
            .handle(state)?;

        println!("{:?}", state);

        Ok(())
    }
    #[test]
    fn test_get_file() -> Result<()> {
        let req = b"GET /file/hello HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let req = parse_request(req)?;
        let state = State::incomplete(req);
        "/tmp/http-server-tester/hello"
            .create_and_write(b"hello".to_vec())
            .unwrap();
        let read = |file: String| {
            lift(
                match format!("/tmp/http-server-tester/{}", file).as_str().read() {
                    Ok(data) => mk_response(data, StatusCode::SC200),
                    Err(_) => mk_response("", StatusCode::SC404),
                },
            )
        };
        let (state, r) = route::get("/file")
            .set_response(path().flat_map(read))
            .handle(state)?;

        println!("{:?}", state);

        Ok(())
    }

}
