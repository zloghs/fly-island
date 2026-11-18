//! Model IDs and aliases observed in the official Rapoo HUB catalog on 2026-11-18.
//! Only mouse protocol VT_nrf54L (1); bases and other layouts are deliberately excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Mchose,
    MchoseK99V3,
    Rapoo,
}

pub fn identify(
    vid: u16,
    pid: u16,
    page: u16,
    usage: u16,
    product: Option<&str>,
) -> Option<(Profile, &'static str)> {
    if vid == 0x41e4 && pid == 0x2001 && page == 0xff04 {
        return Some((Profile::Mchose, "MCHOSE G87"));
    }
    // 3837:3033 is shared with other GLW receivers; PID alone cannot identify K99 V3.
    if vid == 0x3837
        && page == 1
        && usage == 0
        && (pid == 0x2020 || pid == 0x3033 && product == Some("MCHOSE K99 V3 2.4G"))
    {
        return Some((Profile::MchoseK99V3, "MCHOSE K99 V3"));
    }
    // Windows exposes each top-level collection separately. 000E is configuration;
    // periodic RID 07 battery reports arrive on the distinct 0002 status collection.
    if vid == 0x24ae && page == 0xff00 && usage == 0x02 {
        return RAPOO
            .iter()
            .find(|(wired, wireless, _)| *wired == pid || *wireless == Some(pid))
            .map(|(_, _, name)| (Profile::Rapoo, *name));
    }
    None
}

