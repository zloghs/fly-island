//! Lyrics from LX Music's own local API: the LRC text the player shows, with the translation.
//! Asking LX beats searching by name — it is the song that plays, so no matching is needed, and it
//! arrives without a network round trip. LX answers about whatever plays at that moment, so the
//! answer is checked against the song the island asked about first.
//!
//! LX also hands over the word-by-word lyrics it highlights itself (`lxlyric`, Any Listen lyrics:
//! `<offset,length>字` per character); they become enhanced LRC word times, which is how the island
//! fills each character in turn.

use super::found::{from_lines, lrc_lines, word_tag, FoundLyrics};
use super::matching::{close_duration, fold, same_artist};
use crate::lx_music;

/// LX's lyrics for the song it plays, when that is the song the island asked about.
/// `Ok(None)` when LX plays something else (a track change mid-lookup) or has no lyrics for it.
pub(super) async fn fetch(port: u16, title: &str, artist: &str, duration_ms: i64) -> Result<Option<FoundLyrics>, String> {
    let status = lx_music::status(port).await?;
    if !matches_track(&status, title, artist, duration_ms) {
        return Ok(None);
    }
    let all = lx_music::lyrics(port).await?;
    // These endpoints describe the current song independently. Recheck after the
    // lyric request so a mid-request track change cannot poison the old song's cache.
    let after = lx_music::status(port).await?;
    if status.name != after.name || status.singer != after.singer || status.album_name != after.album_name
        || !matches_track(&after, title, artist, duration_ms) {
        return Ok(None);
    }
    let duration_ms = (status.duration * 1000.0).round() as i64;
    Ok(from_lx(&all, title).map(|found| found.with_duration(duration_ms)))
}

fn matches_track(status: &lx_music::LxStatus, title: &str, artist: &str, duration_ms: i64) -> bool {
    // Search matching strips Live/remaster suffixes; a local player must retain them.
    !title.trim().is_empty() && fold(title) == fold(&status.name) && same_artist(artist, &status.singer)
        && close_duration(duration_ms, (status.duration * 1000.0).round() as i64)
}

/// LX's lyrics and translation as the island shows them: the sung lines with their times, credits
/// and empty metadata tags dropped, the translation of each line attached by time. The word times
/// of `lxlyric` are used when it has them, since they are the highlight LX itself shows.
pub(super) fn from_lx(all: &lx_music::LxLyricAll, title: &str) -> Option<FoundLyrics> {
    let (word_lines, timed) = any_listen_lines(&all.lxlyric);
    let lines = if timed { word_lines } else { lrc_lines(&all.lyric) };
    let translated = lrc_lines(&all.tlyric);
    let translations: Vec<String> = lines
        .iter()
        .map(|(ms, _)| translated.iter().find(|(at, _)| at.abs_diff(*ms) < 50).map(|(_, text)| text.clone()).unwrap_or_default())
        .collect();
    let has_translation = translations.iter().any(|line| !line.is_empty());
    from_lines(&lines, has_translation.then_some(translations.as_slice()), title)
}

/// Any Listen lyrics as `(line ms, enhanced LRC text)` lines, and whether any line carried word
/// times: `[00:30.958]<0,136>别<136,136>让…` becomes `<00:30.95>别<00:31.09>让…`, every word
/// timed from the line's start plus its own offset, with the line's end closing the last word.
/// Lines written as plain LRC come back unchanged, so the caller can fall back when no line is
/// timed at all.
fn any_listen_lines(text: &str) -> (Vec<(u64, String)>, bool) {
    let mut timed = false;
    let lines = lrc_lines(text)
        .into_iter()
        .map(|(ms, line)| {
            let (enhanced, has_words) = any_listen_line(ms, &line);
            timed |= has_words;
            (ms, enhanced)
        })
        .collect();
    (lines, timed)
}

