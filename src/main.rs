use codecrafters_http_server::{
    get, post, AcceptEncoding, Context, Encoding, Header, Request, Response, Result, Router,
    StatusLine,
};
use std::fs::File;
use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        thread::spawn(move || {
            match stream {
                Ok(mut stream) => {
                    let request = Request::read(&mut stream).unwrap();

                    let mut router = Router::new();
                    let router = router
                        .route(
                            "/",
                            get(Arc::new(|_: &Request| {
                                Ok(Response(StatusLine::ok(), vec![], None))
                            })),
                        )
                        .route(
                            "/user-agent",
                            get(Arc::new(|request: &Request| {
                                Response::ok(
                                    request
                                        .user_agent()
                                        .map(|v| v.0)
                                        .get_or_insert("".to_string()),
                                )
                            })),
                        )
                        .route(
                            "/echo",
                            get(Arc::new(|request: &Request| {
                                Response::ok(request.get_path().as_str())
                            })),
                        )
                        .route(
                            "/files",
                            get(Arc::new(|request: &Request| {
                                let file = format!(
                                    "/tmp/data/codecrafters.io/http-server-tester/{}",
                                    request.get_path()
                                );
                                match read(file.as_ref()) {
                                    Ok(data) => Response::ok_bin(data.as_slice()),
                                    Err(_) => Ok(Response(StatusLine::not_found(), vec![], None)),
                                }
                            })),
                        )
                        .route(
                            "/files",
                            post(Arc::new(|request: &Request| {
                                let file = format!(
                                    "/tmp/data/codecrafters.io/http-server-tester/{}",
                                    request.get_path()
                                );
                                match write(file.as_ref(), request.body().unwrap().into()) {
                                    Ok(_) => Ok(Response(StatusLine::created(), vec![], None)),
                                    Err(_) => Ok(Response(StatusLine::created(), vec![], None)),
                                }
                            })),
                        );

                    println!("request: {:?}", &request);
                    let response: Result<Vec<u8>> = match router
                        .find_route(request.get_route().as_ref(), request.http_method())
                    {
                        None => Ok(Response(StatusLine::not_found(), vec![], None).into()),
                        Some(handler) => {
                            let h = Arc::clone(&handler);
                            let mut r = h.handle(&request).unwrap();
                            if let Some(AcceptEncoding(Encoding::Gzip)) =
                                request.headers().accept_encoding()
                            {
                                Ok((r
                                    .add_header(Header::content_encoding(Encoding::Gzip))
                                    .clone())
                                .into())
                            } else {
                                println!("response: {:?}", r);
                                Ok(r.into())
                            }
                        }
                    };

                    stream.write(response.unwrap().as_ref()).unwrap();
                    //stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        });
        ()
    }
}

pub fn read(path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(path).context("Open file {path}")?; // Open the file
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Reading file")?; // Read file contents into buffer
    Ok(buffer)
}

pub fn write(path: &str, data: Vec<u8>) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&data).context("write to file")
}
