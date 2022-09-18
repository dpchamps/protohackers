use http_utils::tcp_listener::ServiceTcpListener;
use std::array::TryFromSliceError;

use std::convert::TryFrom;

use std::fmt;
use std::fmt::Formatter;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::ReadHalf;

use std::collections::HashMap;

impl TryFrom<&[u8]> for I32Result {
    type Error = MessageParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(I32Result(i32::from_be_bytes(value.try_into().map_err(
            |e| MessageParseError::ByteRangeError(e, value.to_vec()),
        )?)))
    }
}

enum Message {
    Insert(Insert),
    Query(Query),
}

impl TryFrom<[u8; 9]> for Message {
    type Error = MessageParseError;

    fn try_from(value: [u8; 9]) -> Result<Self, Self::Error> {
        let message_type = value[0];
        let lhs: I32Result = value[1..=4].try_into()?;
        let rhs: I32Result = value[5..=8].try_into()?;

        match message_type as char {
            'I' => Ok(Message::Insert(Insert {
                timestamp: lhs.into(),
                price: rhs.into(),
            })),
            'Q' => Ok(Message::Query(Query {
                min_time: lhs.into(),
                max_time: rhs.into(),
            })),
            _ => Err(MessageParseError::InvalidMessageType(message_type as char)),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Message::Insert(Insert { timestamp, price }) => {
                write!(f, "Insert\t{}\t{}", timestamp, price)
            }
            Message::Query(Query { min_time, max_time }) => {
                write!(f, "Query\t{}\t{}", min_time, max_time)
            }
        }
    }
}

struct Insert {
    timestamp: i32,
    price: i32,
}

struct Query {
    min_time: i32,
    max_time: i32,
}

struct I32Result(i32);

impl From<I32Result> for i32 {
    fn from(result: I32Result) -> Self {
        result.0
    }
}

enum MessageParseError {
    InvalidMessageType(char),
    ByteRangeError(TryFromSliceError, Vec<u8>),
}

impl fmt::Display for MessageParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MessageParseError::InvalidMessageType(message_type) => {
                write!(f, "Received an invalid message type: {} ", message_type)
            }
            MessageParseError::ByteRangeError(e, v) => {
                write!(f, "Couldn't coerce range from slice: {} {:?}", e, v)
            }
        }
    }
}

#[derive(Default)]
struct State {
    data: HashMap<i32, i32>,
}

impl State {
    fn insert(&mut self, Insert { timestamp, price }: Insert) -> Result<(), i32> {
        if self.data.contains_key(&timestamp) {
            return Err(timestamp);
        }

        self.data.insert(timestamp, price);

        Ok(())
    }

    fn select_prices(&self, Query { min_time, max_time }: &Query) -> Vec<i64> {
        let mut results = vec![];

        for (timestamp, price) in self.data.iter() {
            if timestamp >= min_time && timestamp <= max_time {
                results.push(*price as i64)
            }
        }

        results
    }
}

struct Reader<'a> {
    socket: ReadHalf<'a>,
    remaining_buffer: Vec<u8>,
}

impl<'a> Reader<'a> {
    pub fn new(socket: ReadHalf<'a>) -> Self {
        Self {
            socket,
            remaining_buffer: vec![],
        }
    }

    pub fn extract_next_bytes(&mut self) -> Option<[u8; 9]> {
        match self.remaining_buffer.len() {
            value if value > 8 => {
                let mut next = [0; 9];
                next.copy_from_slice(&self.remaining_buffer[0..=8]);
                self.remaining_buffer = self.remaining_buffer[9..].to_vec();

                Some(next.try_into().expect("This is an unreachable error"))
            }
            _ => None,
        }
    }
    pub async fn read(&mut self) -> Option<[u8; 9]> {
        if let Some(message) = self.extract_next_bytes() {
            return Some(message);
        }

        let mut read_buffer = [0; 1024];

        loop {
            let bytes_read = self
                .socket
                .read(&mut read_buffer)
                .await
                .expect("Failed to read from stream");

            self.remaining_buffer
                .extend_from_slice(&read_buffer[0..bytes_read]);

            match self.remaining_buffer.len() {
                value if value == 0 => return None,
                _ => {
                    if let Some(message) = self.extract_next_bytes() {
                        return Some(message);
                    }

                    if bytes_read == 0 {
                        return None;
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
                    let mut state = State::default();

                    println!("[{}] OPEN", address);

                    while let Some(next_bytes) = reader.read().await {
                        match Message::try_from(next_bytes) {
                            Ok(message) => {
                                println!("[{}] Received {}", address, message);
                                match message {
                                    Message::Insert(insert) => {
                                        if state.insert(insert).is_err() {
                                            println!("Got undefined behavior at insert");
                                            break;
                                        }
                                    }
                                    Message::Query(query) => {
                                        let results = state.select_prices(&query);
                                        let mean = {
                                            let total: i64 = results.iter().sum();

                                            if total == 0 {
                                                0
                                            } else {
                                                (total / results.len() as i64) as i32
                                            }
                                        };

                                        println!(
                                            "[{}] Writing: {:?}",
                                            address,
                                            &mean.to_be_bytes()
                                        );

                                        match write.write_all(&mean.to_be_bytes()).await {
                                            Ok(_) => println!("Write success"),
                                            Err(e) => println!("Failed to write {}", e),
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Got undefined behavior {}", e);
                                break;
                            }
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
