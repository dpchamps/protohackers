use http_utils::tcp_listener::ServiceTcpListener;
use serde::{Deserialize, Serialize};

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::ReadHalf;


enum ParseStreamError {
    StringError,
    ParseError(serde_json::Error),
    InvalidObject,
}

impl From<ParseStreamError> for String {
    fn from(e: ParseStreamError) -> Self {
        match e {
            ParseStreamError::StringError => "Failed to parse tcp buffer into string".into(),
            ParseStreamError::ParseError(reason) => {
                format!("Failed to parse string into Result: {}", reason)
            }
            ParseStreamError::InvalidObject => "Failed to validate request object".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: u32,
}

impl Request {
    fn validate(&self) -> bool {
        self.method == "isPrime"
    }

    fn parse(result: &[u8]) -> Result<Request, ParseStreamError> {
        let result =
            String::from_utf8(result.to_vec()).map_err(|_| ParseStreamError::StringError)?;
        let trimmed = result.trim();
        let parsed: Request =
            serde_json::from_str(trimmed).map_err(ParseStreamError::ParseError)?;

        match parsed.validate() {
            true => Ok(parsed),
            _ => Err(ParseStreamError::InvalidObject),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

impl Response {
    fn is_prime_number(n: u32) -> bool {
        if n <= 1 {
            return false;
        }

        for i in 2..(n as f32).sqrt() as u32 {
            if n % i == 0 {
                return false;
            }
        }

        true
    }

    fn create_is_prime(prime: bool) -> Self {
        Response {
            method: "isPrime".into(),
            prime,
        }
    }

    fn from_request(Request { number, .. }: &Request) -> Response {
        let is_prime = Self::is_prime_number(*number);

        Response::create_is_prime(is_prime)
    }
}

struct Reader<'a> {
    socket: ReadHalf<'a>,
    buffer: Vec<u8>,
}

impl<'a> Reader<'a> {
    fn new(socket: ReadHalf<'a>) -> Self {
        Self {
            socket,
            buffer: vec![0; 1024],
        }
    }

    fn create_response(buffer: &[u8]) -> String {
        match Request::parse(buffer) {
            Ok(request) => {
                let response = Response::from_request(&request);
                serde_json::to_string(&response).expect("Could not deserialize json")
            }
            Err(e) => String::from(e),
        }
    }

    async fn read(&mut self) -> Option<String> {
        let n = self
            .socket
            .read(&mut self.buffer)
            .await
            .expect("failed to read data from socket");

        if n == 0 {
            return None;
        }

        Some(Self::create_response(&self.buffer[0..n]))
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = ServiceTcpListener::from_env()?.bind().await?;

    loop {
        match listener.accept().await {
            Ok((mut socket, address)) => {
                let (read, mut write) = socket.split();
                let mut reader = Reader::new(read);

                println!("Connection Open {}", address);

                while let Some(result) = reader.read().await {
                    write
                        .write_all(result.as_bytes())
                        .await
                        .expect("Failed to write to socket");
                }

                println!("Connection Closed {}", address);
            }
            Err(e) => {
                println!("Failed to get client with error: {:?}", e)
            }
        }
    }
}
