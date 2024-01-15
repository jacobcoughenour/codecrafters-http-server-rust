use nom::AsBytes;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

const MAX_REQUEST_SIZE: usize = 102400;

#[derive(Debug, Default, Eq, PartialEq, Hash, Clone)]
pub enum HttpVerb {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    TRACE,
    CONNECT,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct EndpointKey {
    verb: HttpVerb,
    path: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct StaticDirectoryEntry {
    pub directory: String,
    pub allow_upload: bool,
}

#[derive(Debug, Default)]
pub struct Request {
    pub verb: HttpVerb,
    /// full requested path
    pub path: String,
    /// key will always be lowercase
    pub headers: HashMap<String, String>,
    /// body of the request
    pub body: String,
}

#[derive(Debug, Default)]
pub struct Server {
    port: u16,
    registry: ServerRegistry,
}
impl Server {
    pub fn new(port: u16) -> Server {
        Server {
            port,
            registry: ServerRegistry::new(),
        }
    }

    pub async fn listen(self) -> io::Result<()> {
        let port = self.port;
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .unwrap();

        println!("Server started on port {port}!");

        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    let handler = self.registry.clone();
                    tokio::spawn(async move {
                        handler.handle_socket(socket).await;
                    });
                }
                Err(e) => {
                    println!("failed to accept socket; error = {:?}", e);
                }
            }
        }
    }

    /// Registers a new endpoint with the server.
    /// Consider using `get` instead.
    pub fn register_endpoint(
        &mut self,
        verb: HttpVerb,
        path: String,
        handler: fn(Request) -> String,
    ) {
        let mut normalized_path = path;
        if !normalized_path.starts_with("/") {
            normalized_path = format!("/{}", normalized_path);
        }
        let endpoint_key = EndpointKey {
            verb,
            path: normalized_path,
        };
        self.registry
            .endpoints
            .insert(endpoint_key, Box::new(handler));
    }

    pub fn get(&mut self, path: String, handler: fn(Request) -> String) {
        self.register_endpoint(HttpVerb::GET, path, handler);
    }

    pub fn post(&mut self, path: String, handler: fn(Request) -> String) {
        self.register_endpoint(HttpVerb::POST, path, handler);
    }

    /// Serves a directory of static files at the given endpoint.
    /// leave the endpoint empty to serve the directory at the root.
    pub fn serve(&mut self, path: String, directory: String, allow_upload: bool) {
        if directory.is_empty() {
            return;
        }
        let mut normalized_path = path;
        if !normalized_path.starts_with("/") {
            normalized_path = format!("/{}", normalized_path);
        }
        self.registry.static_directories.insert(
            normalized_path,
            StaticDirectoryEntry {
                directory,
                allow_upload,
            },
        );
    }

    pub fn respond(
        status: Option<u16>,
        body: Option<String>,
        headers: Option<HashMap<String, String>>,
    ) -> String {
        let status_code = status.unwrap_or(200);
        let status_message = match status_code {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            _ => "Unknown",
        };
        let body_string = body.unwrap_or(String::from(""));

        // build headers block
        let mut header_map = headers.unwrap_or(HashMap::new());
        if !body_string.is_empty() {
            // we only add this if they aren't already in the headers
            header_map
                .entry(String::from("Content-Type"))
                .or_insert(String::from("text/plain"));
            header_map
                .entry(String::from("Content-Length"))
                .or_insert(body_string.len().to_string());
        }

        let headers_string = header_map
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\r\n");
        let status_code_string = status.unwrap_or(200).to_string();
        return format!("HTTP/1.1 {status_code_string} {status_message}\r\n{headers_string}\r\n\r\n{body_string}");
    }
}

#[derive(Debug, Default, Clone)]
pub struct ServerRegistry {
    // map of endpoint to directory
    pub endpoints: HashMap<EndpointKey, Box<fn(Request) -> String>>,
    pub static_directories: HashMap<String, StaticDirectoryEntry>,
}
impl ServerRegistry {
    pub fn new() -> ServerRegistry {
        ServerRegistry {
            endpoints: HashMap::new(),
            static_directories: HashMap::new(),
        }
    }

