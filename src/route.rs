use std::sync::Arc;
use crate::{HttpMethod, Response, Request, Result, AcceptEncoding, Encoding};

pub struct Router<'a> {
    routes:Vec<(&'a str,HttpMethod, BoxedHandler)>
}

impl <'a> Router <'a> {
    pub fn new() -> Self {
        Self{
            routes : Vec::new()
        }
    }

    pub fn route(&mut self, path:& 'a str, (http_method,handler):(HttpMethod, BoxedHandler)) -> & mut Self {
        self.routes.push((path.clone(), http_method, handler));
        self
    }
    pub fn find_route(&self, path:&str, http_method: HttpMethod) -> Option<BoxedHandler> {
        self.routes.clone().iter().find_map(|(p, hm, h)|{
            if *hm==http_method && path.contains(p) {
                Some(h.clone())
            } else {
                None
            }
        })
    }
}

pub type BoxedHandler = Arc<dyn Handler>;



pub struct Map<F,G> {
    f:F,
    g:G
}


impl <F,G>Handler2 for Map<F, G>
where
    F:Handler2,
    G:FnMut(&Response) -> Result<Response>
{
    fn handle(&mut self, r: &Request) -> Result<Response> {
        let res = self.f.handle(r)?;
        (self.g)(&res.clone())
    }
}

pub trait Handler2 : Sized{
    fn handle(&mut self, r:&Request) -> Result<Response>;
    fn map<A,B, G>(self,g:G) -> Map<Self, G> {
        Map{
            f:self,
            g
        }
    }

}

struct Gzip();
impl Handler2 for Gzip {
    fn handle(&mut self, r: &Request) -> Result<Response> {
        /*let Some(AcceptEncoding(Encoding::Gzip)) = r.headers().accept_encoding() {

        }*/
        todo!()
    }
}



pub trait Handler: Send + Sync + 'static{
    fn handle(&self, r:&Request) -> Result<Response>;

}

impl <F> Handler for F
where
    F:Fn(&Request) -> Result<Response> + Send + Sync + 'static,
{
    fn handle(&self, r: &Request) -> Result<Response> {
        self(r)
    }
}

pub fn get(handler: BoxedHandler)-> (HttpMethod, BoxedHandler) {
    (HttpMethod::Get, handler)
}

pub fn post(handler: BoxedHandler)-> (HttpMethod, BoxedHandler) {
    (HttpMethod::Post, handler)
}
