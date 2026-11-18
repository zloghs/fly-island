//! M HUB GLW getFuncConfig: read one packed battery byte at profile 0 offset 32.
//! Source: cizhou/CZ_SHARED_DATA/main.002bbdb0f17b5f7135ee.js (2026-11-18).
use crate::protocol::{Charge, Connection, Reading};

pub fn request() -> [u8; 65] {
    // Windows/hidapi needs a zero Report ID before the unnumbered 64-byte frame.
    // Header: flag, command, reserved, sum(length/offset), length, offset LE, reserved.
    let mut bytes = [0; 65];
    bytes[1..9].copy_from_slice(&[0x55, 0x05, 0, 0x21, 1, 0x20, 0, 0]);
    bytes
}

pub fn matches(report: &[u8]) -> bool {
    // hidapi removes the synthetic zero ID from unnumbered input reports.
    report.len() == 64
        && report[0] == 0xaa
        && report[1] == 5
        && report[4] == 1
        && report[5] == 32
        && report[6] == 0
}

pub fn parse(report: &[u8], wired: bool) -> Option<Reading> {
    if !matches(report) {
        return None;
    }
    let percent = report[8] & 0x7f;
    if percent > 100 {
        return None;
    }
    Some(Reading {
        percent,
        charge: if report[8] & 0x80 != 0 {
            Charge::Charging
        } else {
            Charge::Discharging
        },
        connection: if wired {
            Connection::Usb
        } else {
            Connection::Wireless
        },
    })
}

/// Accept only an unnumbered collection with exactly 64-byte input AND output reports.
/// Parse HID items rather than matching a byte sequence: Windows reconstructs descriptors.
pub fn accepts_descriptor(bytes: &[u8]) -> bool {
    let (mut size, mut count) = (0u32, 0u32);
    let (mut input_bits, mut output_bits) = (0u32, 0u32);
    let mut stack = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let prefix = bytes[offset];
        if prefix == 0xfe {
            return false;
        }
        let length = match prefix & 3 {
            3 => 4,
            v => v as usize,
        };
        let Some(data) = bytes.get(offset + 1..offset + 1 + length) else {
            return false;
        };
        let value = data
            .iter()
            .enumerate()
            .fold(0, |v, (i, b)| v | ((*b as u32) << (8 * i)));
        match prefix & 0xfc {
            0x74 => size = value,
            0x94 => count = value,
            0x84 => return false, // Numbered reports are a different protocol.
            0xa4 if stack.len() < 16 => stack.push((size, count)),
            0xa4 => return false,
            0xb4 => match stack.pop() {
                Some(v) => (size, count) = v,
                None => return false,
            },
            tag @ (0x80 | 0x90) => {
                let Some(bits) = size.checked_mul(count) else {
                    return false;
                };
                let total = if tag == 0x80 {
                    &mut input_bits
                } else {
                    &mut output_bits
                };
                let Some(next) = total.checked_add(bits).filter(|n| *n <= 512) else {
                    return false;
                };
                *total = next;
            }
            _ => {}
        }
        offset += length + 1;
    }
    stack.is_empty() && input_bits == 512 && output_bits == 512
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_reads_one_byte_without_a_configuration_payload() {
        let raw = request();
        assert_eq!(&raw[..9], &[0, 0x55, 5, 0, 0x21, 1, 32, 0, 0]);
        assert!(raw[9..].iter().all(|v| *v == 0));
        assert_eq!(raw.len(), 65);
    }

    fn response(value: u8) -> [u8; 64] {
        let mut report = [0; 64];
        report[..8].copy_from_slice(&[0xaa, 5, 0, value.wrapping_add(33), 1, 32, 0, 0]);
        report[8] = value;
        report
    }

    #[test]
    fn packed_battery_separates_percent_from_charge_and_rejects_other_commands() {
        for percent in [0, 1, 40, 100] {
            for charging in [false, true] {
                let report = response(percent | if charging { 0x80 } else { 0 });
                let reading = parse(&report, false).unwrap();
                assert_eq!(reading.percent, percent);
                assert_eq!(
                    reading.charge,
                    if charging {
                        Charge::Charging
                    } else {
                        Charge::Discharging
                    }
                );
                assert_eq!(reading.connection, Connection::Wireless);
                assert_eq!(parse(&report, true).unwrap().connection, Connection::Usb);
            }
        }
        assert!(parse(&response(101), false).is_none());
        assert!(parse(&response(255), false).is_none());
        assert!(parse(&response(50)[..63], false).is_none());
        for (index, value) in [(0, 0x55), (1, 3), (4, 0), (4, 56), (5, 0), (6, 1)] {
            let mut report = response(50);
            report[index] = value;
            assert!(parse(&report, false).is_none());
        }
        let mut info = response(100);
        info[1] = 3; // getInfo payload begins with firmware version, NOT battery.
        assert!(parse(&info, false).is_none());
    }

    #[test]
    fn descriptor_gate_accepts_item_reordering_and_rejects_boot_mouse_and_report_ids() {
        let descriptor = [
            5, 1, 9, 0, 0xa1, 1, 0x75, 8, 0x95, 64, 0x81, 2, 0x91, 2, 0xc0,
        ];
        assert!(accepts_descriptor(&descriptor));
        let reordered = [
            5, 1, 9, 0, 0xa1, 1, 0x95, 64, 0x75, 8, 0x81, 2, 0x91, 2, 0xc0,
        ];
        assert!(accepts_descriptor(&reordered));
        assert!(!accepts_descriptor(&descriptor[..12]));
        assert!(!accepts_descriptor(&[0x75, 8, 0x95, 3, 0x81, 2]));
        let mut numbered = vec![0x85, 1];
        numbered.extend(descriptor);
        assert!(!accepts_descriptor(&numbered));
        assert!(!accepts_descriptor(&[0x77, 255]));
        assert!(!accepts_descriptor(&[0xb4]));
        assert!(!accepts_descriptor(&[0xfe, 0, 0]));
    }
}
