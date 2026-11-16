import { useEffect, useState } from "react";
import { getLxMusicStatus, type LxMusicStatus } from "./api";

// Enough to follow LX starting or stopping without asking the API constantly.
const POLL_MS = 5_000;

/**
 * LX Music's local API as it answers now: its status on success, `failed` when it does not answer
 * (the API switched off, or another port). Read while the settings page is open, so the user sees
 * whether the port is right before a song needs lyrics.
 */
export function useLxMusicStatus(port: number) {
  const [status, setStatus] = useState<LxMusicStatus | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let disposed = false;
    const read = () => {
      void getLxMusicStatus(port)
        .then((value) => {
          if (disposed) return;
          setStatus(value);
          setFailed(false);
        })
        .catch(() => {
          if (disposed) return;
          setStatus(null);
          setFailed(true);
        });
    };
    read();
    const timer = window.setInterval(read, POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [port]);

  return { status, failed };
}
