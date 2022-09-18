use tokio::net::TcpStream;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use http_utils::tcp_stream::{ServiceTcpStream};


#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stream = ServiceTcpStream::from_env()?.connect().await?;
    let (mut reader, mut writer) = stream.split();

    writer.write_all(&[
        0x49, 0x00, 0x00, 0x30, 0x39, 0x00, 0x00, 0x00, 0x65,
        0x49, 0x00, 0x00, 0x30, 0x3a, 0x00, 0x00, 0x00, 0x66,
        0x49, 0x00, 0x00, 0x30, 0x3b, 0x00, 0x00, 0x00, 0x64,
        0x49, 0x00, 0x00, 0xa0, 0x3b, 0x00, 0x00, 0x00, 0x05,
        0x51, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x40, 0x00,
    ]).await;

    loop {
        let mut buffer = [0; 1024];

        let bytes_read = reader.read(&mut buffer).await?;

        println!("Read {} bytes", bytes_read);

        match bytes_read {
            value if value == 0 => break,
            _ => {
                println!("Read {:?}",  buffer[0..bytes_read].to_vec());
            }
        }
    }

    Ok(())
}
