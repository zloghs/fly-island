mod catalog;
#[cfg(test)]
mod contract_tests;
mod hid;
mod k99_v3;
mod panel;
mod protocol;
mod settings;
mod state;

use fly_island_plugin_sdk::{json, now, run, Value};
use settings::Settings;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex, PoisonError,
    },
    time::Duration,
};

struct Monitor {
    settings: Settings,
    state: hid::SharedState,
    rescan: SyncSender<()>,
    stop: Arc<AtomicBool>,
    worker: std::thread::JoinHandle<()>,
}

impl Monitor {
    fn new(value: &Value) -> Self {
        let settings = Settings::parse(value);
        let state = Arc::new(Mutex::new(state::State {
            scanning: true,
            ..state::State::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (rescan, receiver) = mpsc::sync_channel(1);
        let worker = {
            let (settings, state, stop) = (settings.clone(), Arc::clone(&state), Arc::clone(&stop));
            std::thread::spawn(move || hid::watch(settings, state, receiver, stop))
        };
        Self {
            settings,
            state,
            rescan,
            stop,
            worker,
        }
    }

    fn snapshot(&self) -> Value {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if self.worker.is_finished() {
            state.failed = true;
            state.scanning = false;
        }
        panel::snapshot(&state, &self.settings, now())
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // A slow kernel feature read must not hold stdin EOF/shutdown hostage. The worker
        // closes its handles on exit; process termination also closes all native handles.
    }
}

fn main() {
    let monitor: Arc<Mutex<Option<Monitor>>> = Arc::new(Mutex::new(None));
    let poll = {
        let monitor = Arc::clone(&monitor);
        move |settings: &Value| {
            let mut slot = monitor.lock().unwrap_or_else(PoisonError::into_inner);
            Ok(slot
                .get_or_insert_with(|| Monitor::new(settings))
                .snapshot())
        }
    };
    let action = {
        let monitor = Arc::clone(&monitor);
        move |id: &str, settings: &Value| {
            if id != "refresh" {
                return Err("Unknown action".into());
            }
            let mut slot = monitor.lock().unwrap_or_else(PoisonError::into_inner);
            let monitor = slot.get_or_insert_with(|| Monitor::new(settings));
            match monitor.rescan.try_send(()) {
                Ok(()) | Err(mpsc::TrySendError::Full(())) => Ok(json!({})),
                Err(mpsc::TrySendError::Disconnected(())) => {
                    *slot = Some(Monitor::new(settings));
                    Ok(json!({}))
                }
            }
        }
    };
    // Hardware polling belongs to the worker; the RPC loop stays responsive at all scan intervals.
    if let Err(error) = run(Duration::from_secs(1), poll, action) {
        eprintln!("Device battery stopped: {error}");
        std::process::exit(1);
    }
}
