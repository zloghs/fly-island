import { act } from "react";

export async function checkCollapsedLyrics(panel: HTMLElement, check: (condition: unknown, message: string) => void) {
  const row = [...panel.querySelectorAll<HTMLElement>(".settings-row")].find((item) => item.textContent?.includes("收起时显示附加歌词"));
  const toggle = row?.querySelector<HTMLInputElement>("[role='switch']");
  check(toggle && !toggle.checked, "Additional collapsed lyrics are off by default");
  await act(async () => { toggle?.click(); });
  check(JSON.parse(localStorage.getItem("fly-island-settings") ?? "{}").collapsedAdditionalLyrics === true,
    "The additional-lyrics switch saves its enabled state");
  await act(async () => { toggle?.click(); });
}
