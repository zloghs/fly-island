//! Loopback fixtures for the actual HTTP client and the multi-request lyric lookup.
use std::{io::{Read, Write}, net::TcpListener, thread::{self, JoinHandle}, time::{Duration, Instant}};

pub(crate) struct Server {
    pub(crate) port: u16,
    worker: Option<JoinHandle<Vec<String>>>,
}

impl Server {
    pub(crate) fn new(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut requests = Vec::new();
            for response in responses {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        _ => return requests,
                    }
                };
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap_or(0);
                    if count == 0 || bytes.len() > 8192 { break; }
                    bytes.extend_from_slice(&buffer[..count]);
                }
                requests.push(String::from_utf8_lossy(&bytes).lines().next().unwrap_or_default().to_owned());
                // A bounded client can intentionally close before an oversized body finishes.
                let _ = stream.write_all(response.as_bytes());
            }
            requests
        });
        Self { port, worker: Some(worker) }
    }

    pub(crate) fn finish(mut self) -> Vec<String> {
        self.worker.take().unwrap().join().unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}

pub(crate) fn json(body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}
