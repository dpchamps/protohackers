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

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: f32,
}

impl Request {
    fn validate(&self) -> bool {
        self.method == "isPrime"
    }

    fn parse(result: &[u8]) -> Result<Vec<Request>, ParseStreamError> {
        return String::from_utf8(result.to_vec()).map_err(|_| ParseStreamError::StringError)?
            .lines()
            .map(|x| serde_json::from_str(x.trim()).map_err(ParseStreamError::ParseError))
            .collect();
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

impl Response {
    fn is_prime_number(n: f32) -> bool {
        let n_int: u32 = n as u32;
        if n_int <= 1 || n_int as f32 != n {
            return false;
        }

        for i in 2..=(n as f32).sqrt() as u32 {
            if n_int % i == 0 {
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
    in_buffer: Vec<u8>,
    out_buffer: Vec<ResponseResult>
}

impl<'a> Reader<'a> {
    fn new(socket: ReadHalf<'a>) -> Self {
        Self {
            socket,
            in_buffer: vec![0; 1024],
            out_buffer: vec![]
        }
    }

    fn create_response(buffer: &[u8]) -> Vec<ResponseResult> {
        match Request::parse(buffer) {
            Ok(request) => {
                let response = request
                    .iter()
                    .map(|req| match req.validate() {
                        true => ResponseResult::Prime(format!(
                            "{}\n",
                            serde_json::to_string(&Response::from_request(req))
                                .expect("Couldn't Deserialize json")
                        )),
                        _ => ResponseResult::Malformed(format!("Received incorrect method: {}\n", req.method)),
                    })
                    .collect();

                response
            }
            Err(e) => {
                let error = format!("{}\n", String::from(e));
                println!("Got malformed request: {}", &error);
                vec![ResponseResult::Malformed(error)]
            }
        }
    }

    async fn read(&mut self) -> Option<ResponseResult> {
        if let Some(next) = self.out_buffer.pop(){
            return Some(next);
        }


        let bytes_read = self
            .socket
            .read(&mut self.in_buffer)
            .await
            .expect("failed to read data from socket");

        if bytes_read == 0 {
            return None;
        }

        let response =  Self::create_response(&self.in_buffer[0..bytes_read]);
        let (head, tail) = response.split_at(1);

        self.out_buffer.append(&mut tail.to_vec());

        Some(head[0].clone())
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

                    println!("Connection Open {}", address);

                    while let Some(result) = reader.read().await {
                        write
                            .write_all(&result.as_bytes())
                            .await
                            .expect("Failed to write to socket");

                        match result {
                            ResponseResult::Malformed(_) => break,
                            _ => {}
                        }
                    }

                    println!("Connection Closed {}", address);
                });
            }
            Err(e) => {
                println!("Failed to get client with error: {:?}", e)
            }
        }
    }
}
