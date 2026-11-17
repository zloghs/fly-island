import type { LxMusicStatus, MediaState } from "./api";

/**
 * Whether a media session's source app is LX Music (洛雪音乐, and the LX-N build this island also
 * supports). Windows reports it as `cn.toside.music.desktop` or `cn.toside.music.desktop.netease`,
 * and both executables carry `lx-music-desktop`; either way it is not NetEase Cloud Music, whose
 * AUMID merely ends in `.netease`.
 */
export function isLxMusicPlayer(sourceApp: string | null | undefined) {
  const id = (sourceApp ?? "").toLowerCase();
  return id.includes("toside.music.desktop") || id.includes("lx-music-desktop");
}

/** Use LX's own seek-aware clock only when its API describes this exact session track. */
export function withLxMusicTimeline(state: MediaState, status: LxMusicStatus, sampledAt: number): MediaState {
  const fold = (text: string) => text.normalize("NFKC").trim().toLowerCase().replace(/\s+/g, " ");
  const artists = (text: string) => fold(text).split(/[,/、;&]/).map((name) => name.trim()).filter(Boolean).sort().join("\u001f");
  if (!isLxMusicPlayer(state.sourceApp) || !state.title || fold(state.title) !== fold(status.name)
    || (state.artist && artists(state.artist) !== artists(status.singer))
    || !Number.isFinite(status.progress) || status.progress < 0
    || !Number.isFinite(status.duration) || status.duration <= 0 || status.progress > status.duration
    || !["playing", "paused"].includes(status.status)) return state;
  return {
    ...state, positionMs: Math.round(status.progress * 1000), durationMs: Math.round(status.duration * 1000),
    updatedAt: sampledAt, hasTimeline: true, playbackStatus: status.status === "playing" ? "playing" : "paused",
  };
}
