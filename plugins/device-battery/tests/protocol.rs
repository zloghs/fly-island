//! Exercise the real stdio process with both HID providers disabled: no hardware access.
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn initialize_rescan_unknown_action_and_shutdown_or_eof_remain_responsive() {
    for eof in [false, true] {
        let mut child = Guard(
            Command::new(env!("CARGO_BIN_EXE_device-battery"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let mut input = child.0.stdin.take().unwrap();
        let output = child.0.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else {
                    break;
                };
                let value: Value = serde_json::from_str(&line).unwrap();
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        let mut request = |value: Value| {
            writeln!(input, "{value}").unwrap();
            input.flush().unwrap();
        };
        request(
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
            "apiVersion":1,"settings":{"mchose":false,"rapoo":false,"scan-seconds":"5","language":"en"}}}),
        );
        let response = |id: u64| loop {
            let v = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
            if v.get("id") == Some(&json!(id)) {
                return v;
            }
            assert_eq!(v["method"], "publish");
            assert!(v["params"]["panel"]["rows"].as_array().unwrap().is_empty());
        };
        assert_eq!(response(0)["result"]["apiVersion"], 1);
        let published = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(published["method"], "publish");
        assert_eq!(published["params"]["panel"]["actions"], json!(["refresh"]));
        request(json!({"jsonrpc":"2.0","id":1,"method":"action","params":{"id":"refresh"}}));
        assert!(response(1).get("result").is_some());
        request(json!({"jsonrpc":"2.0","id":2,"method":"action","params":{"id":"firmware"}}));
        assert_eq!(response(2)["error"]["code"], -32000);
        if !eof {
            request(json!({"jsonrpc":"2.0","method":"shutdown"}));
        }
        drop(input);
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < end, "plugin did not exit promptly");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
