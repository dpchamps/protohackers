use http_utils::tcp_listener::ServiceTcpListener;
use std::array::TryFromSliceError;

use std::convert::TryFrom;

use std::fmt;
use std::fmt::Formatter;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::ReadHalf;

use std::collections::HashMap;

struct I32Result(i32);

impl From<I32Result> for i32 {
    fn from(result: I32Result) -> Self {
        result.0
    }
}

impl TryFrom<&[u8]> for I32Result {
    type Error = MessageParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(I32Result(i32::from_be_bytes(value.try_into().map_err(
            |e| MessageParseError::ByteRangeError(e, value.to_vec()),
        )?)))
    }
}

pub enum Message {
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

pub struct Insert {
    pub timestamp: i32,
    pub price: i32,
}

pub struct Query {
    pub min_time: i32,
    pub max_time: i32,
}

pub enum MessageParseError {
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