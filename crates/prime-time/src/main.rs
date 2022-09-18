use std::collections::VecDeque;
use http_utils::tcp_listener::ServiceTcpListener;
use serde::{Deserialize, Serialize};

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::ReadHalf;

enum ParseStreamError {
    StringError,
    ParseError(serde_json::Error, String),
    InvalidObject(String),
}

impl From<ParseStreamError> for String {
    fn from(e: ParseStreamError) -> Self {
        match e {
            ParseStreamError::StringError => "Failed to parse tcp buffer into string".into(),
            ParseStreamError::ParseError(reason, input) => {
                format!("Failed to parse string: {} {}", reason, input)
            }
            ParseStreamError::InvalidObject(input) => format!("Failed to validate request object: {:?}", input),
        }
    }
}

#[derive(Clone)]
enum ResponseResult {
    Prime(String),
    Malformed(String),
}

impl ResponseResult {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            ResponseResult::Prime(str) => str.clone().into_bytes(),
            ResponseResult::Malformed(str) => str.clone().into_bytes(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
struct Request {
    method: String,
    number: f64,
}

impl Request {
    fn validate(&self) -> bool {
        self.method == "isPrime"
    }

    fn parse(input: &str) -> Result<Request, ParseStreamError> {
        let result: Request = serde_json::from_str(input).map_err(|e| ParseStreamError::ParseError(e, input.into()))?;

        match result.validate() {
            true => Ok(result),
            false => Err(ParseStreamError::InvalidObject(input.into()))
        }

    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

impl Response {
    fn is_prime_number(n: f64) -> bool {
        let n_int: u64 = n as u64;

        if n <= 1.0 || n.floor() != n {
            return false;
        }

        for i in 2..=(n.sqrt() as u64) {
            if n_int % i == 0 {
                return false;
            }
        }

        true
    }

    fn from_request(Request { number, .. }: &Request) -> Response {
        let is_prime = Self::is_prime_number(*number);

        Response {
            method: "isPrime".into(),
            prime: is_prime,
        }
    }
}

struct Reader<'a> {
    socket: ReadHalf<'a>,
    stream_buffer: String,
}

impl<'a> Reader<'a> {
    fn new(socket: ReadHalf<'a>) -> Self {
        Self {
            socket,
            stream_buffer: String::new(),
        }
    }

    fn create_response(input: &str) -> ResponseResult {
        match Request::parse(input){
            Ok(request) => {
                ResponseResult::Prime(format!("{}\n", serde_json::to_string(&Response::from_request(&request)).expect("Failed to serialize JSON")))
            }
            Err(e) => ResponseResult::Malformed(String::from(e))
        }
    }

    fn read_line_from_buffer(&mut self) -> Option<ResponseResult>{
        for (idx, c) in self.stream_buffer.chars().enumerate() {
            if c == '\n' {
                let result = Self::create_response(&self.stream_buffer[..idx]);
                self.stream_buffer = (self.stream_buffer[idx+1..]).into();

                return Some(result);
            }
        }

        None
    }

    async fn read(&mut self) -> Option<ResponseResult> {
        if let Some(next_line) = self.read_line_from_buffer(){
            return Some(next_line);
        }

        let mut read_buffer = [0;1024];

        loop {
            let bytes_read = self
                .socket
                .read(&mut read_buffer)
                .await
                .expect("failed to read data from socket");

            match bytes_read {
                value if value == 0 => return None,
                _ => {
                    self.stream_buffer.push_str(&String::from_utf8(self.in_buffer[0..bytes_read].to_vec()).expect(""));

                    if let Some(next_line) = self.read_line_from_buffer() {
                        return Some(next_line)
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = ServiceTcpListener::from_env()?.bind().await?;

    loop {
        match listener.accept().await {
            Ok((mut socket, address)) => {
                tokio::spawn(async move {
                    let (read, mut write) = socket.split();
                    let mut reader = Reader::new(read);

                    println!("[{}] OPEN", address);

                    while let Some(result) = reader.read().await {
                        write
                            .write_all(&result.as_bytes())
                            .await
                            .expect("Failed to write to socket");

                        match result {
                            ResponseResult::Malformed(_) => {
                                break;
                            },
                            _ => {}
                        }
                    }

                    println!("[{}] CLOSED", address);
                });
            }
            Err(e) => {
                println!("Failed to get client with error: {:?}", e)
            }
        }
    }
}
