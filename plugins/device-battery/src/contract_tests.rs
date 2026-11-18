//! Validate real host schemas without compiling or running the desktop host.
#[allow(dead_code)]
#[path = "../../../src-tauri/src/plugin_host/protocol.rs"]
mod host_protocol;
#[allow(dead_code)]
#[path = "../../../src-tauri/src/plugin_host/manifest.rs"]
mod manifest;

use crate::{
    panel,
    protocol::{Charge, Connection, Reading},
    settings::Settings,
    state::{Device, State},
};
use fly_island_plugin_sdk::json;

#[test]
fn manifest_defaults_and_all_panel_states_match_the_host_contract() {
    let manifest = manifest::Manifest::parse(include_bytes!("../fly-island-plugin.json")).unwrap();
    let defaults: serde_json::Map<_, _> = manifest
        .settings
        .iter()
        .map(|s| (s.key.clone(), s.default.clone()))
        .collect();
    assert_eq!(manifest.runtime.command, "device-battery.exe");
    assert_eq!(manifest.permissions, ["panel"]);
    for english in [false, true] {
        let mut value = serde_json::Value::Object(defaults.clone());
        value["language"] = json!(if english { "en" } else { "zh-CN" });
        value["diagnostics"] = json!(true);
        let settings = Settings::parse(&value);
        let mut state = State::default();
        for index in 0..16 {
            let id = format!("device-{index}");
            let mut d = Device {
                id: id.clone(),
                name: "VT0".into(),
                vid: 0x24ae,
                pid: 0x1410,
                present: true,
                available: true,
                reading: None,
                updated: 0,
                raw: String::new(),
            };
            if index != 0 {
                d.record(
                    Reading {
                        percent: 10,
                        charge: Charge::Discharging,
                        connection: Connection::Wireless,
                    },
                    &[7, 0x10, 3, 0x20, 3, 0x20, 3, 1, 10, 0, 4, 4, 0],
                    1_000,
                );
            }
            state.devices.insert(id, d);
        }
        state.unsupported = vec!["24AE:FFFF".into(); 20];
        for at in [2_000, 122_000] {
            let data = panel::snapshot(&state, &settings, at);
            assert_eq!(data["panel"]["attention"], at == 2_000);
            assert!(data["panel"]["rows"][0]["percent"].is_null());
            assert!(data["panel"]["rows"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("usage FF00:0002 · RID 07"));
            let rpc = json!({"jsonrpc":"2.0","method":"publish","params":data}).to_string();
            let parsed = host_protocol::parse_publish(&rpc, &manifest)
                .unwrap()
                .unwrap();
            assert_eq!(parsed.panel.unwrap().rows.len(), 16);
        }
        for state in [
            State::default(),
            State {
                failed: true,
                ..State::default()
            },
        ] {
            let rpc = json!({"jsonrpc":"2.0","method":"publish","params":panel::snapshot(&state, &settings, 2_000)}).to_string();
            assert!(host_protocol::parse_publish(&rpc, &manifest)
                .unwrap()
                .is_some());
        }
    }
}
