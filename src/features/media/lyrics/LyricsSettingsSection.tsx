import { SettingsGroup } from "../../../components/settings/SettingsGroup";
import { SettingsNumberRow } from "../../../components/settings/SettingsNumberRow";
import { SettingsToggleRow } from "../../../components/settings/SettingsToggleRow";
import type { IslandSettings } from "../../appearance/types";
import { useTranslation } from "../../../i18n";
import { useLxMusicStatus } from "../useLxMusicStatus";
import "./LyricsSettingsSection.css";

/**
 * Settings → Music → Lyrics: looking lyrics up online, the Chinese sources with them, where
 * LX Music serves its own, and the Apple Music group, which lives in its own file with the
 * source it switches on. Timing is fixed per song on the music page itself
 * (LyricsOffsetControl).
 */
export function LyricsSettingsSection({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const text = useTranslation().settings.lyrics;
  const lx = useLxMusicStatus(settings.lyricsLxPort);
  // Before the first answer there is nothing to say, which is not the same as not connected.
  const lxLine = lx.failed
    ? text.lxOff
    : lx.status
      ? lx.status.name.trim()
        ? text.lxConnected(lx.status.name)
        : text.lxIdle
      : null;

  return (
    <>
      <SettingsGroup title={text.title} footer={text.hint}>
        <SettingsToggleRow
          title={text.online}
          description={text.onlineHint}
          checked={settings.lyricsOnline}
          onChange={(lyricsOnline) => onSettingsChange({ ...settings, lyricsOnline })}
        />
        {settings.lyricsOnline && (
          <SettingsToggleRow
            indent
            title={text.chinese}
            description={text.chineseHint}
            checked={settings.lyricsChinese}
            onChange={(lyricsChinese) => onSettingsChange({ ...settings, lyricsChinese })}
          />
        )}
        <SettingsToggleRow
          title={text.wordFill}
          checked={settings.lyricWordFill}
          onChange={(lyricWordFill) => onSettingsChange({ ...settings, lyricWordFill })}
        />
        <SettingsToggleRow
          title={text.collapsedAdditional}
          description={text.collapsedAdditionalHint}
          checked={settings.collapsedAdditionalLyrics}
          onChange={(collapsedAdditionalLyrics) => onSettingsChange({ ...settings, collapsedAdditionalLyrics })}
        />
        <SettingsNumberRow
          title={text.lx}
          description={
            <>
              {lxLine && <span className="lyrics-settings__line">{lxLine}</span>}
              {text.lxHint}
            </>
          }
          value={settings.lyricsLxPort}
          min={1}
          max={65535}
          onChange={(lyricsLxPort) => onSettingsChange({ ...settings, lyricsLxPort })}
        />
      </SettingsGroup>
    </>
  );
}