/// One line of Any Listen lyrics in enhanced LRC; `false` when it had no word times.
fn any_listen_line(line_ms: u64, text: &str) -> (String, bool) {
    let (mut enhanced, mut rest, mut end, mut has_words) = (String::new(), text, line_ms, false);
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else { break };
        let Some((offset, length)) = any_listen_word(&rest[open + 1..open + close]) else {
            // Not a word time (a stray `<`): keep it and carry on after it.
            enhanced.push_str(&rest[..=open]);
            rest = &rest[open + 1..];
            continue;
        };
        enhanced.push_str(&rest[..open]);
        let Some((start, finish)) = line_ms.checked_add(offset)
            .and_then(|start| start.checked_add(length).map(|finish| (start, finish))) else {
                return (String::new(), false);
            };
        enhanced.push_str(&word_tag(start));
        end = end.max(finish);
        has_words = true;
        rest = &rest[open + close + 1..];
    }
    enhanced.push_str(rest);
    if end > line_ms {
        enhanced.push_str(&word_tag(end));
    }
    (enhanced, has_words)
}

/// A word time of Any Listen lyrics: `<offset,length>` in ms, relative to the line's start. Writers
/// add a third field (Kugou's `<offset,length,0>`); it carries nothing the island uses.
fn any_listen_word(tag: &str) -> Option<(u64, u64)> {
    let mut parts = tag.split(',');
    let offset = parts.next()?.trim().parse().ok()?;
    let length = parts.next()?.trim().parse().ok()?;
    Some((offset, length))
}

#[cfg(test)]
#[path = "lxmusic_race_tests.rs"]
mod race_tests;

#[cfg(test)]
mod tests {
    use super::{any_listen_line, from_lx};
    use crate::lx_music::LxLyricAll;

    // As LX answers `/lyric-all` from Kugou: metadata tags, a heading line and credits around the
    // sung lines, and a translation with its own times.
    const LYRIC: &str = "[kuwo:113]\n[ti:篇章]\n[ar:张韶涵,王赫野]\n[offset:0]\n[00:00.000]篇章 - 张韶涵/王赫野\n[00:01.360]曲Composer：李浩瑞\n[00:05.168]制作人Producer：刘涛\n[00:20.000]别让乌云遮住天空\n[00:25.500]别让风沙模糊了梦\n";
    const TLYRIC: &str = "[00:20.000]Do not let the clouds cover the sky\n[00:25.500]Do not let the sand blur the dream\n";
    /// The same song word by word, as `lxlyric` has it.
    const LXYRIC: &str = "[ti:篇章]\n[00:01.360]<0,136>曲<136,136>Composer：\n[00:20.000]<0,140>别<140,140>让<280,140>乌<420,140>云\n[00:25.500]<0,150>别<150,150>让<300,150>风<450,150>沙\n";

    fn all(lyric: &str, lxlyric: &str, tlyric: &str) -> LxLyricAll {
        LxLyricAll { lyric: lyric.into(), lxlyric: lxlyric.into(), tlyric: tlyric.into() }
    }

    #[test]
    fn sung_lines_keep_their_times_and_credits_go() {
        let found = from_lx(&all(LYRIC, "", ""), "篇章").unwrap();
        assert_eq!(found.lrc, "[00:20.00]别让乌云遮住天空\n[00:25.50]别让风沙模糊了梦\n");
        assert_eq!(found.translation, None);
        assert_eq!(found.duration_ms, None);
    }

    #[test]
    fn the_translation_follows_its_line_by_time() {
        let found = from_lx(&all(LYRIC, "", TLYRIC), "篇章").unwrap();
        assert_eq!(found.translation.as_deref(), Some("[00:20.00]Do not let the clouds cover the sky\n[00:25.50]Do not let the sand blur the dream\n"));
        // A translation that is only partly there still lines up, and a stray line is ignored.
        let partial = "[00:25.500]Do not let the sand blur the dream\n[00:40.000]unrelated\n";
        assert_eq!(from_lx(&all(LYRIC, "", partial), "篇章").unwrap().translation.as_deref(),
            Some("[00:25.50]Do not let the sand blur the dream\n"));
    }

