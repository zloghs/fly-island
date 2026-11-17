import { test } from "node:test";
import assert from "node:assert/strict";

import { isLxMusicPlayer, withLxMusicTimeline } from "../../src/features/media/lxMusic.ts";

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

test("LX API progress follows seeks and pauses, while mismatched or invalid samples are ignored", () => {
  const state = { title: "Song (Live)", artist: "A、B", sourceApp: "cn.toside.music.desktop", positionMs: 1000, durationMs: 0, hasTimeline: false };
  const status = { name: "Song (Live)", singer: "B / A", status: "playing", progress: 65.5, duration: 180 };
  const reading = withLxMusicTimeline(state, status, 5000);
  assert.equal(reading.positionMs, 65500);
  assert.equal(reading.durationMs, 180000);
  assert.equal(reading.updatedAt, 5000);
  assert.equal(reading.hasTimeline, true);
  assert.equal(withLxMusicTimeline(reading, { ...status, progress: 10, status: "paused" }, 6000).positionMs, 10000);
  assert.equal(withLxMusicTimeline(reading, { ...status, status: "paused" }, 6000).playbackStatus, "paused");
  for (const change of [{ name: "Song" }, { singer: "Other" }, { progress: NaN }, { progress: -1 }, { progress: 181 }, { duration: 0 }, { status: "stoped" }]) {
    assert.equal(withLxMusicTimeline(state, { ...status, ...change }, 5000), state);
  }
});
