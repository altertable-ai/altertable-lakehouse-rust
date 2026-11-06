use altertable_lakehouse::{AltertableClient, AltertableError, QueryRequest};
use futures_util::StreamExt;
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct Server {
    url: String,
    release: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn new(status: u16, chunks: Vec<Vec<u8>>, tail: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (release, wait) = mpsc::channel();
        let thread = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            reader.read_exact(&mut vec![0; length]).unwrap();
            write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
            for chunk in chunks {
                if write!(socket, "{:x}\r\n", chunk.len()).is_err()
                    || socket.write_all(&chunk).is_err()
                    || socket.write_all(b"\r\n").is_err()
                {
                    return;
                }
                socket.flush().unwrap();
            }
            let _ = wait.recv_timeout(Duration::from_secs(3));
            if !tail.is_empty() {
                let _ = write!(socket, "{:x}\r\n", tail.len());
                let _ = socket.write_all(&tail);
                let _ = socket.write_all(b"\r\n");
            }
            let _ = socket.write_all(b"0\r\n\r\n");
        });
        Self {
            url,
            release,
            thread: Some(thread),
        }
    }

    fn client(&self) -> AltertableClient {
        AltertableClient::builder()
            .base_url(&self.url)
            .credentials("user", "pass")
            .build()
            .unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.release.send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn request() -> QueryRequest {
    QueryRequest {
        statement: "SELECT name".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn rows_arrive_before_response_end_with_split_lines_and_utf8() {
    let prefix =
        "{}\n[{\"name\":\"name\",\"type\":\"VARCHAR\"}]\n[\"Zoë\"]\n[\"two\"]\n".as_bytes();
    let split = prefix.iter().position(|byte| *byte == 0xc3).unwrap() + 1;
    let server = Server::new(
        200,
        vec![prefix[..split].to_vec(), prefix[split..].to_vec()],
        b"[\"last\"]".to_vec(),
    );
    let mut result =
        tokio::time::timeout(Duration::from_secs(1), server.client().query(&request()))
            .await
            .expect("query buffered the response")
            .unwrap();
    for name in ["Zoë", "two"] {
        let row = tokio::time::timeout(Duration::from_secs(1), result.rows.next())
            .await
            .expect("row waited for the response end")
            .unwrap()
            .unwrap();
        assert_eq!(row, json!([name]));
    }
    server.release.send(()).unwrap();
    assert_eq!(result.rows.next().await.unwrap().unwrap(), json!(["last"]));
    assert!(result.rows.next().await.is_none());
}

#[tokio::test]
async fn malformed_later_line_is_reported_after_earlier_rows() {
    let server = Server::new(200, vec![b"{}\n[]\n[1]\n{broken}\n".to_vec()], vec![]);
    server.release.send(()).unwrap();
    let mut result = server.client().query(&request()).await.unwrap();
    assert_eq!(result.rows.next().await.unwrap().unwrap(), json!([1]));
    assert!(matches!(
        result.rows.next().await.unwrap(),
        Err(AltertableError::ParseError { line: Some(4), .. })
    ));
    assert!(result.rows.next().await.is_none());
}

#[tokio::test]
async fn query_all_still_collects_rows_and_errors_keep_their_body() {
    let server = Server::new(200, vec![b"{}\n[]\n[1]\n[2]".to_vec()], vec![]);
    server.release.send(()).unwrap();
    let result = server.client().query_all(&request()).await.unwrap();
    assert_eq!(result.rows, vec![json!([1]), json!([2])]);
    for status in [400, 401, 503] {
        let server = Server::new(status, vec![b"query rejected".to_vec()], vec![]);
        server.release.send(()).unwrap();
        let Err(error) = server.client().query(&request()).await else {
            panic!("expected HTTP error");
        };
        assert_eq!(error.context().unwrap().status, Some(status));
        assert_eq!(error.to_string(), "query rejected");
    }
}
