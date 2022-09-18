use std::env;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

use std::io::ErrorKind;
use std::fmt;
use std::fmt::Formatter;

pub struct ServiceTcpStream {
    host: String,
    port: String,
}

impl fmt::Display for ServiceTcpStream{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ServiceTcpStream {}:{}", self.host, self.port)
    }
}

impl From<&ServiceTcpStream> for String {
    fn from(ServiceTcpStream { host, port }: &ServiceTcpStream) -> Self {
        format!("{}:{}", host, port)
    }
}

impl ServiceTcpStream {
    fn create_env_error(env_var_name: &str) -> io::Error {
        io::Error::new(
            ErrorKind::Other,
            format!("{} not found in environment", env_var_name),
        )
    }

    pub fn from_env() -> Result<Self, io::Error> {
        Ok(ServiceTcpStream {
            host: env::var("HOST").map_err(|_| Self::create_env_error("HOST"))?,
            port: env::var("PORT").map_err(|_| Self::create_env_error("PORT"))?,
        })
    }

    pub async fn connect(&self) -> io::Result<TcpStream> {
        let address = String::from(self);
        let stream = TcpStream::connect(address).await?;

        println!("{}", self);

        Ok(stream)
    }
}