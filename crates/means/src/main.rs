use http_utils::tcp_listener::ServiceTcpListener;
use std::array::TryFromSliceError;

use std::convert::TryFrom;

use std::fmt;
use std::fmt::Formatter;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{ReadHalf, WriteHalf};

use std::collections::HashMap;
mod message;
use message::*;

#[derive(Default)]
struct State {
    data: HashMap<i32, i32>,
}

impl State {
    fn insert(&mut self, Insert { timestamp, price }: Insert) -> Result<(), ()> {
        if self.data.contains_key(&timestamp) {
            return Err(());
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

    pub fn extract_next_bytes(&mut self) -> Option<Result<Message, MessageParseError>> {
        match self.remaining_buffer.len() {
            value if value > 8 => {
                let mut next = [0; 9];
                next.copy_from_slice(&self.remaining_buffer[0..=8]);
                self.remaining_buffer = self.remaining_buffer[9..].to_vec();

                Some(next.try_into())
            }
            _ => None,
        }
    }

    pub async fn read(&mut self) -> Option<Result<Message, MessageParseError>> {
        if let Some(message) = self.extract_next_bytes() {
            return Some(message)
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

fn handle_message(message: Message, state: &mut State) -> Result<Option<i32>, ()> {
    match message {
        Message::Insert(insert) => {
            if state.insert(insert).is_err() {
                return Err(());
            }

            Ok(None)
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

            Ok(Some(mean))
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

                    while let Some(Ok(message)) = reader.read().await {
                        println!("[{}] Received Message {}", address, message);

                        match handle_message(message, &mut state){
                            Ok(Some(to_write)) => write.write_all(&to_write.to_be_bytes()).await.expect("Failed to write to client"),
                            Err(_) => break,
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
