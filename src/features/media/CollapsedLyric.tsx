import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import type { MediaState } from "./api";
import { lineEndMs, songOffsetKey, wordFill, type LyricLine, type LyricWord } from "./lyrics/lrc";
import { lyricClockAt, LYRIC_LEAD_MS } from "./lyrics/lyricClock";
import { useLyricClock } from "./lyrics/useLyricClock";
import { useLyrics, type LyricsSettings } from "./lyrics/useLyrics";
import { useSongOffset } from "./lyrics/useSongOffset";
import "./CollapsedLyric.css";

/** Snapshot the lyric clock on mount; CSS fills every frame without re-rendering the island. */
function CollapsedSungWords({ words, mediaState, offsetMs, playing }: {
  words: LyricWord[]; mediaState: MediaState; offsetMs: number; playing: boolean;
}) {
  const [start] = useState(() => lyricClockAt(null, mediaState, Date.now(), offsetMs).position);
  return words.map((word, index) => {
    const { delayMs, durationMs } = wordFill(word, start);
    const style = { animationDelay: `${delayMs}ms`, animationDuration: `${durationMs}ms`,
      animationPlayState: playing ? "running" : "paused" } as CSSProperties;
    return <span className="collapsed-lyric__word" key={index} style={style}>{word.text}</span>;
  });
}

/**
 * The line being sung, for the collapsed island; the song's title and artist when it has no
 * lyrics or between lines. It wakes on its own for each line, so only this leaf re-renders while a
 * song plays; a new line replaces the last at once (the music page lets it rise in).
 * Keep prefetching while expanded; only the visible leaf drives the line-change clock.
 *
 * The island keeps one width whatever the line says, so a line too wide for it slides across
 * while it is sung, finishing before the next line so its tail can be read.
 * Word highlighting keeps its own sung times. A line that fits stays still.
 */
export function CollapsedLyric({
  mediaState,
  lyricsSettings,
  lines: suppliedLines,
  active,
}: {
  mediaState: MediaState;
  lyricsSettings: LyricsSettings & { lyricsOffsetMs: number };
  lines?: LyricLine[] | null;
  active: boolean;
}) {
  const loaded = useLyrics(mediaState, lyricsSettings, suppliedLines === undefined);
  const lines = suppliedLines === undefined ? loaded.lines : suppliedLines;
  const [songOffsetMs] = useSongOffset(lines ? songOffsetKey(mediaState) : null);
  const offsetMs = lyricsSettings.lyricsOffsetMs + songOffsetMs;
  const { index, position, playing } = useLyricClock(lines, mediaState, offsetMs, active);
  const song = [mediaState.title, mediaState.artist].filter(Boolean).join(" · ");
  const line = lines && index >= 0 ? lines[index] : null;
  const words = lyricsSettings.lyricWordFill === true && line?.text ? line.words : undefined;
  const { width: overflow, boxRef } = useOverflow(active);
  const additional = lyricsSettings.collapsedAdditionalLyrics && line?.text ? line.translation?.trim() : "";
  const { width: additionalOverflow, boxRef: additionalRef } = useOverflow(active);

  // Finish before the early line switch, leaving the last words visible for a moment.
  const span = lines && line ? Math.max(1, lineEndMs(lines, index) - line.timeMs - LYRIC_LEAD_MS - 500) : 0;
  const elapsed = line ? Math.min(Math.max(0, position - line.timeMs), span) : 0;
  const follows = overflow > 0 && span > 0;
  const style = follows
    ? {
        "--collapsed-lyric-shift": `${-overflow}px`,
        animationDuration: `${span}ms`,
        animationDelay: `${-elapsed}ms`,
        animationPlayState: playing && active ? "running" : "paused",
      } as CSSProperties
    : undefined;
  // A new line, a new song or a fresh reading of the position starts the slide (and the word
  // fill inside it) again where it belongs: at the offset a linear animation would have reached.
  const contentKey = `${index}:${mediaState.updatedAt}:${offsetMs}:${active}:${playing}:${follows}`;

  return (
    <span className={`collapsed-lyrics${additional ? " collapsed-lyrics--additional" : ""}`}>
    <span className={`collapsed-lyric${follows ? " collapsed-lyric--overflow" : ""}`} ref={boxRef}>
      <span className={`collapsed-lyric__text${follows ? " collapsed-lyric__text--follow" : ""}`}
        key={contentKey} style={style}>
        {words ? <CollapsedSungWords
          words={words} mediaState={mediaState} offsetMs={offsetMs} playing={playing && active}
        /> : line?.text || song}
      </span>
    </span>
    {additional && <span className="collapsed-lyric collapsed-lyric--additional" ref={additionalRef}>
      <span className={`collapsed-lyric__text${additionalOverflow > 0 && span > 0 ? " collapsed-lyric__text--follow" : ""}`}
        key={`${contentKey}:${additional}`} style={additionalOverflow > 0 && span > 0 ? {
          "--collapsed-lyric-shift": `${-additionalOverflow}px`, animationDuration: `${span}ms`,
          animationDelay: `${-elapsed}ms`, animationPlayState: playing && active ? "running" : "paused",
        } as CSSProperties : undefined}>{additional}</span>
    </span>}
    </span>
  );
}

/**
 * How much wider the line is than the space it has, read after every render (a new line, a font
 * scale, lyrics arriving late) and whenever the island itself changes width; 0 while expanded,
 * where nothing is on screen to measure.
 */
function useOverflow(active: boolean) {
  const boxRef = useRef<HTMLSpanElement>(null);
  const [overflow, setOverflow] = useState(0);

  useLayoutEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    let frame: number | null = null;
    const schedule = () => {
      if (frame !== null) return;
      frame = window.requestAnimationFrame(() => {
        frame = null;
        const inner = box.firstElementChild;
        // Both widths use local CSS pixels: animated transforms and fractional
        // screen coordinates must not feed a render -> layout -> setState loop.
        const width = active && inner instanceof HTMLElement ? Math.max(0, inner.scrollWidth - box.clientWidth) : 0;
        setOverflow((current) => (current === width ? current : width));
      });
    };
    schedule();
    const observer = new ResizeObserver(schedule);
    observer.observe(box);
    if (box.firstElementChild) observer.observe(box.firstElementChild);
    // Reconnect after a render because a new lyric/anchor may replace the inner
    // node. Measurements always happen on a frame, never recursively in commit.
    return () => {
      observer.disconnect();
      if (frame !== null) window.cancelAnimationFrame(frame);
    };
  });

  return { width: overflow, boxRef };
}
