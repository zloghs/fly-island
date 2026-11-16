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
