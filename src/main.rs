use std::io::Write;
use std::io::Read;
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let mut buffer = [0u8; 4096];
                stream.read(&mut buffer).unwrap();
                let response = handle_request(buffer);
                stream.write_all(response.as_bytes()).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_request(stream: [u8; 4096]) -> String {

    let mut status = "";
    let mut headers = "";

    // read the request
    let request_str = String::from_utf8_lossy(&stream);
    let request_lines: Vec<&str> = request_str.split("\r\n").collect();

    if (request_lines.len() == 0) {
        status = "400 Bad Request";
    } else {
        let first_line = request_lines[0];
        let first_line_split: Vec<&str> = first_line.split(" ").collect();

        let requested_path = first_line_split[1];
        println!("requested path: {}", requested_path);
    
        match requested_path {
            "/" => {
               status = "200 OK";
            }
            _ => {
                status = "404 Not Found";
            }
        }
    }

    return format!("HTTP/1.1 {status}\r\n{headers}\r\n");
}