const RAPOO: &[(u16, Option<u16>, &str)] = &[
    (0x4610, Some(0x1410), "VT0"),
    (0x4611, Some(0x1411), "VT3S"),
    (0x4612, Some(0x1412), "VT3"),
    (0x4613, Some(0x1413), "VT7"),
    (0x4614, Some(0x1414), "VT9"),
    (0x4615, Some(0x1415), "VT1"),
    (0x4616, Some(0x1416), "VT7MAX"),
    (0x4617, Some(0x1417), "VT3MAX"),
    (0x4618, Some(0x1418), "VT9MAX"),
    (0x4619, Some(0x1419), "VT7Air"),
    (0x461a, Some(0x141a), "VT3Air"),
    (0x461b, Some(0x141b), "VT9Air"),
    (0x461c, Some(0x141c), "VT7Air MAX"),
    (0x461d, Some(0x141d), "VT3Air MAX"),
    (0x461e, Some(0x141e), "VT9Air MAX"),
    (0x4620, Some(0x1420), "VT0 MAX"),
    (0x4621, Some(0x1421), "VT3S MAX"),
    (0x4622, Some(0x1422), "VT2"),
    (0x4623, Some(0x1423), "VT2 MAX"),
    (0x4644, Some(0x1444), "VT1 MAX"),
    (0x4647, Some(0x1447), "VT0Air MAX MASTER"),
    (0x4648, None, "VT3SAir MAX MASTER"),
    (0x464a, Some(0x144a), "VT0 MAX MASTER"),
    (0x464b, Some(0x144b), "VT3S MAX MASTER"),
    (0x4651, Some(0x1451), "VT6"),
    (0x4652, Some(0x1452), "VT6MAX"),
    (0x4653, Some(0x1453), "VT9S V2"),
    (0x4659, Some(0x1459), "VT7S V2"),
    (0x465b, Some(0x145b), "VT4 V2"),
    (0x465d, Some(0x145d), "VT2S V2"),
    (0x4660, Some(0x1460), "VT7 V2"),
    (0x4662, Some(0x1462), "VT3 V2"),
    (0x4664, Some(0x1464), "VT3S V2"),
    (0x4666, Some(0x1466), "VT9 V2"),
    (0x4668, Some(0x1468), "VT2 V2"),
    (0x466d, Some(0x146d), "VT3 MAX MASTER V2"),
    (0x466e, Some(0x146e), "VT3S MAX MASTER V2"),
    (0x4670, Some(0x1470), "VT7 MAX MASTER V2"),
    (0x4671, Some(0x1471), "VT7S MAX MASTER V2"),
    (0x4673, Some(0x1473), "VT9 MAX MASTER V2"),
    (0x4674, Some(0x1474), "VT9S MAX MASTER V2"),
    (0x4675, Some(0x1475), "VT2 MAX MASTER V2"),
    (0x4676, Some(0x1476), "VT2S MAX MASTER V2"),
    (0x46a1, Some(0x14a1), "VT0Air MAX"),
    (0x46a2, Some(0x14a2), "VT3SAir MAX"),
    (0x46a4, Some(0x14a4), "VT2K MAX"),
    (0x46a5, Some(0x14a5), "VT3PRO V2"),
    (0x46a6, Some(0x14a6), "VT3S PRO V2"),
    (0x46a7, Some(0x14a7), "VT7PRO V2"),
    (0x46a8, Some(0x14a8), "VT7S PRO V2"),
    (0x46a9, Some(0x14a9), "VT9PRO V2"),
    (0x46aa, Some(0x14aa), "VT9S PRO V2"),
    (0x46ab, Some(0x14ab), "VT7K Air MAX V2"),
    (0x46ac, Some(0x14ac), "VT2K Air MAX V2"),
    (0x4b0c, Some(0x1b0c), "VT3Air MAX V2"),
    (0x4b0d, Some(0x1b0d), "VT3SAir MAX V2"),
    (0x4b0e, Some(0x1b0e), "VT7Air MAX V2"),
    (0x4b0f, Some(0x1b0f), "VT7SAir MAX V2"),
    (0x4b10, Some(0x1b10), "VT9Air MAX V2"),
    (0x4b11, Some(0x1b11), "VT9SAir MAX V2"),
    (0x4b12, Some(0x1b12), "VT2Air MAX V2"),
    (0x4b13, Some(0x1b13), "VT2SAir MAX V2"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_models_require_the_vendor_collection_and_unknown_devices_are_not_opened() {
        assert_eq!(
            identify(0x41e4, 0x2001, 0xff04, 1, None),
            Some((Profile::Mchose, "MCHOSE G87"))
        );
        assert_eq!(
            identify(0x24ae, 0x1410, 0xff00, 2, None),
            Some((Profile::Rapoo, "VT0"))
        );
        assert_eq!(
            identify(0x24ae, 0x4610, 0xff00, 2, None),
            Some((Profile::Rapoo, "VT0"))
        );
        for args in [
            (0x24ae, 0x1416, 1, 2),
            (0x24ae, 0x1416, 0xff00, 14),
            (0x24ae, 0x144d, 0xff00, 2),
            (0x41e4, 0xffff, 0xff04, 1),
        ] {
            assert!(identify(args.0, args.1, args.2, args.3, None).is_none());
        }
    }
    #[test]
    fn rapoo_status_and_configuration_collections_never_create_duplicate_rows() {
        // Receiver and cable each expose separate status/configuration collections.
        for pid in [0x1410, 0x4610] {
            let collections = [(1, 2), (0xff00, 14), (0xff00, 2), (0xff00, 1)];
            let selected: Vec<_> = collections
                .into_iter()
                .filter(|(page, usage)| identify(0x24ae, pid, *page, *usage, None).is_some())
                .collect();
            assert_eq!(selected, [(0xff00, 2)]);
        }
    }
    #[test]
    fn catalog_ids_do_not_overlap() {
        let mut ids = std::collections::HashSet::new();
        for (wired, wireless, _) in RAPOO {
            assert!(ids.insert(*wired));
            if let Some(wireless) = wireless {
                assert!(ids.insert(*wireless));
            }
        }
    }
    #[test]
    fn k99_v3_requires_its_command_collection_and_exact_shared_receiver_name() {
        assert_eq!(
            identify(0x3837, 0x3033, 1, 0, Some("MCHOSE K99 V3 2.4G")),
            Some((Profile::MchoseK99V3, "MCHOSE K99 V3"))
        );
        assert_eq!(
            identify(0x3837, 0x2020, 1, 0, None),
            Some((Profile::MchoseK99V3, "MCHOSE K99 V3"))
        );
        for name in [
            None,
            Some("MCHOSE G75 V2 2.4G"),
            Some("MCHOSE K99 V3 Upgrader"),
        ] {
            assert!(identify(0x3837, 0x3033, 1, 0, name).is_none());
        }
        assert!(identify(0x3837, 0x3033, 1, 2, Some("MCHOSE K99 V3 2.4G")).is_none());
        assert!(identify(0x3837, 0x2010, 0xff00, 1, Some("MCHOSE K99 V3 2.4G")).is_none());
    }
}