    #[test]
    fn a_song_without_sung_lines_has_no_lyrics() {
        assert_eq!(from_lx(&all("[ti:篇章]\n[00:00.000]篇章 - 张韶涵\n", "", ""), "篇章"), None);
        assert_eq!(from_lx(&all("", "", TLYRIC), "篇章"), None);
    }

    #[test]
    fn word_times_become_enhanced_lrc() {
        // Each character starts at the line's time plus its own offset, and the line ends with the
        // last word's end, so the view can fill them in one after another.
        let (line, words) = any_listen_line(20_000, "<0,140>别<140,140>让<280,140>乌<420,140>云");
        assert_eq!(line, "<00:20.00>别<00:20.14>让<00:20.28>乌<00:20.42>云<00:20.56>");
        assert!(words);
        // Kugou adds a third field, which says nothing the view needs.
        assert_eq!(any_listen_line(1_000, "<0,100>a<100,100>b").0, "<00:01.00>a<00:01.10>b<00:01.20>");
        assert!(any_listen_line(1_000, "<0,100,0>a<100,100,0>b").0.starts_with("<00:01.00>a"));
        // A plain line stays plain and reports no words; stray brackets survive.
        assert_eq!(any_listen_line(1_000, "别让乌云"), ("别让乌云".to_string(), false));
        assert_eq!(any_listen_line(1_000, "a<b>c"), ("a<b>c".to_string(), false));
    }

    #[test]
    fn the_players_own_word_times_are_preferred() {
        let timed = from_lx(&all(LYRIC, LXYRIC, TLYRIC), "篇章").unwrap();
        assert_eq!(timed.lrc, "[00:20.00]<00:20.00>别<00:20.14>让<00:20.28>乌<00:20.42>云<00:20.56>\n[00:25.50]<00:25.50>别<00:25.65>让<00:25.80>风<00:25.95>沙<00:26.10>\n");
        // Credits are dropped from the word-by-word text too, and the translation still lines up.
        assert!(!timed.lrc.contains("Composer"));
        assert_eq!(timed.translation.as_deref().map(|lines| lines.lines().count()), Some(2));
        // Without word times anywhere, the plain LRC is used as before.
        let plain = from_lx(&all(LYRIC, "[00:20.000]别让乌云遮住天空\n", ""), "篇章").unwrap();
        assert_eq!(plain.lrc, "[00:20.00]别让乌云遮住天空\n[00:25.50]别让风沙模糊了梦\n");
    }

    /// The whole path against a running player: `cargo test -- --ignored --nocapture` while LX
    /// Music plays a song, with its "开放 API 服务" on.
    #[test]
    #[ignore = "needs LX Music running with its open API service on"]
    fn the_running_player_hands_over_the_song_it_plays() {
        let port = crate::lx_music::DEFAULT_PORT;
        let status = tauri::async_runtime::block_on(crate::lx_music::status(port)).expect("LX Music answers on its default port");
        println!("playing: {} - {} ({:.1}s)", status.name, status.singer, status.duration);
        assert!(!status.name.is_empty(), "a song plays");
        let found = tauri::async_runtime::block_on(super::fetch(port, &status.name, &status.singer, 0))
            .expect("the API is reachable")
            .expect("the playing song has lyrics");
        println!("{} lines, translation: {}", found.lrc.lines().count(), found.translation.is_some());
        // LX has word times for many songs but not all; either way the lines must be there.
        let worded = found.lrc.lines().filter(|line| line.contains('<')).count();
        println!("{worded} lines with word times");
        for line in found.lrc.lines().take(2) {
            println!("  {line}");
        }
        assert!(found.lrc.lines().count() > 3);
    }
}
