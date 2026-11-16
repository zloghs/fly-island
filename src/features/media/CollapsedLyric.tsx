import { useCallback, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import type { MediaState } from "./api";
import { lineEndMs, songOffsetKey, wordFill, type LyricWord } from "./lyrics/lrc";
import { lyricClockAt } from "./lyrics/lyricClock";
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
 * while it is sung: the movement lasts exactly as long as the line, which keeps the words lighting
 * up at their own times in step with it. A line that fits stays still.
 */
export function CollapsedLyric({
  mediaState,
  lyricsSettings,
  active,
}: {
  mediaState: MediaState;
  lyricsSettings: LyricsSettings & { lyricsOffsetMs: number };
  active: boolean;
}) {
  const { lines } = useLyrics(mediaState, lyricsSettings);
  const [songOffsetMs] = useSongOffset(lines ? songOffsetKey(mediaState) : null);
  const offsetMs = lyricsSettings.lyricsOffsetMs + songOffsetMs;
  const { index, position, playing } = useLyricClock(lines, mediaState, offsetMs, active);
  const song = [mediaState.title, mediaState.artist].filter(Boolean).join(" · ");
  const line = lines && index >= 0 ? lines[index] : null;
  const words = lyricsSettings.lyricWordFill === true && line?.text ? line.words : undefined;
  const { width: overflow, boxRef } = useOverflow(active);

  const span = lines && line ? Math.max(1_200, lineEndMs(lines, index) - line.timeMs) : 0;
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
    <span className={`collapsed-lyric${follows ? " collapsed-lyric--overflow" : ""}`} ref={boxRef}>
      <span className={`collapsed-lyric__text${follows ? " collapsed-lyric__text--follow" : ""}`}
        key={contentKey} style={style}>
        {words ? <CollapsedSungWords
          words={words} mediaState={mediaState} offsetMs={offsetMs} playing={playing && active}
        /> : line?.text || song}
      </span>
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

  const measure = useCallback(() => {
    const box = boxRef.current;
    if (!box) return;
    const inner = box.firstElementChild;
    const width = active && inner instanceof HTMLElement ? Math.max(0, inner.scrollWidth - box.clientWidth) : 0;
    setOverflow((current) => (current === width ? current : width));
  }, [active]);

  useLayoutEffect(measure);

  useLayoutEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    return () => observer.disconnect();
  }, [measure]);

  return { width: overflow, boxRef };
}
