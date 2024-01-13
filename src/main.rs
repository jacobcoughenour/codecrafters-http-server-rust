use std::env;
use std::io;

use http_server_starter_rust::*;

#[tokio::main]
async fn main() -> io::Result<()> {
    // parse command line arguments
    let args = env::args().collect::<Vec<String>>();
    let mut directory = String::from("");
    if args.len() > 2 && args[1] == "--directory" {
        directory = args[2].clone();
    }

    let mut server = Server::new(4221);

    server.get(String::from("echo/*"), |request| {
        if !request.path.starts_with("/echo/") {
            return Server::respond(Some(400), Some(String::from("Bad Request")), None);
        }
        let echo_param = request.path[6..].to_string();
        return Server::respond(Some(200), Some(String::from(echo_param)), None);
    });

    server.get(String::from("user-agent"), |request| {
        let unknown_agent = String::from("unknown");
        let user_agent = request.headers.get("user-agent").unwrap_or(&unknown_agent);
        return Server::respond(Some(200), Some(user_agent.to_string()), None);
    });

    if !directory.is_empty() {
        server.serve(String::from("files"), directory);
    }

    // start server
    server.listen().await
}
