use fly_island_plugin_sdk::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct Settings {
    pub interval: Duration,
    pub mchose: bool,
    pub rapoo: bool,
    pub devices: Result<Vec<(u16, u16)>, ()>,
    pub low_percent: u8,
    pub diagnostics: bool,
    pub english: bool,
}

impl Settings {
    pub fn parse(value: &Value) -> Self {
        let seconds = match value["scan-seconds"].as_str() {
            Some("5") => 5,
            Some("60") => 60,
            _ => 30,
        };
        Self {
            interval: Duration::from_secs(seconds),
            mchose: value["mchose"].as_bool().unwrap_or(true),
            rapoo: value["rapoo"].as_bool().unwrap_or(true),
            devices: parse_devices(value["devices"].as_str().unwrap_or_default()),
            low_percent: value["low-percent"]
                .as_str()
                .and_then(|v| v.parse().ok())
                .filter(|v| (1..=50).contains(v))
                .unwrap_or(20),
            diagnostics: value["diagnostics"].as_bool().unwrap_or(false),
            english: value["language"] == "en",
        }
    }

    pub fn includes(&self, vid: u16, pid: u16) -> bool {
        self.devices
            .as_ref()
            .is_ok_and(|ids| ids.is_empty() || ids.contains(&(vid, pid)))
    }

    pub fn text<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        if self.english {
            en
        } else {
            zh
        }
    }
}

fn parse_devices(text: &str) -> Result<Vec<(u16, u16)>, ()> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    text.split([',', '，'])
        .map(|pair| {
            let (vid, pid) = pair.trim().split_once(':').ok_or(())?;
            let hex = |s: &str| {
                let s = s.trim();
                if s.len() != 4 || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err(());
                }
                u16::from_str_radix(s, 16).map_err(|_| ())
            };
            Ok((hex(vid)?, hex(pid)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fly_island_plugin_sdk::json;

    #[test]
    fn invalid_filters_fail_closed_and_settings_have_safe_defaults() {
        for input in ["garbage", "41e4", "41e4:2001,", "ffff:10000", "0x41e4:2001"] {
            let settings = Settings::parse(&json!({"devices":input}));
            assert!(settings.devices.is_err());
            assert!(!settings.includes(0x41e4, 0x2001));
        }
        let settings = Settings::parse(
            &json!({"devices":"41e4:2001， 24AE:1416", "scan-seconds":"0", "low-percent":"101"}),
        );
        assert!(settings.includes(0x24ae, 0x1416));
        assert!(!settings.includes(0x24ae, 0x1417));
        assert_eq!(settings.interval, Duration::from_secs(30));
        assert_eq!(settings.low_percent, 20);
        assert!(Settings::parse(&Value::Null).includes(0x41e4, 0x2001));
    }
}
