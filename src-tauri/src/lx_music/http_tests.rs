use super::{get_text, MAX_RESPONSE_BYTES};
use crate::lx_music::test_server::Server;

#[test]
fn local_api_redirects_are_rejected() {
    let server = Server::new(vec!["HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/other\r\nContent-Length: 0\r\n\r\n".into()]);
    let result = tauri::async_runtime::block_on(get_text(&format!("http://127.0.0.1:{}/status", server.port)));
    let error = result.unwrap_err();
    assert!(error.contains("302"), "{error}");
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn both_declared_and_chunked_oversized_responses_are_bounded() {
    let length = MAX_RESPONSE_BYTES + 1;
    let responses = [
        format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"),
        format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{length:x}\r\n{}\r\n0\r\n\r\n", "x".repeat(length)),
    ];
    for response in responses {
        let server = Server::new(vec![response]);
        let result = tauri::async_runtime::block_on(get_text(&format!("http://127.0.0.1:{}/lyric-all", server.port)));
        let error = result.unwrap_err();
        assert!(error.contains("too large"), "{error}");
        assert_eq!(server.finish().len(), 1);
    }
}
