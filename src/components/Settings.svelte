<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "./Icon.svelte";
  import { getThemePreference, setThemePreference, type ThemePreference } from "$lib/theme";

  // Appearance applies immediately (no Save needed).
  let theme = $state<ThemePreference>(getThemePreference());
  const THEME_OPTIONS: { value: ThemePreference; label: string; icon: string }[] = [
    { value: "system", label: "System", icon: "monitor" },
    { value: "light", label: "Light", icon: "sun" },
    { value: "dark", label: "Dark", icon: "moon" },
  ];

  function chooseTheme(value: ThemePreference) {
    theme = value;
    setThemePreference(value);
  }

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  // Settings state
  let autoLockTimeout = $state(5); // minutes
  let clipboardTimeout = $state(30); // seconds
  let loading = $state(false);
  let error = $state("");
  let success = $state("");

  const STORAGE_KEYS = {
    autoLockTimeout: "sp_auto_lock_timeout",
    clipboardTimeout: "sp_clipboard_timeout",
  };

  // Load current settings on mount
  function loadSettings() {
    const storedAutoLock = localStorage.getItem(STORAGE_KEYS.autoLockTimeout);
    const storedClipboard = localStorage.getItem(STORAGE_KEYS.clipboardTimeout);
    if (storedAutoLock) autoLockTimeout = parseInt(storedAutoLock, 10);
    if (storedClipboard) clipboardTimeout = parseInt(storedClipboard, 10);
  }

  // Save settings
  async function saveSettings() {
    loading = true;
    error = "";
    success = "";

    try {
      if (autoLockTimeout < 1 || autoLockTimeout > 60) {
        throw new Error("Auto-lock timeout must be between 1 and 60 minutes");
      }
      if (clipboardTimeout < 5 || clipboardTimeout > 300) {
        throw new Error("Clipboard timeout must be between 5 and 300 seconds");
      }

      localStorage.setItem(STORAGE_KEYS.autoLockTimeout, String(autoLockTimeout));
      localStorage.setItem(STORAGE_KEYS.clipboardTimeout, String(clipboardTimeout));

      success = "Settings saved successfully!";
      setTimeout(() => success = "", 3000);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save settings";
    } finally {
      loading = false;
    }
  }

  // ── Change master password ────────────────────────────────────────────────

  let currentPassword = $state("");
  let newPassword = $state("");
  let confirmPassword = $state("");
  let pwLoading = $state(false);
  let pwError = $state("");
  let pwSuccess = $state("");

  async function changePassword() {
    pwError = "";
    pwSuccess = "";

    if (!currentPassword) { pwError = "Enter your current password."; return; }
    if (newPassword.length < 8) { pwError = "New password must be at least 8 characters."; return; }
    if (newPassword !== confirmPassword) { pwError = "New passwords do not match."; return; }
    if (newPassword === currentPassword) { pwError = "New password must be different from the current one."; return; }

    pwLoading = true;
    try {
      await invoke("change_master_password", { currentPassword, newPassword });
      pwSuccess = "Master password changed successfully!";
      currentPassword = "";
      newPassword = "";
      confirmPassword = "";
      setTimeout(() => pwSuccess = "", 4000);
    } catch (e) {
      pwError = e instanceof Error ? e.message : String(e);
    } finally {
      pwLoading = false;
    }
  }

  // ── Cloud storage ──────────────────────────────────────────────────────────

  let syncStatus = $state<{ connected: boolean; lastSync: string | null; hasPendingConflicts: boolean } | null>(null);
  let cloudLoading = $state(false);
  let cloudError = $state("");
  let cloudSuccess = $state("");
  let syncing = $state(false);

  async function loadSyncStatus() {
    try {
      syncStatus = await invoke("get_sync_status");
    } catch {
      // Not a fatal error — cloud may not be configured
    }
  }

  async function connectGoogleDrive() {
    cloudLoading = true;
    cloudError = "";
    cloudSuccess = "";
    try {
      await invoke("connect_google_drive");
      cloudSuccess = "Connected to Google Drive!";
      await loadSyncStatus();
      setTimeout(() => (cloudSuccess = ""), 3000);
    } catch (e) {
      cloudError = e instanceof Error ? e.message : String(e);
    } finally {
      cloudLoading = false;
    }
  }

  async function disconnectGoogleDrive() {
    if (!confirm("Disconnect from Google Drive? Your local vault will not be affected.")) return;
    try {
      await invoke("disconnect_google_drive");
      await loadSyncStatus();
    } catch (e) {
      cloudError = e instanceof Error ? e.message : String(e);
    }
  }

  async function syncNow() {
    syncing = true;
    cloudError = "";
    cloudSuccess = "";
    try {
      const result: any = await invoke("sync_vault");
      if (result.status === "synced") {
        const parts = [];
        if (result.entriesAdded) parts.push(`${result.entriesAdded} added`);
        if (result.entriesUpdated) parts.push(`${result.entriesUpdated} updated`);
        cloudSuccess = parts.length
          ? `Synced — ${parts.join(", ")}.`
          : "Already up to date.";
        setTimeout(() => (cloudSuccess = ""), 4000);
        await loadSyncStatus();
      } else if (result.status === "conflicts") {
        cloudError = `${result.conflicts.length} conflict(s) need your attention. Close Settings and choose "Resolve conflicts" in the sidebar.`;
      }
    } catch (e) {
      cloudError = e instanceof Error ? e.message : String(e);
    } finally {
      syncing = false;
    }
  }

  function formatLastSync(iso: string | null) {
    if (!iso) return "Never";
    return new Date(iso).toLocaleString();
  }

  // Initialize
  loadSettings();
  loadSyncStatus();
