#[allow(unused_imports)]
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use codecrafters_http_server::{
    close_connection, gzip, lift, mk_response, not_found, ok, path, req_body, route, state,
    user_agent as get_user_agent, Complete, Connection, Context, Endpoint, FileOps, Request,
    RequestBody, Result, State, StatusCode, UnitT, UserAgent,
};

#[tokio::main]
async fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221")
        .await
        .with_context(|| "")?;
    loop {
        let (stream, _) = listener.accept().await.with_context(|| "")?;
        tokio::spawn(handle(stream));
    }
}

async fn handle(mut stream: TcpStream) -> Result<()> {
    loop {
        let request = Request::read(&mut stream).await?;
        let state = routes().handle(State::incomplete(request))?.0;
        match state {
            State::Incomplete(_) => {}
            State::Complete(Complete(req, resp)) => {
                let bytes: Vec<u8> = resp.into();
                stream.write_all(bytes.as_ref()).await.with_context(|| "")?;
                stream.flush().await.with_context(|| "flushing ")?;
                if req.headers.connection() == Some(Connection::Close) {
                    break;
                }
            }
        }
    }
    Ok(())
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
    let response = |v: Option<UserAgent>| {
        println!("user agent: {:?}", v);
        ok(v.map(|v| v.0).unwrap_or("".to_string()))
    };
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

#[cfg(test)]
mod test {
    use super::*;
    use codecrafters_http_server::parse_request;

    #[test]
    fn test_echo() -> Result<()> {
        let req = b"GET /user-agent HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: pear/raspberry-raspberry\r\n\r\n";
        let state = State::incomplete(parse_request(req)?);
        let (s, _) = routes().handle(state)?;
        println!("state: {:?}", s);
        Ok(())
    }
}
