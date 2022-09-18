pub mod tcp_listener;
pub mod tcp_stream;
// https://www.w3.org/Protocols/rfc2616/rfc2616-sec6.html

struct Status {
    version: String,
    code: u8,
    reason: String,
}

impl From<Status> for String {
    fn from(status: Status) -> Self {
        format!("{} {} {}", status.version, status.code, status.reason)
    }
}

struct Response {
    status: Status,
    body: String,
}

impl From<Response> for String {
    fn from(response: Response) -> String {
        format!("{}\n\n{}", String::from(response.status), response.body)
    }
}

impl Response {
    pub fn into_bytes(self) -> Vec<u8> {
        let response_string = String::from(self);

        response_string.into_bytes()
    }
}

impl From<Response> for Vec<u8> {
    fn from(response: Response) -> Vec<u8> {
        let x: String = response.into();
        x.into_bytes()
    }
}
