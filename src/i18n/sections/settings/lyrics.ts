export type SettingsLyricsText = {
  title: string;
  hint: string;
  online: string;
  onlineHint: string;
  chinese: string;
  chineseHint: string;
  wordFill: string;
  lx: string;
  lxHint: string;
  /** The LX row's second line while its API answers: the song it plays, if any. */
  lxConnected: (song: string) => string;
  lxIdle: string;
  /** The LX row's second line when nothing answers on the port. */
  lxOff: string;
};

export const zhCN: SettingsLyricsText = {
  wordFill: "逐字歌词填充",
  title: "歌词",
  hint: "找到带时间轴的歌词时，音乐页会跟着播放滚动显示。某首歌的歌词对不上时，可以在音乐页上单独调它的快慢。",
  online: "在线查找歌词",
  onlineHint: "查 LRCLIB（lrclib.net，免费开源的歌词库），会把歌名、歌手、专辑和时长发过去。每首歌只查一次，结果存在本机。中文歌收录较少。",
  chinese: "也查 QQ 音乐、酷狗和网易云",
  chineseHint: "优先匹配当前播放器，支持逐字歌词与翻译。在线接口为非官方接口，会发送歌名、歌手和时长；酷狗与汽水的本机歌词缓存仅只读访问。",
  lx: "LX Music 端口",
  lxHint: "岛直接从 LX Music 取它正在播放的歌词和翻译，不按歌名搜索，也不联网。先在 LX Music 的「设置 → 开放 API」里打开「启用开放 API 服务」；那边改过端口时，这里填一样的。",
  lxConnected: (song) => `已连接 · 正在播放「${song}」`,
  lxIdle: "已连接 · 没有在播放",
  lxOff: "没连上：在 LX Music 的「设置 → 开放 API」里启用「开放 API 服务」，端口默认 23330",
};

export const en: SettingsLyricsText = {
  wordFill: "Word-by-word lyric fill",
  title: "Lyrics",
  hint: "When synced lyrics are found, the music page scrolls them along with the song. A song whose lyrics run out of step can be moved on its own on the music page.",
  online: "Find lyrics online",
  onlineHint: "Asks LRCLIB (lrclib.net, a free and open lyrics library), sending the title, artist, album and length. Each song is asked about once and the answer is kept on this computer.",
  chinese: "Also ask QQ Music, Kugou and NetEase",
  chineseHint: "Prefers the current player, with word timing and translations. Unofficial online interfaces receive the title, artist and length. Native Kugou and Soda lyric caches are accessed read-only.",
  lx: "LX Music port",
  lxHint: "The island asks LX Music for the lyrics and translation of the song it plays — no search by name, nothing sent online. Turn on \"Enable open API service\" in LX Music's Settings → Open API first; if the port was changed there, enter the same one here.",
  lxConnected: (song) => `Connected · playing "${song}"`,
  lxIdle: "Connected · nothing playing",
  lxOff: "Not connected: turn on \"Enable open API service\" in LX Music's Settings → Open API; the default port is 23330",
};
