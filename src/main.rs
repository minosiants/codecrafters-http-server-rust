#[allow(unused_imports)]
use std::sync::Arc;

use codecrafters_http_server::{
    close_connection, gzip, lift, mk_response, not_found, ok, path, req_body, route, state,
    user_agent as get_user_agent, Endpoint, FileOps, RequestBody, Result, Serve, Server,
    StatusCode, UnitT, UserAgent,
};

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::bind("127.0.0.1:4221").await?;
    server
        .serve(Arc::new(async move |state| {
            routes().handle(state).map(|v| v.0)
        }))
        .await
}

pub fn routes() -> impl Endpoint<Output = UnitT> {
    let v = user_agent()
        .or(route::get("/echo").set_response(path().flat_map(|v| ok(v))))
        .or(get_file())
        .or(post_file())
        .or(route::get("/").set_response(ok("")))
        .or(state().set_response(not_found("")))
        .and(gzip().and(close_connection()))
        .unit();

    v
}
fn user_agent() -> impl Endpoint<Output = UnitT> {
    let response = |v: Option<UserAgent>| ok(v.map(|v| v.0).unwrap_or("".to_string()));

    route::get("/user-agent").set_response(get_user_agent().flat_map(response))
}

fn get_file() -> impl Endpoint<Output = UnitT> {
    let read = |file: String| {
        lift(
            match format!("/tmp/data/codecrafters.io/http-server-tester/{}", file)
                .as_str()
                .read()
            {
                Ok(data) => mk_response(data, StatusCode::SC200),
                Err(_) => mk_response("", StatusCode::SC404),
            },
        )
    };
    route::get("/files").set_response(path().flat_map(read))
}
fn post_file() -> impl Endpoint<Output = UnitT> {
    let response = |(file, body): (String, Option<RequestBody>)| {
        lift(
            match format!("/tmp/data/codecrafters.io/http-server-tester/{}", file)
                .as_str()
                .write(body.unwrap().0)
            {
                Ok(_) => mk_response("", StatusCode::SC201),
                Err(_) => mk_response("", StatusCode::SC404),
            },
        )
    };
    route::post("/files").set_response(path().and(req_body()).flat_map(response))
}
