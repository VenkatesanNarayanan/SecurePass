// Theme preference: "system" follows the OS (prefers-color-scheme); "light" and
// "dark" force a theme. Stored in localStorage and applied as
// <html data-theme="..."> — see src/app.css. src/app.html applies the stored
// value before first paint so the window never flashes the wrong theme.

import { invoke } from "@tauri-apps/api/core";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "sp_theme";

export function getThemePreference(): ThemePreference {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    /* storage unavailable — fall back to system */
  }
  return "system";
}

const DARK_QUERY = "(prefers-color-scheme: dark)";
let currentPref: ThemePreference = "system";
let listening = false;

// Android draws the app behind its status and navigation bars, so their icon
// colour has to follow the effective theme (a no-op on desktop).
function syncSystemBars(): void {
  const dark = currentPref === "dark" || (currentPref === "system" && window.matchMedia(DARK_QUERY).matches);
  invoke("set_system_bars_theme", { dark }).catch(() => {
    /* not running in Tauri (plain `npm run dev`) */
  });
}

export function applyTheme(pref: ThemePreference): void {
  const root = document.documentElement;
  if (pref === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", pref);
  }
  currentPref = pref;
  if (!listening) {
    listening = true;
    window.matchMedia(DARK_QUERY).addEventListener("change", () => {
      if (currentPref === "system") syncSystemBars();
    });
  }
  syncSystemBars();
}

export function setThemePreference(pref: ThemePreference): void {
  try {
    if (pref === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, pref);
  } catch {
    /* storage unavailable — still apply for this session */
  }
  applyTheme(pref);
}
