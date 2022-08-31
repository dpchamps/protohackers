use std::io;
use std::net::SocketAddr;
use tokio::io::copy;
use tokio::net::{TcpListener, TcpStream};

async fn echo(socket: &mut TcpStream, address: SocketAddr) -> io::Result<()> {
    println!("Connection Open: {}", address);
    let (mut reader, mut writer) = socket.split();

    copy(&mut reader, &mut writer).await?;

    println!("Connection Closed: {}", address);
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                tokio::spawn(async move { echo(&mut socket, addr).await });
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}