    pub async fn handle_socket(self, mut stream: TcpStream) {
        let mut buffer = [0u8; MAX_REQUEST_SIZE];
        stream.read(&mut buffer).await.unwrap();
        let response = self.handle_request(buffer);
        stream.write(response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }

    fn handle_request(self, stream: [u8; MAX_REQUEST_SIZE]) -> String {
        // read the request and split it into lines
        let request_str = String::from_utf8_lossy(&stream);

        // write request to file
        // let mut file1 = std::fs::File::create("request.txt").unwrap();
        // file1.write_all(request_str.as_bytes()).unwrap();

        let request_lines: Vec<&str> = request_str.split("\r\n").collect();

        if request_lines.len() == 0 {
            return Server::respond(Some(400), None, None);
        }

        // parse the first line
        // ex: GET / HTTP/1.1
        let first_line = request_lines[0];
        let first_line_split: Vec<&str> = first_line.split(" ").collect();

        if first_line_split.len() != 3 {
            return Server::respond(Some(400), None, None);
        }

        let verb = match first_line_split[0] {
            "GET" => HttpVerb::GET,
            "POST" => HttpVerb::POST,
            "PUT" => HttpVerb::PUT,
            "DELETE" => HttpVerb::DELETE,
            "HEAD" => HttpVerb::HEAD,
            "OPTIONS" => HttpVerb::OPTIONS,
            "TRACE" => HttpVerb::TRACE,
            "CONNECT" => HttpVerb::CONNECT,
            _ => HttpVerb::GET,
        };
        let requested_path = first_line_split[1];

        if !requested_path.starts_with("/") {
            return Server::respond(Some(200), None, None);
        }

        let requested_path_split: Vec<&str> = requested_path
            .split("/")
            // filter out the empty strings
            // this means // will be treated as /
            .filter(|s| s.len() > 0)
            .collect();

        // respond with 200 when the path is empty
        if requested_path_split.len() == 0 {
            return Server::respond(Some(200), None, None);
        }

        // parse headers
        let mut headers: HashMap<String, String> = HashMap::new();
        // for each line after the first
        let mut i = 1;
        while i < request_lines.len() {
            let line = request_lines[i];
            if line.is_empty() {
                break;
            }
            let line_split: Vec<&str> = line.split(":").collect();
            if line_split.len() == 2 {
                headers.insert(
                    String::from(line_split[0].trim().to_lowercase()),
                    String::from(line_split[1].trim()),
                );
            }
            i += 1;
        }

        // parse body
        let mut body = String::from("");
        let mut body_raw: &[u8] = &[];
        i += 1;
        if i < request_lines.len() {
            let request_bin = &stream;
            // find first instance of \r\n\r\n
            let mut body_start = 0;
            for j in 0..(request_bin.len() - 3) {
                if request_bin[j] == '\r' as u8
                    && request_bin[j + 1] == '\n' as u8
                    && request_bin[j + 2] == '\r' as u8
                    && request_bin[j + 3] == '\n' as u8
                {
                    body_start = j + 4;
                    break;
                }
            }

            if body_start > 0 {
                let content_length = match headers.get("content-length") {
                    Some(length) => length.parse::<usize>().unwrap_or(0),
                    None => 0,
                };

                body = String::from_utf8_lossy(
                    &request_bin[body_start..(body_start + content_length)],
                )
                .to_string();
                body_raw = request_bin[body_start..(body_start + content_length)].as_bytes();
            }
        }
        println!("body length: {}", body.len());

        // match endpoints
        for (key, handler) in self.endpoints.iter() {
            if key.verb != verb {
                continue;
            }

            if !key.path.starts_with(requested_path)
                && !(key.path.ends_with("*")
                    && requested_path.starts_with(&key.path[..key.path.len() - 1]))
            {
                continue;
            }

            return handler(Request {
                verb,
                path: requested_path.to_string(),
                headers: headers.clone(),
                body,
            });
        }

        // match for static file serving
        for (path, entry) in self.static_directories.iter() {
            if !requested_path.starts_with(path) {
                // println!("path doesn't start with {}", path);
                continue;
            }

            let dir = entry.directory.clone();

            let file_path = format!("{}{}", dir, &requested_path[path.len()..]);

            if verb == HttpVerb::GET {
                // println!("file path: {}", file_path);
                // try to load the file
                // todo would be cool to cache these files
                let file_path2 = file_path.clone();
                let file_contents = std::fs::read_to_string(file_path);
                match file_contents {
                    Ok(contents) => {
                        let file_length = contents.len();

                        let file_type = match file_path2.split(".").last() {
                            Some("html") => "text/html",
                            Some("css") => "text/css",
                            Some("js") => "text/javascript",
                            Some("png") => "image/png",
                            _ => "application/octet-stream",
                        };

                        return Server::respond(
                            Some(200),
                            Some(contents),
                            Some(
                                [
                                    (String::from("Content-Type"), file_type.to_string()),
                                    (String::from("Content-Length"), file_length.to_string()),
                                ]
                                .iter()
                                .cloned()
                                .collect(),
                            ),
                        );
                    }
                    Err(_) => {
                        // continue
                    }
                }
            } else if verb == HttpVerb::POST && entry.allow_upload {
                let mut file = std::fs::File::create(file_path).unwrap();
                file.write_all(body_raw.as_bytes()).unwrap();
                // println!("created file");
                return Server::respond(Some(201), None, None);
            }
        }

        return Server::respond(Some(404), None, None);
    }
}
