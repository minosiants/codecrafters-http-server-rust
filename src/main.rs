use std::fs::File;
use std::io::{BufReader, Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::thread::Thread;
use codecrafters_http_server::{ContentType, Header, Result, Request, RequestTarget, Response, StatusLine, ContentLength, ResponseBody, Context};
use regex::Regex;

fn read_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut reader = BufReader::new(stream);
    let mut req = Vec::new();
    let mut buffer = [0; 1024];
    let mut total_read = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break; // EOF
        }
        req.extend_from_slice(&buffer[..n]);
        total_read += n;

        // Check for end of headers
        if req.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    Ok(req)
}

fn main() {

    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        thread::spawn( move ||
        match stream {

            Ok(mut stream) => {

                fn get_path(v:&str) -> String {
                    let req_path = Regex::new(r"^/\w+/(?<path>\w+)").unwrap();
                    let r = req_path.captures(v).unwrap();
                    r["path"].to_string()
                }

                let req = read_request(&mut stream).unwrap();

                let request = Request::parse(req.as_slice()).unwrap();
                println!("request: {:?}", &request);
                let response:Vec<u8> = match  request.target().0.as_str() {
                    "/" => Ok(Response(StatusLine::ok(), vec![], None)),
                    "/user-agent" =>
                        Response::ok(request.user_agent().map(|v|v.0).get_or_insert("".to_string())),
                    s if
                    s.starts_with("/echo") =>
                        Response::ok(get_path(s).as_ref()),
                    s if s.starts_with("/files") => {
                        let file_name = request.get_path();
                        let file = format!("/tmp/data/codecrafters.io/http-server-tester/{}", file_name);
                        println!("file: {:?}", file);
                        match read(file.as_ref()){
                            Ok(data) => {
                                Response::ok_bin(data.as_slice())
                            }
                            Err(_) =>
                                Ok(Response(StatusLine::not_found(), vec![], None))

                        }
                    }
                    _ =>
                        Ok(Response(StatusLine::not_found(), vec![], None))


                    }.unwrap().into();

               stream.write(response.as_ref()).unwrap();
               //stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
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