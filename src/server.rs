use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};
use std::fmt::{Display, Formatter};
use crate::{ThreadPool};

pub struct Server {
    listener: TcpListener,
    pool: ThreadPool,
    endpoints: Vec<Endpoint>,
}

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

struct Request {
    method: HttpMethod,
    path: String,
    protocol: String,
    body: String,
}

#[derive(Clone)]
enum StatusCode {
    Ok = 200,
    BadRequest = 400,
    NotFound = 404,
    InternalServerError = 500,
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Ok => write!(f, "200 OK"),
            StatusCode::BadRequest => write!(f, "400 Bad Request"),
            StatusCode::NotFound => write!(f, "404 Not Found"),
            StatusCode::InternalServerError => write!(f, "500 Internal Server Error"),
        }.expect("Invalid/unimplemented status code");
        Ok(())
    }
}

#[derive(Clone)]
struct Response {
    protocol: String,
    status_code: StatusCode,
    body: String,
}

impl Server {

    pub fn new(ip: String, port: u32) -> Server {
        let address = format!("{ip}:{port}");
        let listener = match TcpListener::bind(&address.to_string()) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Error binding to address {}: {}", address, error);
                panic!();
            }
        };
        let pool = ThreadPool::new(4);
        let endpoints = vec![];
        Server {
            listener,
            pool,
            endpoints
        }
    }

    pub fn run(&self) {
        for stream in self.listener.incoming() {
            // read the stream into a Request
            let mut stream = stream.expect("Error reading stream");
            let request = Server::read_stream(&stream);

            // Find the corresponding endpoint
            let handler = self.find_endpoint(&request.path).unwrap_or_else(|| {
                eprintln!("No handler found for path: {}", &request.path);
                Endpoint::default().handler
            });

            // Execute the handler in a thread
            self.pool.execute(move || {
                Server::send_response(handler, &mut stream);
            });
        }
    }

    fn find_endpoint(&self, path: &str) -> Option<Response> {
        for endpoint in self.endpoints.clone() {
            if path == endpoint.path {
                return Some(endpoint.handler);
            }
        }
        None
    }

    fn read_stream(stream: &TcpStream) -> Request {
        let mut lines = BufReader::new(stream).lines().map(|line| line.unwrap());
        let first_line = lines.next().unwrap();
        let mut parts = first_line.split_whitespace();

        let method = match parts.next().unwrap() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            _ => {
                eprintln!("Invalid HTTP method");
                HttpMethod::GET
            }
        };

        let path = match parts.next() {
            Some(path) => path.to_string(),
            None => {
                eprintln!("Invalid path");
                "/".to_string()
            }
        };

        let protocol = match parts.next() {
            Some(protocol) => protocol.to_string(),
            None => {
                eprintln!("Invalid protocol");
                "HTTP/1.1".to_string()
            }
        };

        let body = " ".to_string(); //lines.collect::<Vec<String>>().join("\n");

        Request {
            method,
            path,
            protocol,
            body
        }
    }

    fn send_response(response: Response, stream: &mut TcpStream) {
        let (protocol, status_code, body) = (&response.protocol, &response.status_code, &response.body);
        let length = response.body.len();
        let response =
            format!("{protocol} {status_code}\r\nContent-Length: {length}\r\n\r\n{body}");

        stream.write_all(response.as_bytes()).unwrap_or_else(|error| {
            eprintln!("Error writing response to stream: {error}");
        });
    }

    fn html_response(file_name: String) -> Response {
        let contents = fs::read_to_string(file_name.clone()).unwrap_or_else(|error| {
            eprintln!("Error reading contents of {file_name}: {error}");
            return fs::read_to_string("unknown.html").unwrap();
        });

        Response {
            protocol: "HTTP/1.1".to_string(),
            status_code: StatusCode::Ok,
            body: contents,
        }
    }

    pub fn add_get_endpoint(&mut self, path: &str, file_name: &str) {
        self.add_endpoint(path, Server::html_response(file_name.to_string()));
    }

    fn add_endpoint(&mut self, path: &str, handler: Response) {
        self.endpoints.push(Endpoint::new(path.to_string(), handler));
    }
}

#[derive(Clone)]
struct Endpoint {
    path: String,
    handler: Response, //TODO: Allow for dynamic endpoints
}

impl Endpoint {
    pub fn new(path: String, handler: Response) -> Endpoint {
        Endpoint {
            path,
            handler,
        }
    }
    pub fn default() -> Endpoint {
        Endpoint::new("/".to_string(), Server::html_response("unknown.html".to_string()))
    }
}