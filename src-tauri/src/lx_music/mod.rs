//! LX Music as a music host. Windows already lists it among the media sessions, but the session
//! carries no lyrics, so they come from LX's own local API ("开放 API 服务" in its settings, off by
//! default, port 23330) instead of a search by name. That answer is the song LX itself shows — no
//! guessing, and the translation comes with it.
//!
//! Requests go to 127.0.0.1 only, without a proxy and with a short timeout: the API is local, and a
//! miss only means the user has it switched off, in which case the lyrics are looked up elsewhere.

mod client;

#[cfg(test)]
pub(crate) mod test_server;

pub(crate) use client::{LxLyricAll, LxStatus};

/// The port the API listens on until the user changes it in LX Music's own settings.
pub(crate) const DEFAULT_PORT: u16 = 23330;

/// Whether an SMTC source app is LX Music: the original (`cn.toside.music.desktop`) and the NetEase
/// build (`cn.toside.music.desktop.netease`) share the first AUMID prefix, and both executables
/// carry `lx-music-desktop` in their name.
pub(crate) fn is_app(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    id.contains("toside.music.desktop") || id.contains("lx-music-desktop")
}

/// The port to ask, with 0 (an empty setting) meaning the default.
pub(crate) fn port_or_default(port: Option<u16>) -> u16 {
    port.filter(|port| *port > 0).unwrap_or(DEFAULT_PORT)
}

/// What LX plays now, and the length of the song.
pub(crate) async fn status(port: u16) -> Result<LxStatus, String> {
    client::status(port).await
}

/// The LRC text LX shows for its current song, with the translation.
pub(crate) async fn lyrics(port: u16) -> Result<LxLyricAll, String> {
    client::lyrics(port).await
}

/// Whether LX's API answers on `port`, and what it plays; the settings row uses it to tell the user
/// the port is right before a song needs lyrics. `None` asks the default port.
#[tauri::command]
pub async fn lx_music_status(port: Option<u16>) -> Result<LxStatus, String> {
    status(port_or_default(port)).await
}

/// This module's Tauri commands, chained into `lib.rs`'s `generate_handler!`.
macro_rules! tauri_commands {
    ($next:ident [$($done:tt)*] $($rest:tt)*) => {
        $next!([$($done)*
            crate::lx_music::lx_music_status,
        ] $($rest)*)
    };
}
pub(crate) use tauri_commands;

#[cfg(test)]
mod tests {
    use super::{is_app, port_or_default, DEFAULT_PORT};

    #[test]
    fn both_lx_music_builds_are_recognized() {
        assert!(is_app("cn.toside.music.desktop"));
        assert!(is_app("cn.toside.music.desktop.netease"));
        assert!(is_app("lx-music-desktop-netease.exe"));
        assert!(is_app("C:\\Program Files\\LX Music\\lx-music-desktop.exe"));
        assert!(!is_app("cloudmusic.exe"));
        assert!(!is_app("QQMusic.exe"));
    }

    #[test]
    fn an_empty_port_means_the_default() {
        assert_eq!(port_or_default(None), DEFAULT_PORT);
        assert_eq!(port_or_default(Some(0)), DEFAULT_PORT);
        assert_eq!(port_or_default(Some(24000)), 24000);
    }
}
