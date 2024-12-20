use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};
use crate::ThreadPool;

pub struct Server {
    port: u32,
    ip: String,
    listener: TcpListener,
    pool: ThreadPool,
}

impl Server {
    pub fn new(port: u32, ip: String, pool: ThreadPool) -> Server {
        let address = format!("{ip}:{port}");
        let listener = match TcpListener::bind(&address.to_string()) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Error binding to address {}: {}", address, error);
                panic!();
            }
        };
        Server {
            port,
            ip,
            listener,
            pool,
        }
    }

    pub fn run(&self) {
        for stream in self.listener.incoming() {
            let stream = stream.unwrap();
            self.pool.execute(|| {
                Server::handle_connection(stream);
            });
        }
    }

    fn handle_connection(stream: TcpStream) {
        let buf_reader = BufReader::new(&stream);
        let request_line = buf_reader.lines().next().unwrap().unwrap();

        if request_line == "GET / HTTP/1.1" {
            Server::handle_get(stream, "main.html");
        } else if request_line == "GET /makena HTTP/1.1" {
            Server::handle_get(stream, "makena.html");
        } else {
            println!("Unknown HTTP Request:\n{}", request_line);
            Server::handle_get(stream, "unknown.html");
        }
    }

    fn handle_get(mut stream: TcpStream, file_name : &str) {
        let status_line = "HTTP/1.1 200 OK";
        let contents = fs::read_to_string(file_name).unwrap_or_else(|error| {
            eprintln!("Error reading contents of {file_name}: {error}");
            return fs::read_to_string("unknown.html").unwrap();
        });
        let length = contents.len();
        let response =
            format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
        stream.write_all(response.as_bytes()).unwrap_or_else(|error| {
            eprintln!("Error writing response to stream: {error}");
        });
    }
}

struct Endpoint {
    path: String,
    route: fn(stream: TcpStream),
}

impl Endpoint {
    pub fn new(path: String, route: fn(stream: TcpStream)) -> Endpoint {
        Endpoint {
            path,
            route,
        }
    }
}