use crate::{
    protocol::{Charge, Connection},
    settings::Settings,
    state::{Device, State},
};
use fly_island_plugin_sdk::{json, Value};

pub fn snapshot(state: &State, settings: &Settings, at: i64) -> Value {
    let t = |zh, en| settings.text(zh, en);
    let devices: Vec<_> = state.devices.values().collect();
    let online = devices.iter().filter(|d| d.fresh(at)).count();
    let attention = devices.iter().any(|d| d.low(at, settings.low_percent));
    let headline = if settings.devices.is_err() {
        t("设备筛选格式有误", "Invalid device filter").to_owned()
    } else if state.failed {
        t("无法扫描 HID 设备", "HID scan unavailable").to_owned()
    } else if devices.is_empty() {
        t("未发现支持的设备", "No supported devices found").to_owned()
    } else if settings.english {
        format!("{online} / {} batteries available", devices.len())
    } else {
        format!("{online} / {} 台设备有有效电量", devices.len())
    };
    let mut note = if settings.devices.is_err() {
        t(
            "请填写如 3837:3033,24AE:1410",
            "Use pairs such as 3837:3033,24AE:1410",
        )
        .to_owned()
    } else if state.failed {
        t(
            "扫描失败，保留上次读数；请重新扫描",
            "Scan failed; keeping previous readings. Try Rescan.",
        )
        .to_owned()
    } else {
        t(
            "迈从 K99 V3 / G87 · 雷柏 VT 二代；唤醒设备后等待状态上报",
            "MCHOSE K99 V3 / G87 · Rapoo VT Gen 2; wake the device and wait for status",
        )
        .to_owned()
    };
    if state.scanning {
        note = t("正在扫描并读取设备…", "Scanning and reading devices…").to_owned();
    }
    let mut rows: Vec<_> = devices
        .iter()
        .take(16)
        .map(|d| row(d, settings, at))
        .collect();
    if settings.diagnostics {
        for (index, info) in state.unsupported.iter().take(16 - rows.len()).enumerate() {
            rows.push(json!({"id":format!("unsupported-{index}"), "title":info,
                "detail":t("暂无适配，仅枚举，未向设备发送命令", "Unsupported; enumerated only, no commands sent"),
                "percent":null, "countdownTo":null, "tone":"normal"}));
        }
    }
    json!({"panel":{"headline":headline, "note":note, "rows":rows,
        "actions":["refresh"], "attention":attention}})
}

fn row(device: &Device, settings: &Settings, at: i64) -> Value {
    let t = |zh, en| settings.text(zh, en);
    let mut detail = if let Some(reading) = device.reading {
        let connection = match reading.connection {
            Connection::Wireless => "2.4G",
            Connection::Usb => "USB",
            Connection::Bluetooth => "Bluetooth",
        };
        let status = if !device.present {
            t("接收器已断开", "Receiver disconnected")
        } else if !device.available {
            t(
                "等待有效报告／读取失败",
                "Waiting for valid report / read failed",
            )
        } else if !device.fresh(at) {
            t("等待上报／可能休眠", "Waiting for report / possibly asleep")
        } else {
            match reading.charge {
                Charge::Discharging => t("使用中", "On battery"),
                Charge::Charging => t("充电中", "Charging"),
                Charge::Full => t("已充满", "Fully charged"),
                Charge::Unknown => t("充电状态未知", "Charge state unknown"),
            }
        };
        // Minute-level ages keep identical publishes from waking the island every second.
        let minutes = at.saturating_sub(device.updated).max(0) / 60_000;
        let age = if minutes == 0 {
            t("不足 1 分钟前", "less than 1 min ago").to_owned()
        } else if settings.english {
            format!("{minutes} min ago")
        } else {
            format!("{minutes} 分钟前")
        };
        if settings.english {
            format!(
                "{connection} · {status} · last read {}% ({age})",
                reading.percent
            )
        } else {
            format!(
                "{connection} · {status} · 上次 {}%（{age}）",
                reading.percent
            )
        }
    } else if !device.present {
        t(
            "接收器已断开 · 尚无电量读数",
            "Receiver disconnected · no battery reading yet",
        )
        .to_owned()
    } else if !device.available {
        t(
            "暂无有效读数，请唤醒设备或关闭驱动后重新扫描",
            "No valid reading; wake the device or close its driver and rescan",
        )
        .to_owned()
    } else {
        t(
            "等待电量报告；唤醒后仍无读数时请开启诊断",
            "Waiting for battery status; enable diagnostics if waking does not help",
        )
        .to_owned()
    };
    if settings.diagnostics {
        detail.push_str(&format!(
            " · {:04X}:{:04X} · {}",
            device.vid,
            device.pid,
            if device.raw.is_empty() {
                "—"
            } else {
                &device.raw
            }
        ));
        if device.vid == 0x24ae {
            detail.push_str(" · usage FF00:0002 · RID 07");
        }
    }
    json!({"id":device.id, "title":device.name, "detail":detail,
        "percent":device.reading.map(|r| r.percent), "countdownTo":null,
        "tone":if device.low(at, settings.low_percent) {"warning"} else {"normal"}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_failed_and_bad_filter_panels_are_bilingual_and_never_fake_battery() {
        for language in ["zh-CN", "en"] {
            let settings = Settings::parse(&json!({"language":language}));
            let empty = snapshot(&State::default(), &settings, 1_000);
            assert_eq!(empty["panel"]["rows"], json!([]));
            assert_eq!(empty["panel"]["attention"], false);
            let failed = snapshot(
                &State {
                    failed: true,
                    ..State::default()
                },
                &settings,
                1_000,
            );
            assert_ne!(failed["panel"]["headline"], empty["panel"]["headline"]);
            let bad = Settings::parse(&json!({"language":language,"devices":"bad"}));
            assert_ne!(
                snapshot(&State::default(), &bad, 1_000)["panel"]["headline"],
                empty["panel"]["headline"]
            );
        }
    }
}
