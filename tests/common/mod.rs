use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use assinafy::{BaseUrl, Client};
use serde_json::{Value, json};

pub struct Request {
    pub method: String,
    pub url: url::Url,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

pub fn mock(data: Value) -> (Client, JoinHandle<Request>) {
    mock_response(
        "application/json",
        json!({"status":200, "message":"", "data":data})
            .to_string()
            .into_bytes(),
    )
}

pub fn mock_response(
    content_type: &'static str,
    response: Vec<u8>,
) -> (Client, JoinHandle<Request>) {
    mock_http("200 OK", content_type, "", response)
}

pub fn mock_http(
    status: &'static str,
    content_type: &'static str,
    extra_headers: &str,
    response: Vec<u8>,
) -> (Client, JoinHandle<Request>) {
    let extra_headers = extra_headers.to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let parts: Vec<_> = line.split_whitespace().collect();
        let method = parts[0].to_owned();
        let url = url::Url::parse(&format!("http://{address}{}", parts[1])).unwrap();
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        let len = headers
            .get("content-length")
            .map_or(0, |s| s.parse().unwrap());
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes).unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
        stream.write_all(&response).unwrap();
        Request {
            method,
            url,
            headers,
            body,
        }
    });
    let client = Client::builder()
        .base_url(BaseUrl::custom(format!("http://{address}/v1")).unwrap())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    (client, server)
}

pub fn validation_client() -> Client {
    Client::builder()
        .base_url(BaseUrl::custom("http://127.0.0.1:1/v1").unwrap())
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap()
}
