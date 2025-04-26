use std::fmt::Debug;
use std::process::Output;
use derive_more::{Deref, From};
use crate::{Error, HttpMethod, Request, Response, Result};



#[derive(Debug, Clone)]
struct New();
#[derive(Debug, Clone)]
struct Incomplete<'a>(&'a Request);
#[derive(Debug, Clone)]
struct Complete<'a>(&'a Request, Response);
#[derive(Debug, Clone)]
enum State<'a> {
    //New(New),
    Incomplete(Incomplete<'a>),
    Complete(Complete<'a>)
}

impl <'a>State<'a> {
    fn incomplete(r:&'a Request) -> State {
        Self::Incomplete(Incomplete(r))
    }
    fn request(&self) -> &Request {
        match self {
            State::Incomplete(Incomplete(r)) => r,
            State::Complete(Complete(r, _)) => r
        }
    }
}


pub trait Handler2 {
    type Output<'b>:Debug;
    fn handle<'a,'b>(&self, r: &'a State) -> Result<(State<'b>, Self::Output<'a>)>;
    fn map<'a, F, O2>(self,f:F) -> Map<Self, F>
    where
        F:Fn(Self::Output<'a>) -> O2,
        Self:Sized,
    {
        Map{
            g:self,
            f
        }
    }

   /* fn request(self) -> Req<Self> where
        Self:Sized,
    {
        Req{
            h:self
        }
    }*/
   /* fn request2<'a>(self) -> impl Handler2 +'a
    where Self: Sized
    {
        type Output<'a> = &'a Request;
        self.map(|v| v.request())

    }*/
}



struct Map<G,F>{
    g:G,
    f:F,
}

impl <G,F, O2> Handler2 for Map<G, F>
where

    O2:Debug,
    G:Handler2,
    for <'b> F:Fn(G::Output<'b>) -> O2,
   // for <'b> F::Output<'b>:Debug
{
    type Output<'a> = O2;
    fn handle<'a, 'b>(&self, r: &'a State) -> Result<(State<'b>, Self::Output<'a>)> {
        let (state, o) = self.g.handle(r)?;
        Ok((state, (self.f)(o)))
    }
}

struct Req<H>{
    h:H
}

/*impl <H> Handler2 for Req<H>
    where
    H:Handler2
{
    type Output<'b> = &'b Request;

    fn handle<'a>(&self, r: &'a State<'a>) -> Result<(&'a State<'a>, Self::Output<'a>)> {
        self.h.map(|v|v.request())
    }
}

*/
struct FlatMap<H,F>{
    h:H,
    f:F
}

impl <H, F, HH>Handler2 for FlatMap<H,F>
where
    H:Handler2,
    HH:Handler2,
    for <'a> F:Fn(H::Output<'a>) -> HH,
    for <'a> HH:Handler2<Output<'a> = H::Output<'a>>,
{
    type Output<'a> = HH::Output<'a>;

    fn handle<'a, 'b>(&self, r: State) -> Result<(State<'b>, Self::Output<'a>)> {
       let (state, o) = self.h.handle(r)?;
        let hh = (self.f)(o);
        hh.handle(&state)
    }
}

struct Get <'a, F>{
    f:F,
    path:&'a str

}
impl <'a, F>Handler2 for Get<'a, F>
where
    F:Fn(&Request) -> Result<Response> {
    type Output<'b> = ();
    fn handle<'b, 'c>(&self, s: &'b State) -> Result<(State<'c>, Self::Output<'b>)> {
        match s.request().http_method() {
            HttpMethod::Get if s.request().get_route().as_str() == self.path => {
                match s {
                    State::Incomplete(_) => todo!(),
                    State::Complete(v) => Ok((State::Complete((*v).clone()),()))
                }
             //   let res = (&self.f)(s.request())?;


            }
            _ => Err(Error::CantHandle)
        }
    }
}
fn get<'a, F>(path:&'a str, f:F)-> impl Handler2 +'a
    where
        F:Fn(&Request) -> Result<Response> +'a
{
    Get{
        f,
        path
    }
}



#[cfg(test)]
mod test {
    use crate::parse_request;
    use super::*;

    #[test]
    fn test_get() -> Result<()> {
        let req = b"GET / HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";
        let req = parse_request(req)?;
        let state = State::incomplete(&req);
        let g = get("/", |r| Response::ok("ok"));

        let res = g.handle(&state)?;
        println!("{:?}",res);

        Ok(())
    }

}
