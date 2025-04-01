use std::io::{BufReader, Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::net::TcpStream;
use codecrafters_http_server::{Request, RequestTarget, Response, StatusLine};


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
        match stream {

            Ok(mut stream) => {
                let mut req: Vec<u8> = vec![];
                let req = read_request(&mut stream).unwrap();

                let request = Request::parse(req.as_slice()).unwrap();
                println!("request: {:?}", &request);
                let response:Vec<u8> = match  request.target().0.as_str() {
                    "/" => Response(StatusLine::ok(), vec![], None),
                    _ => {
                        let r = Response(StatusLine::not_found(), vec![], None);
                        println!("response: {:?}", r);
                        r
                    }
                    }.into();

                println!("respnse: {:?}", response);

               stream.write(response.as_ref()).unwrap();
               //stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
