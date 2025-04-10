use crate::{HttpMethod, Request, Response, Result};
use std::sync::Arc;

pub struct Router<'a> {
    routes: Vec<(&'a str, HttpMethod, BoxedHandler)>,
}

impl<'a> Default for Router<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Router<'a> {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn route(
        &mut self,
        path: &'a str,
        (http_method, handler): (HttpMethod, BoxedHandler),
    ) -> &mut Self {
        self.routes.push((path, http_method, handler));
        self
    }
    pub fn find_route(&self, path: &str, http_method: HttpMethod) -> Option<BoxedHandler> {
        self.routes.clone().iter().find_map(|(p, hm, h)| {
            if *hm == http_method && p.contains(path) {
                Some(h.clone())
            } else {
                None
            }
        })
    }
}

pub type BoxedHandler = Arc<dyn Handler>;

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, r: &Request) -> Result<Response>;
}

impl<F> Handler for F
where
    F: Fn(&Request) -> Result<Response> + Send + Sync + 'static,
{
    fn handle(&self, r: &Request) -> Result<Response> {
        self(r)
    }
}

pub fn get(handler: BoxedHandler) -> (HttpMethod, BoxedHandler) {
    (HttpMethod::Get, handler)
}

pub fn post(handler: BoxedHandler) -> (HttpMethod, BoxedHandler) {
    (HttpMethod::Post, handler)
}
