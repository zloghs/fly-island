//! Battery queries only; K99 V3 uses a fixed read command, never configuration or firmware writes.
use crate::{settings::Settings, state::State};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
    Arc, Mutex, PoisonError,
};

pub type SharedState = Arc<Mutex<State>>;

#[cfg(windows)]
pub fn watch(settings: Settings, shared: SharedState, rescan: Receiver<()>, stop: Arc<AtomicBool>) {
    use crate::{
        catalog::{identify, Profile},
        k99_v3, protocol,
        state::Device,
    };
    use fly_island_plugin_sdk::{now, sha256};
    use hidapi::{HidApi, HidDevice};
    use std::{
        collections::BTreeMap,
        time::{Duration, Instant},
    };

    struct OpenDevice {
        handle: HidDevice,
        profile: Profile,
        pending_until: Option<Instant>,
    }
    if settings.devices.is_err() || !(settings.mchose || settings.rapoo) {
        *shared.lock().unwrap_or_else(PoisonError::into_inner) = State::default();
        while !stop.load(Ordering::Acquire) {
            // Disabled providers and invalid filters must never enumerate or open hardware.
            let _ = rescan.recv_timeout(Duration::from_secs(1));
        }
        return;
    }
    let mut state = State::default();
    let mut handles: BTreeMap<String, OpenDevice> = BTreeMap::new();
    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(_) => {
            *shared.lock().unwrap_or_else(PoisonError::into_inner) = State {
                failed: true,
                ..state
            };
            return;
        }
    };
    let mut next_scan = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if rescan.try_recv().is_ok() {
            next_scan = Instant::now();
        }
        if Instant::now() >= next_scan {
            state.scanning = true;
            *shared.lock().unwrap_or_else(PoisonError::into_inner) = state.clone();
            if api.refresh_devices().is_err() {
                state.failed = true;
                for device in state.devices.values_mut() {
                    device.available = false;
                }
                handles.clear();
            } else {
                state.failed = false;
                state.unsupported.clear();
                for device in state.devices.values_mut() {
                    device.present = false;
                }
                let mut present = std::collections::HashSet::new();
                let mut infos: Vec<_> = api.device_list().collect();
                infos.sort_by(|a, b| a.path().to_bytes().cmp(b.path().to_bytes()));
                for info in infos {
                    let (vid, pid) = (info.vendor_id(), info.product_id());
                    if !settings.includes(vid, pid)
                        || !([0x41e4, 0x3837].contains(&vid) && settings.mchose
                            || vid == 0x24ae && settings.rapoo)
                    {
                        continue;
                    }
                    let Some((profile, name)) = identify(
                        vid,
                        pid,
                        info.usage_page(),
                        info.usage(),
                        info.product_string(),
                    ) else {
                        // K99 V3's command collection uses Generic Desktop / Undefined.
                        if (info.usage_page() >= 0xff00
                            || info.usage_page() == 1 && info.usage() == 0)
                            && state.unsupported.len() < 16
                        {
                            let detail = format!(
                                "{vid:04X}:{pid:04X} · usage {:04X}:{:04X}",
                                info.usage_page(),
                                info.usage()
                            );
                            if !state.unsupported.contains(&detail) {
                                state.unsupported.push(detail);
                            }
                        }
                        continue;
                    };
                    let hash = sha256(info.path().to_bytes());
                    let id: String = hash[..16].iter().map(|b| format!("{b:02x}")).collect();
                    if present.len() >= 16 || !present.insert(id.clone()) {
                        continue;
                    }
                    let device = state.devices.entry(id.clone()).or_insert_with(|| Device {
                        id: id.clone(),
                        name: name.into(),
                        vid,
                        pid,
                        present: true,
                        available: true,
                        reading: None,
                        updated: 0,
                        raw: String::new(),
                    });
                    device.present = true;
                    if !handles.contains_key(&id) {
                        match info.open_device(&api).and_then(|handle| {
                            handle.set_blocking_mode(false)?;
                            Ok(handle)
                        }) {
                            Ok(handle) => {
                                if profile == Profile::MchoseK99V3 {
                                    let mut descriptor = [0u8; 4096];
                                    let valid = handle
                                        .get_report_descriptor(&mut descriptor)
                                        .ok()
                                        .and_then(|size| descriptor.get(..size))
                                        .is_some_and(k99_v3::accepts_descriptor);
                                    if !valid {
                                        device.available = false;
                                        continue;
                                    }
                                }
                                handles.insert(
                                    id.clone(),
                                    OpenDevice {
                                        handle,
                                        profile,
                                        pending_until: None,
                                    },
                                );
                                // Reopening a receiver does not make its old sample current.
                                device.available = device.reading.is_none();
                            }
                            Err(_) => {
                                device.available = false;
                                continue;
                            }
                        }
                    }
                    if profile == Profile::Mchose {
                        let mut raw = [0u8; 8];
                        raw[0] = 6;
                        let size = handles
                            .get(&id)
                            .and_then(|open| open.handle.get_feature_report(&mut raw).ok());
                        let report = size.and_then(|size| raw.get(..size));
                        let reading = report.and_then(protocol::mchose);
                        if let Some(reading) = reading {
                            device.record(reading, &raw, now());
                        } else {
                            device.reject(report.unwrap_or_default());
                            handles.remove(&id);
                        }
                    } else if profile == Profile::MchoseK99V3 {
                        if let Some(open) = handles.get_mut(&id) {
                            // Discard queued replies before starting a new query, with a strict bound.
                            let mut raw = [0u8; 65];
                            for _ in 0..32 {
                                if !matches!(open.handle.read_timeout(&mut raw, 0), Ok(n) if n > 0)
                                {
                                    break;
                                }
                            }
                            let request = k99_v3::request();
                            if open.handle.write(&request).ok() == Some(request.len()) {
                                open.pending_until =
                                    Some(Instant::now() + Duration::from_millis(1500));
                            } else {
                                device.available = false;
                                handles.remove(&id);
                            }
                        }
                    }
                }
                handles.retain(|id, _| present.contains(id));
                // Prefer connected receivers; bound cached disconnected rows to ten minutes/16 rows.
                let mut old: Vec<_> = state
                    .devices
                    .values()
                    .filter(|d| !d.present && now().saturating_sub(d.updated) <= 600_000)
                    .map(|d| (d.updated, d.id.clone()))
                    .collect();
                old.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
                let keep: std::collections::HashSet<_> = old
                    .into_iter()
                    .take(16usize.saturating_sub(present.len()))
                    .map(|(_, id)| id)
                    .collect();
                state.devices.retain(|id, d| d.present || keep.contains(id));
            }
            state.scanning = false;
            next_scan = Instant::now() + settings.interval;
        }
        let mut failed_handles = Vec::new();
        for (id, open) in &mut handles {
            if open.profile == Profile::Mchose {
                continue;
            }
            for _ in 0..8 {
                let mut raw = [0u8; 65];
                match open.handle.read_timeout(&mut raw, 0) {
                    Ok(0) => break,
                    Ok(size) => {
                        if let Some(device) = state.devices.get_mut(id) {
                            let report = &raw[..size];
                            let reading = match open.profile {
                                Profile::Rapoo => protocol::rapoo(report),
                                Profile::MchoseK99V3
                                    if open
                                        .pending_until
                                        .is_some_and(|end| Instant::now() <= end) =>
                                {
                                    k99_v3::parse(report, device.pid == 0x2020)
                                }
                                _ => None,
                            };
                            if let Some(reading) = reading {
                                device.record(reading, &raw[..size], now());
                                open.pending_until = None;
                            } else if open.profile == Profile::Rapoo && raw[0] == 7
                                || open.profile == Profile::MchoseK99V3
                                    && open.pending_until.is_some()
                                    && k99_v3::matches(report)
                            {
                                device.reject(&raw[..size]);
                            }
                        }
                    }
                    Err(_) => {
                        if let Some(device) = state.devices.get_mut(id) {
                            device.available = false;
                        }
                        failed_handles.push(id.clone());
                        break;
                    }
                }
            }
            if open.pending_until.is_some_and(|end| Instant::now() > end) {
                if let Some(device) = state.devices.get_mut(id) {
                    device.available = false;
                }
                open.pending_until = None;
            }
        }
        for id in failed_handles {
            handles.remove(&id);
        }
        *shared.lock().unwrap_or_else(PoisonError::into_inner) = state.clone();
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(not(windows))]
pub fn watch(
    _settings: Settings,
    shared: SharedState,
    _rescan: Receiver<()>,
    _stop: Arc<AtomicBool>,
) {
    *shared.lock().unwrap_or_else(PoisonError::into_inner) = State {
        failed: true,
        ..State::default()
    };
}
