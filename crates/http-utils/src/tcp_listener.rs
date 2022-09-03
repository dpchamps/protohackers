use tokio::net::TcpListener;
use tokio::io;
use std::env;
use std::future::Future;
use std::io::ErrorKind;

pub struct ServiceTcpListener {
    host: String,
    port: String
}

impl From<&ServiceTcpListener> for String {
    fn from(ServiceTcpListener { host, port }: &ServiceTcpListener) -> Self {
        format!("{}:{}", host, port)
    }
}

impl ServiceTcpListener {
    fn create_env_error(env_var_name: &str) -> io::Error {
        io::Error::new(
            ErrorKind::Other,
            format!("{} not found in environment", env_var_name)
        )
    }

    pub fn from_env() -> Result<Self, io::Error> {
        Ok(ServiceTcpListener {
            host: env::var("HOST").map_err(|_| Self::create_env_error("HOST"))?,
            port: env::var("PORT").map_err(|_| Self::create_env_error("PORT"))?
        })
    }

    pub async fn bind(&self) -> io::Result<TcpListener> {
        let address = String::from(self);
        let listener = TcpListener::bind(&address).await?;

        println!("{}", format!("Listening on: {}", &address));

        Ok(listener)
    }
}