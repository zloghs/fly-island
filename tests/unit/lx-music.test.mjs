import { test } from "node:test";
import assert from "node:assert/strict";

import { isLxMusicPlayer } from "../../src/features/media/lxMusic.ts";

test("LX Music is told apart from the players it shares a name with", () => {
  // What Windows reports for LX Music: the original and the LX-N build.
  assert.equal(isLxMusicPlayer("cn.toside.music.desktop"), true);
  assert.equal(isLxMusicPlayer("cn.toside.music.desktop.netease"), true);
  // A process path, as some players report it.
  assert.equal(isLxMusicPlayer("C:\\Program Files\\LX Music\\lx-music-desktop-netease.exe"), true);
  // NetEase Cloud Music's own AUMID, which is not LX Music.
  assert.equal(isLxMusicPlayer("cloudmusic.exe"), false);
  assert.equal(isLxMusicPlayer("QQMusic.exe"), false);
  assert.equal(isLxMusicPlayer(null), false);
  assert.equal(isLxMusicPlayer(undefined), false);
});
