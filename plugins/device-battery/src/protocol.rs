//! Wire formats independently implemented from the referenced protocol observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charge {
    Discharging,
    Charging,
    Full,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Wireless,
    Usb,
    Bluetooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    pub percent: u8,
    pub charge: Charge,
    pub connection: Connection,
}

pub fn mchose(report: &[u8]) -> Option<Reading> {
    // Windows/hidapi includes the report ID; G87 has an eight-byte feature report.
    if report.len() < 8 || report[0] != 0x06 || report[3] > 100 {
        return None;
    }
    let charge = match report[2] {
        0 => Charge::Discharging,
        1 => Charge::Charging,
        2 if report[3] == 100 => Charge::Full,
        _ => Charge::Unknown,
    };
    Some(Reading {
        percent: report[3],
        charge,
        connection: Connection::Wireless,
    })
}

pub fn rapoo(report: &[u8]) -> Option<Reading> {
    // Official VT_nrf54L parser: WebHID payload offsets 6=status, 7=percent.
    // hidapi prepends RID 7. Do not apply this layout to bases or other protocols.
    if report.len() < 13 || report[0] != 0x07 || report[8] > 100 {
        return None;
    }
    let connection = match report[1] & 0x0f {
        0 => Connection::Wireless,
        1 => Connection::Bluetooth,
        2 => Connection::Usb,
        _ => return None,
    };
    let charge = match report[7] {
        1 => Charge::Discharging,
        2 => Charge::Charging,
        _ => return None, // status 0 is invalid/offline, including its zero percent.
    };
    Some(Reading {
        percent: report[8],
        charge,
        connection,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mchose_offsets_and_state_do_not_confuse_full_with_charging() {
        let mut raw = [6, 0, 2, 100, 100, 100, 101, 90];
        assert_eq!(mchose(&raw).unwrap().charge, Charge::Full);
        raw[3] = 70;
        assert_eq!(mchose(&raw).unwrap().percent, 70);
        assert_eq!(mchose(&raw).unwrap().charge, Charge::Unknown);
        raw[2] = 1;
        assert_eq!(mchose(&raw).unwrap().charge, Charge::Charging);
        raw[2] = 0;
        raw[3] = 0;
        assert_eq!(mchose(&raw).unwrap().percent, 0);
        raw[3] = 255;
        assert!(mchose(&raw).is_none());
        raw[0] = 7;
        assert!(mchose(&raw).is_none());
        assert!(mchose(&[6, 0, 1, 50]).is_none());
    }

    #[test]
    fn rapoo_accounts_for_report_id_and_rejects_invalid_status() {
        let mut raw = [7, 0x10, 3, 0x20, 3, 0x20, 3, 1, 78, 0, 4, 4, 0];
        assert_eq!(
            rapoo(&raw),
            Some(Reading {
                percent: 78,
                charge: Charge::Discharging,
                connection: Connection::Wireless
            })
        );
        raw[1] = 0x12;
        raw[7] = 2;
        assert_eq!(rapoo(&raw).unwrap().connection, Connection::Usb);
        assert_eq!(rapoo(&raw).unwrap().charge, Charge::Charging);
        for status in [0, 3, 255] {
            raw[7] = status;
            assert!(rapoo(&raw).is_none());
        }
        raw[7] = 1;
        raw[8] = 0;
        assert_eq!(rapoo(&raw).unwrap().percent, 0);
        raw[8] = 101;
        assert!(rapoo(&raw).is_none());
        assert!(rapoo(&raw[..8]).is_none());
        raw[8] = 50;
        raw[0] = 8;
        assert!(rapoo(&raw).is_none());
        raw[0] = 7;
        raw[1] = 0xff;
        assert!(rapoo(&raw).is_none());
    }

    #[test]
    fn rapoo_captured_status_frames_accept_both_receiver_lengths_and_cable() {
        // Windows FF00:0002 captures: docs/rapoo.md in Wireless-Device-Battery-Monitor.
        let receiver_19 = [
            0x07, 0x20, 0x03, 0x20, 0x03, 0x20, 0x03, 0x01, 0x19, 0x00, 0x04, 0x84, 0x00, 0x17,
            0x46, 0x00, 0x02, 0x03, 0xbc,
        ];
        let receiver_16 = [
            0x07, 0x10, 0x03, 0x40, 0x06, 0x40, 0x06, 0x01, 0x64, 0x00, 0x03, 0x01, 0x00, 0x14,
            0x46, 0x01,
        ];
        for (report, percent) in [(receiver_19.as_slice(), 25), (&receiver_16, 100)] {
            assert_eq!(
                rapoo(report),
                Some(Reading {
                    percent,
                    charge: Charge::Discharging,
                    connection: Connection::Wireless,
                })
            );
        }
        let mut cable = receiver_19;
        cable[1] = 0x22;
        cable[7] = 2;
        assert_eq!(
            rapoo(&cable),
            Some(Reading {
                percent: 25,
                charge: Charge::Charging,
                connection: Connection::Usb,
            })
        );
        for len in 0..13 {
            assert!(rapoo(&receiver_19[..len]).is_none());
        }
    }
}
