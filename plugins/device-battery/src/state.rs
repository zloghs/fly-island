use crate::protocol::{Charge, Reading};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub present: bool,
    pub available: bool,
    pub reading: Option<Reading>,
    pub updated: i64,
    pub raw: String,
}

impl Device {
    pub fn record(&mut self, reading: Reading, raw: &[u8], at: i64) {
        self.reading = Some(reading);
        self.updated = at;
        self.available = true;
        self.store_raw(raw);
    }

    pub fn reject(&mut self, raw: &[u8]) {
        self.available = false;
        self.store_raw(raw);
    }

    fn store_raw(&mut self, raw: &[u8]) {
        self.raw = raw
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
    }

    pub fn fresh(&self, at: i64) -> bool {
        self.present
            && self.available
            && self.reading.is_some()
            && at.saturating_sub(self.updated) <= 120_000
    }

    pub fn low(&self, at: i64, threshold: u8) -> bool {
        self.fresh(at)
            && self.reading.is_some_and(|r| {
                r.percent <= threshold && !matches!(r.charge, Charge::Charging | Charge::Full)
            })
    }
}

#[derive(Clone, Default)]
pub struct State {
    pub devices: BTreeMap<String, Device>,
    pub scanning: bool,
    pub failed: bool,
    pub unsupported: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Connection;

    pub fn device() -> Device {
        Device {
            id: "test".into(),
            name: "Keyboard".into(),
            vid: 0x41e4,
            pid: 0x2001,
            present: true,
            available: true,
            reading: None,
            updated: 0,
            raw: String::new(),
        }
    }

    #[test]
    fn failures_keep_the_last_sample_and_do_not_create_zero_or_fresh_attention() {
        let mut d = device();
        assert!(!d.low(1_000, 20));
        d.record(
            Reading {
                percent: 10,
                charge: Charge::Discharging,
                connection: Connection::Wireless,
            },
            &[6, 0, 0, 10],
            1_000,
        );
        assert!(d.low(2_000, 20));
        assert!(!d.low(122_000, 20));
        d.reject(&[7, 0, 0]);
        assert!(!d.low(2_000, 20));
        assert_eq!(d.reading.unwrap().percent, 10);
        assert_eq!(d.updated, 1_000);
        assert_eq!(d.raw, "07 00 00");
        d.available = true;
        d.present = false;
        assert!(!d.low(2_000, 20));
        d.present = true;
        d.record(
            Reading {
                percent: 0,
                charge: Charge::Charging,
                connection: Connection::Usb,
            },
            &[7],
            3_000,
        );
        assert!(!d.low(3_000, 20));
    }
}
