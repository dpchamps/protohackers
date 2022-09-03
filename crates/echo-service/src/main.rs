use std::io;
use std::env;
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
    let host = env::var("HOST").unwrap_or("127.0.0.1".into());
    let port = env::var("PORT").unwrap_or("8080".into());
    let address = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&address).await;

    println!("Listening on {}", &address);

    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                tokio::spawn(async move { echo(&mut socket, addr).await });
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}
