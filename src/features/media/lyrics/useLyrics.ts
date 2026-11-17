import { useEffect, useState } from "react";
import type { MediaState } from "../api";
import { isLxMusicPlayer } from "../lxMusic";
import { getLyrics } from "./api";
import { parseLrc, trackKey, withTranslations, type LyricLine } from "./lrc";

export type LyricsSettings = {
  lyricsFolder: string; lyricsOnline: boolean; lyricsChinese?: boolean; lyricWordFill?: boolean; lyricsLxPort?: number; collapsedAdditionalLyrics?: boolean;
};

type Found = { lookup: string; lines: LyricLine[]; durationMs: number; expiresAt: number };
// The last few lookups' answers. The music page mounts again on every visit; without these its
// lyrics arrived a few frames in and re-laid the page out while the island was still resizing.
const recent = new Map<string, Found>();
const pending = new Map<string, Promise<Found>>();
const RECENT_LIMIT = 8;
const MISS_TTL_MS = 60_000;
const LX_REFRESH_MS = 5_000;

function remember(lookup: string, found: Found) {
  recent.delete(lookup);
  recent.set(lookup, found);
  if (recent.size > RECENT_LIMIT) recent.delete(recent.keys().next().value ?? lookup);
}

/**
 * Synced lyrics for the playing track, looked up once per track (the native side caches online
 * answers on disk). `lines` is null while looking, when nothing is found, or with neither a
 * folder nor online lyrics set; `notFound` only when a finished lookup found nothing.
 * `durationMs` is the song's length as the lyrics source lists it (0 when it does not).
 */
export function useLyrics(
  mediaState: MediaState,
  settings: LyricsSettings,
): { lines: LyricLine[] | null; notFound: boolean; durationMs: number } {
  const key = trackKey(mediaState);
  const folder = settings.lyricsFolder.trim();
  // Read when the track changes, not on every poll that moves the position.
  const { title, artist, album, durationMs, trackId, sourceApp: sourceAppId } = mediaState;
  // LX Music serves the lyrics of the song it plays, so it is worth asking even with online
  // lookup off: the answer never leaves this computer.
  const lxPlayer = isLxMusicPlayer(sourceAppId);
  const lxPort = settings.lyricsLxPort ?? 0;
  const active = Boolean(key) && (folder !== "" || settings.lyricsOnline || settings.lyricsChinese === true || lxPlayer);
  const [found, setFound] = useState<Found | null>(null);
  const [refresh, setRefresh] = useState(0);
  const chinese = settings.lyricsChinese === true;
  const wordFill = settings.lyricWordFill === true;
  const lxRecording = lxPlayer ? `${album ?? ""}\u001f${durationMs}` : "";
  const lookup = [key, folder, settings.lyricsOnline, chinese, trackId, sourceAppId, wordFill, lxPlayer ? lxPort : 0, lxRecording].join("\u001f");

  useEffect(() => {
    if (!active || !lxPlayer) return;
    // LX may still be loading lyrics after publishing new metadata. Retry without
    // waiting for another track; do not permanently retain an early answer.
    const timer = window.setInterval(() => setRefresh((value) => value + 1), LX_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [active, lxPlayer]);

  useEffect(() => {
    if (!active || !key || !title) return;
    const cached = recent.get(lookup);
    if (cached && cached.expiresAt > Date.now()) return;
    let cancelled = false;
    let request = pending.get(lookup);
    if (!request) {
      request = getLyrics({ title, artist: artist ?? "", album: album ?? "", durationMs, folder, online: settings.lyricsOnline, chinese, trackId: trackId ?? null, sourceAppId, wordFill, lxPort: lxPlayer ? lxPort : 0 })
      .then((lyrics) => {
        const parsed = lyrics ? parseLrc(lyrics.lrc).filter((line, index, all) => line.text || index < all.length - 1) : [];
        const refreshed = withTranslations(parsed, lyrics?.translation ? parseLrc(lyrics.translation) : []);
        const previous = recent.get(lookup);
        // Revalidating unchanged LX lyrics must not reset the line clock or animations.
        const lines = previous && JSON.stringify(previous.lines) === JSON.stringify(refreshed) ? previous.lines : refreshed;
        return { lookup, lines, durationMs: lyrics?.durationMs ?? 0,
          expiresAt: lxPlayer ? Date.now() + LX_REFRESH_MS : lines.length ? Infinity : Date.now() + MISS_TTL_MS };
      })
      .catch((error: unknown) => {
        console.error("Failed to look up lyrics", error);
        return { lookup, lines: [], durationMs: 0, expiresAt: Date.now() + (lxPlayer ? LX_REFRESH_MS : MISS_TTL_MS) };
      })
      .then((answer) => {
        remember(lookup, answer);
        pending.delete(lookup);
        return answer;
      });
      pending.set(lookup, request);
    }
    void request.then((answer) => { if (!cancelled) setFound(answer); });
    return () => {
      cancelled = true;
    };
    // Non-LX duration/album only refine a request; LX uses them to identify the recording.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, key, folder, settings.lyricsOnline, chinese, trackId, sourceAppId, wordFill, lxPlayer, lxPort, lxRecording, refresh]);

  const current = recent.get(lookup) ?? (found?.lookup === lookup ? found : null);
  if (!active || !current) return { lines: null, notFound: false, durationMs: 0 };
  return current.lines.length === 0
    ? { lines: null, notFound: true, durationMs: 0 }
    : { lines: current.lines, notFound: false, durationMs: current.durationMs };
}
