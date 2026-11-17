use super::{any_listen_line, fetch, matches_track};
use crate::lx_music::{LxStatus, test_server::{json, Server}};

const STATUS: &str = r#"{"name":"Song A","singer":"Artist","albumName":"Album","duration":180}"#;
const LYRIC: &str = r#"{"lyric":"[00:01.00]sung words"}"#;

#[test]
fn a_track_change_during_the_lyric_request_is_rejected() {
    let server = Server::new(vec![json(STATUS), json(LYRIC), json(&STATUS.replace("Song A", "Song B"))]);
    let result = tauri::async_runtime::block_on(fetch(server.port, "Song A", "Artist", 180_000, "Album")).unwrap();
    assert!(result.is_none());
    let requests = server.finish();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].starts_with("GET /status?"));
    assert!(requests[1].starts_with("GET /lyric-all "));
    assert!(requests[2].starts_with("GET /status?"));
}

#[test]
fn a_stable_track_keeps_its_lyrics_and_duration() {
    let server = Server::new(vec![json(STATUS), json(LYRIC), json(STATUS)]);
    let found = tauri::async_runtime::block_on(fetch(server.port, "Song A", "Artist", 180_000, "Album")).unwrap().unwrap();
    assert!(found.lrc.contains("sung words"));
    assert_eq!(found.duration_ms, Some(180_000));
    assert_eq!(server.finish().len(), 3);
}

#[test]
fn versions_and_durations_are_not_fuzzy_search_matches() {
    let status: LxStatus = serde_json::from_str(STATUS).unwrap();
    assert!(matches_track(&status, "Song A", "Artist", 180_000));
    assert!(!matches_track(&status, "Song A (Live)", "Artist", 180_000));
    assert!(!matches_track(&status, "Song A", "Artist", 230_000));
    assert!(!matches_track(&status, "Song A", "Other", 180_000));
    assert!(!matches_track(&status, "Song A", "Artist / Other", 180_000));
}

#[test]
fn a_different_album_cannot_supply_the_lyrics() {
    let server = Server::new(vec![json(STATUS)]);
    let result = tauri::async_runtime::block_on(fetch(server.port, "Song A", "Artist", 180_000, "Another Album")).unwrap();
    assert!(result.is_none());
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn overflowing_word_offsets_and_lengths_do_not_panic_or_wrap() {
    for text in ["<18446744073709551615,1>a", "<1,18446744073709551615>a"] {
        assert_eq!(any_listen_line(1000, text), (String::new(), false));
    }
}
