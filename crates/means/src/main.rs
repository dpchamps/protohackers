use http_utils::tcp_listener::ServiceTcpListener;

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::ReadHalf;
#[tokio::main]
fn main() -> io::Result<()> {
    let listener = ServiceTcpListener::from_env()?.bind().await?;


    loop {
        match listener.accept().await {
            Ok((mut socket, address)) => {
                let (read, mut write) = socket.split();

                println!("Connection Open {}", )
            },
            Err(e) => {
                println!("Failed to get client with error: {:?}", e)
            }
        }
    }
}
