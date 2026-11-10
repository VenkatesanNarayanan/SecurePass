import { invoke } from "@tauri-apps/api/core";

// Copies via the Rust `copy_to_clipboard` command and schedules
// `clear_clipboard` after the user's configured timeout (Settings →
// Clipboard auto-clear, localStorage "sp_clipboard_timeout", default 30 s).
// A new copy restarts the timer, so the last copied secret always gets the full window.

let clearTimer: ReturnType<typeof setTimeout> | null = null;

export function clipboardTimeoutSeconds(): number {
  const n = parseInt(localStorage.getItem("sp_clipboard_timeout") || "30", 10);
  return Number.isFinite(n) && n > 0 ? n : 30;
}

/** Copies `text` and returns the number of seconds until it is cleared. */
export async function copySecret(text: string): Promise<number> {
  await invoke("copy_to_clipboard", { text });
  const seconds = clipboardTimeoutSeconds();
  if (clearTimer) clearTimeout(clearTimer);
  clearTimer = setTimeout(async () => {
    clearTimer = null;
    try {
      await invoke("clear_clipboard");
    } catch (error) {
      console.error("Failed to clear clipboard:", error);
    }
  }, seconds * 1000);
  return seconds;
}
