use std::collections::HashMap;
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

fn respond(status: Option<u16>, body: Option<String>, headers: Option<HashMap<String, String>>) -> String {
    let status_code = status.unwrap_or(200);
    let status_message = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Unknown"
    };
    let body_string = body.unwrap_or(String::from(""));
    
    // build headers block
    let mut header_map = headers.unwrap_or(HashMap::new());
    if body_string.len() > 0 {
        header_map.insert(String::from("Content-Type"), String::from("text/plain"));
        header_map.insert(String::from("Content-Length"), body_string.len().to_string());
    }

    let headers_string = header_map.iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<String>>()
        .join("\r\n");
    let status_code_string = status.unwrap_or(200).to_string();
    return format!("HTTP/1.1 {status_code_string} {status_message}\r\n{headers_string}\r\n\r\n{body_string}");
}

fn handle_request(stream: [u8; 4096]) -> String {

    // read the request
    let request_str = String::from_utf8_lossy(&stream);
    let request_lines: Vec<&str> = request_str.split("\r\n").collect();

    if request_lines.len() == 0 {
        return respond(Some(400), None, None);
    } 

    let first_line = request_lines[0];
    let first_line_split: Vec<&str> = first_line
        .split(" ").collect();

    let requested_path = first_line_split[1];
    println!("requested path: {}", requested_path);

    if !requested_path.starts_with("/") {
        return respond(Some(200), None, None);
    }

    let requested_path_split: Vec<&str> = requested_path
        .split("/")
        .filter(|s| s.len() > 0)
        .collect();
    
    if requested_path_split.len() == 0 {
        return respond(Some(200), None, None);
    }

    if requested_path.starts_with("/echo/") {
        let echo_param = requested_path[6..].to_string();
        return respond(Some(200), Some(String::from(echo_param)), None);
    }

    return respond(Some(404), None, None);
}