</script>

<div class="modal-overlay" onclick={onClose} onkeydown={(e) => e.key === "Escape" && onClose()} role="presentation">
  <div
    class="modal wide"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => { e.stopPropagation(); if (e.key === "Escape") onClose(); }}
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2 id="settings-title">Settings</h2>
      <button class="icon-btn" onclick={onClose} aria-label="Close settings"><Icon name="close" size={16} /></button>
    </div>

    <div class="modal-body settings">
      <!-- Appearance -->
      <section class="section">
        <h3>Appearance</h3>
        <div class="segmented" role="radiogroup" aria-label="Theme">
          {#each THEME_OPTIONS as opt (opt.value)}
            <button
              type="button"
              role="radio"
              aria-checked={theme === opt.value}
              class:on={theme === opt.value}
              onclick={() => chooseTheme(opt.value)}
            >
              <Icon name={opt.icon} size={15} /> {opt.label}
            </button>
          {/each}
        </div>
        <p class="hint">System follows your operating system's light or dark setting.</p>
      </section>

      <!-- Security -->
      <section class="section">
        <h3>Security</h3>
        <div class="setting">
          <div class="setting-top">
            <label class="label" for="autoLockTimeout">Auto-lock after inactivity</label>
            <span class="value mono">{autoLockTimeout} min</span>
          </div>
          <input id="autoLockTimeout" type="range" min="1" max="60" step="1" bind:value={autoLockTimeout} />
          <div class="range-hints"><span>1 min</span><span>60 min</span></div>
        </div>
        <div class="setting">
          <div class="setting-top">
            <label class="label" for="clipboardTimeout">Clear copied secrets after</label>
            <span class="value mono">{clipboardTimeout} s</span>
          </div>
          <input id="clipboardTimeout" type="range" min="5" max="300" step="5" bind:value={clipboardTimeout} />
          <div class="range-hints"><span>5 s</span><span>5 min</span></div>
        </div>
        {#if error}<div class="msg msg-error">{error}</div>{/if}
        {#if success}<div class="msg msg-success">{success}</div>{/if}
        <div class="row-end">
          <button type="button" class="btn btn-primary" onclick={saveSettings} disabled={loading}>
            {#if loading}<span class="spinner"></span>{/if} Save security settings
          </button>
        </div>
      </section>

      <!-- Change master password -->
      <section class="section">
        <h3>Change master password</h3>
        <div class="pw-grid">
          <div class="field">
            <label class="label" for="current-pw">Current password</label>
            <input id="current-pw" class="input" type="password" bind:value={currentPassword} disabled={pwLoading} autocomplete="current-password" />
          </div>
          <div class="field">
            <label class="label" for="new-pw">New password</label>
            <input id="new-pw" class="input" type="password" bind:value={newPassword} placeholder="At least 8 characters" disabled={pwLoading} autocomplete="new-password" />
          </div>
          <div class="field">
            <label class="label" for="confirm-pw">Confirm new password</label>
            <input id="confirm-pw" class="input" type="password" bind:value={confirmPassword} disabled={pwLoading} autocomplete="new-password" />
          </div>
        </div>
        {#if pwError}<div class="msg msg-error">{pwError}</div>{/if}
        {#if pwSuccess}<div class="msg msg-success">{pwSuccess}</div>{/if}
        <p class="hint">
          The vault is re-encrypted with a key derived from the new password. If Google Drive sync is on,
          the vault is re-uploaded and your other devices will ask for the new password on their next sync.
        </p>
        <div class="row-end">
          <button class="btn btn-secondary" onclick={changePassword} disabled={pwLoading}>
            {#if pwLoading}<span class="spinner"></span> Changing…{:else}Change password{/if}
          </button>
        </div>
      </section>

      <!-- Cloud storage -->
      <section class="section">
        <h3>Google Drive sync</h3>
        {#if syncStatus?.connected}
          <div class="cloud">
            <span class="dot ok"></span>
            <div class="cloud-text">
              <div class="cloud-title">Connected</div>
              <div class="hint">Last synced: {formatLastSync(syncStatus.lastSync)}</div>
            </div>
            <button class="btn btn-secondary" onclick={syncNow} disabled={syncing}>
              {#if syncing}<span class="spinner"></span> Syncing…{:else}Sync now{/if}
            </button>
            <button class="btn btn-ghost" onclick={disconnectGoogleDrive}>Disconnect</button>
          </div>
        {:else}
          <div class="cloud">
            <span class="dot"></span>
            <div class="cloud-text">
              <div class="cloud-title">Not connected</div>
              <div class="hint">
                Keep an encrypted copy of your vault in your own Google Drive and sync it across devices.
                SecurePass can only see files it created.
              </div>
            </div>
            <button class="btn btn-primary" onclick={connectGoogleDrive} disabled={cloudLoading}>
              {#if cloudLoading}<span class="spinner"></span> Waiting for sign-in…{:else}<Icon name="cloud" size={15} /> Connect{/if}
            </button>
          </div>
        {/if}
        {#if cloudError}<div class="msg msg-error">{cloudError}</div>{/if}
        {#if cloudSuccess}<div class="msg msg-success">{cloudSuccess}</div>{/if}
      </section>
    </div>

    <div class="modal-footer">
      <button type="button" class="btn btn-secondary" onclick={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  .settings { gap: 0; padding-top: 4px; }

  .section {
    display: flex; flex-direction: column; gap: 12px;
    padding: 18px 0; border-bottom: 1px solid var(--border-subtle);
  }
  .section:last-child { border-bottom: 0; }
  h3 { margin: 0; font-size: 14px; font-weight: 700; }

  .hint { margin: 0; font-size: 12px; color: var(--text-muted); }

  .segmented {
    display: inline-flex; align-self: flex-start; gap: 2px; padding: 3px;
    border-radius: 10px; background: var(--chip);
  }
  .segmented button {
    display: inline-flex; align-items: center; gap: 6px;
    min-height: 32px; padding: 0 14px; border: 0; border-radius: 8px;
    background: transparent; color: var(--text-2); font-size: 13px; cursor: pointer;
  }
  .segmented button.on { background: var(--bg); color: var(--text); font-weight: 600; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12); }

  .setting { display: flex; flex-direction: column; gap: 6px; }
  .setting-top { display: flex; justify-content: space-between; align-items: baseline; }
  .value { font-size: 13px; color: var(--text-2); }
  .setting input[type="range"] { width: 100%; }
  .range-hints { display: flex; justify-content: space-between; font-size: 11px; color: var(--text-muted); }

  .pw-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }

  .row-end { display: flex; justify-content: flex-end; }

  .cloud { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .cloud-text { flex: 1; min-width: 200px; }
  .cloud-title { font-weight: 600; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--text-muted); flex: none; }
  .dot.ok { background: var(--success); }

  @media (max-width: 700px) {
    .pw-grid { grid-template-columns: minmax(0, 1fr); }
  }
</style>
