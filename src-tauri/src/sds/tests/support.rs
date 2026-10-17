use serde_json::Value;
use std::{future::Future, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

pub struct Request {
    pub path: String,
    pub body: Value,
    pub authorization: Option<String>,
}
pub struct Response {
    pub status: u16,
    pub body: Value,
    pub location: Option<String>,
    pub raw_body: Option<Vec<u8>>,
    pub chunk_delay_ms: u64,
    pub chunked: bool,
}
impl Response {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body,
            location: None,
            raw_body: None,
            chunk_delay_ms: 0,
            chunked: false,
        }
    }
    pub fn redirect(location: String) -> Self {
        Self {
            status: 302,
            body: Value::Null,
            location: Some(location),
            raw_body: None,
            chunk_delay_ms: 0,
            chunked: false,
        }
    }
    pub fn binary(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            body: Value::Null,
            location: None,
            raw_body: Some(body),
            chunk_delay_ms: 0,
            chunked: false,
        }
    }
}
pub struct TestServer {
    pub base_url: String,
    task: JoinHandle<()>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestServer {
    pub async fn start<F, Fut>(handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let handler = handler.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let headers_end;
                    loop {
                        let mut chunk = [0u8; 1024];
                        let count = stream.read(&mut chunk).await.unwrap();
                        if count == 0 || bytes.len() + count > 65_536 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..count]);
                        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                            headers_end = end + 4;
                            break;
                        }
                    }
                    let headers = String::from_utf8(bytes[..headers_end].to_vec()).unwrap();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|n| n.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    while bytes.len() < headers_end + length {
                        let mut chunk = [0u8; 1024];
                        let count = stream.read(&mut chunk).await.unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..count]);
                    }
                    let path = headers
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_string();
                    let authorization = headers.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_string())
                    });
                    let body = if length == 0 {
                        Value::Null
                    } else {
                        serde_json::from_slice(&bytes[headers_end..headers_end + length]).unwrap()
                    };
                    let response = handler(Request {
                        path,
                        body,
                        authorization,
                    })
                    .await;
                    if response.status == 0 {
                        return;
                    }
                    let body = response
                        .raw_body
                        .unwrap_or_else(|| serde_json::to_vec(&response.body).unwrap());
                    let location = response
                        .location
                        .map(|url| format!("Location: {url}\r\n"))
                        .unwrap_or_default();
                    let length = if response.chunked {
                        "Transfer-Encoding: chunked".into()
                    } else {
                        format!("Content-Length: {}", body.len())
                    };
                    let header = format!("HTTP/1.1 {} Synthetic\r\n{location}Content-Type: application/octet-stream\r\n{length}\r\nConnection: close\r\n\r\n", response.status);
                    let _ = stream.write_all(header.as_bytes()).await;
                    for chunk in body.chunks(65_536) {
                        if response.chunked
                            && stream
                                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                                .await
                                .is_err()
                        {
                            break;
                        }
                        if stream.write_all(chunk).await.is_err() {
                            break;
                        }
                        if response.chunked && stream.write_all(b"\r\n").await.is_err() {
                            break;
                        }
                        if response.chunk_delay_ms > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                response.chunk_delay_ms,
                            ))
                            .await;
                        }
                    }
                    if response.chunked {
                        let _ = stream.write_all(b"0\r\n\r\n").await;
                    }
                });
            }
        });
        Self {
            base_url: format!("http://{address}/skm"),
            task,
        }
    }
}
