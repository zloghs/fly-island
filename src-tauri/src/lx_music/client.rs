//! The two requests the island makes to LX Music's local API, and the JSON they answer with.
//! `/status` reports the playing song and `/lyric-all` its lyrics; neither needs a key, and LX
//! answers only on the machine it runs on unless the user lets it listen on the network.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// The `/status` fields the island reads; `filter` keeps LX from building the others.
const STATUS_FILTER: &str = "status,name,singer,albumName,duration,progress";
/// The API is local, so a missing one is a refusal rather than a slow route.
const TIMEOUT: Duration = Duration::from_secs(2);
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LxStatus {
    /// `playing`, `paused`, `error` or `stoped` (LX spells it so).
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) singer: String,
    #[serde(default)]
    pub(crate) album_name: String,
    /// The song's length and how far it has played, in seconds.
    #[serde(default)]
    pub(crate) duration: f64,
    #[serde(default)]
    pub(crate) progress: f64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LxLyricAll {
    /// The LRC text LX shows for the song.
    #[serde(default)]
    pub(crate) lyric: String,
    /// The same lines with a time for every word, in Any Listen lyrics (`<offset,length>字`), which
    /// is what LX highlights word by word.
    #[serde(default)]
    pub(crate) lxlyric: String,
    /// Its translation in LRC; empty when LX has none.
    #[serde(default)]
    pub(crate) tlyric: String,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        // The API is on this machine: a proxy (the updater's, or the system's) would look for it
        // on the network and fail.
        .no_proxy()
        // A local service must not redirect the request to another host.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("Could not create the LX Music client: {error}"))
}

fn base(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

async fn get_text(url: &str) -> Result<String, String> {
    let mut response = client()?
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not reach the LX Music API at {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("The LX Music API answered {} for {url}", response.status().as_u16()));
    }
    if response.content_length().is_some_and(|length| length > MAX_RESPONSE_BYTES as u64) {
        return Err("The LX Music API response is too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if chunk.len() > MAX_RESPONSE_BYTES.saturating_sub(bytes.len()) {
            return Err("The LX Music API response is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|error| format!("The LX Music API sent invalid UTF-8: {error}"))
}

fn from_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|error| format!("The LX Music API sent something unexpected: {error}"))
}

pub(crate) async fn status(port: u16) -> Result<LxStatus, String> {
    from_json(&get_text(&format!("{}/status?filter={STATUS_FILTER}", base(port))).await?)
}

pub(crate) async fn lyrics(port: u16) -> Result<LxLyricAll, String> {
    from_json(&get_text(&format!("{}/lyric-all", base(port))).await?)
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod http_tests;

#[cfg(test)]
mod tests {
    use super::{base, from_json, LxLyricAll, LxStatus};

    #[test]
    fn a_status_answer_names_the_song_and_its_length() {
        // As LX answers `?filter=status,name,singer,albumName,duration,progress`.
        let answer = r#"{"status":"playing","name":"篇章","singer":"张韶涵、王赫野","albumName":"篇章","duration":248.982542,"progress":197.982095}"#;
        assert_eq!(from_json::<LxStatus>(answer).unwrap(), LxStatus {
            status: "playing".into(), name: "篇章".into(), singer: "张韶涵、王赫野".into(),
            album_name: "篇章".into(), duration: 248.982542, progress: 197.982095,
        });
    }

    #[test]
    fn missing_and_unknown_fields_do_not_break_the_answer() {
        // LX omits fields it has no value for; older and newer builds add their own.
        let idle = from_json::<LxStatus>(r#"{"status":"stoped","picUrl":"http://x","volume":80}"#).unwrap();
        assert_eq!(idle.status, "stoped");
        assert_eq!((idle.name.as_str(), idle.duration), ("", 0.0));
        assert_eq!(from_json::<LxLyricAll>(r#"{"lyric":"[00:01.00]a","rlyric":""}"#).unwrap().lyric, "[00:01.00]a");
        // The word-by-word lyrics ride along in their own field.
        let timed = from_json::<LxLyricAll>(r#"{"lyric":"[00:01.00]ab","lxlyric":"[00:01.00]<0,100>a<100,100>b"}"#).unwrap();
        assert_eq!(timed.lxlyric, "[00:01.00]<0,100>a<100,100>b");
        assert_eq!(from_json::<LxLyricAll>("{}").unwrap(), LxLyricAll::default());
        assert!(from_json::<LxStatus>("not json").is_err());
    }

    #[test]
    fn requests_stay_on_this_machine() {
        assert_eq!(base(23330), "http://127.0.0.1:23330");
    }
}
