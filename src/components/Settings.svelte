<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

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
        cloudError = `${result.conflicts.length} conflict(s) need your attention. Open the sync panel to resolve them.`;
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

<div
  class="modal-overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="presentation"
>
  <div
    class="modal-content"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2>⚙️ Settings</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <div class="settings-content">
      <!-- Security Settings -->
      <div class="settings-section">
        <h3>🔒 Security Settings</h3>

        <div class="setting-group">
          <label for="autoLockTimeout">
            Auto-lock Timeout
            <span class="setting-value">{autoLockTimeout} minutes</span>
          </label>
          <input
            id="autoLockTimeout"
            type="range"
            min="1"
            max="60"
            bind:value={autoLockTimeout}
            step="1"
          />
          <div class="range-hints">
            <span>1 min</span>
            <span>60 min</span>
          </div>
          <p class="setting-description">
            Automatically lock the vault after this period of inactivity.
          </p>
        </div>

        <div class="setting-group">
          <label for="clipboardTimeout">
            Clipboard Auto-Clear
            <span class="setting-value">{clipboardTimeout} seconds</span>
          </label>
          <input
            id="clipboardTimeout"
            type="range"
            min="5"
            max="300"
            bind:value={clipboardTimeout}
            step="5"
          />
          <div class="range-hints">
            <span>5 sec</span>
            <span>5 min</span>
          </div>
          <p class="setting-description">
            Automatically clear copied passwords from clipboard after this time.
          </p>
        </div>
      </div>

      <!-- Change Master Password -->
      <div class="settings-section">
        <h3>🔑 Change Master Password</h3>
        <div class="setting-group">
          <div class="pw-field">
            <label for="current-pw">Current Password</label>
            <input
              id="current-pw"
              type="password"
              bind:value={currentPassword}
              placeholder="Enter current password"
              disabled={pwLoading}
            />
          </div>
          <div class="pw-field">
            <label for="new-pw">New Password</label>
            <input
              id="new-pw"
              type="password"
              bind:value={newPassword}
              placeholder="At least 8 characters"
              disabled={pwLoading}
            />
          </div>
          <div class="pw-field">
            <label for="confirm-pw">Confirm New Password</label>
            <input
              id="confirm-pw"
              type="password"
              bind:value={confirmPassword}
              placeholder="Repeat new password"
              disabled={pwLoading}
            />
          </div>

          {#if pwError}
            <div class="error-message" style="margin-top:10px">{pwError}</div>
          {/if}
          {#if pwSuccess}
            <div class="success-message" style="margin-top:10px">{pwSuccess}</div>
          {/if}

          <button class="btn-change-pw" onclick={changePassword} disabled={pwLoading} style="margin-top:14px">
            {#if pwLoading}
              <span class="spinner"></span> Changing…
            {:else}
              Change Password
            {/if}
          </button>

          <p class="setting-description" style="margin-top:10px">
            A new encryption key will be derived from the new password. If Google Drive sync is active, the vault will be re-uploaded automatically.
          </p>
        </div>
      </div>

      <!-- Cloud Storage -->
      <div class="settings-section">
        <h3>☁️ Cloud Storage (Google Drive)</h3>

        <div class="setting-group">
          {#if syncStatus?.connected}
            <div class="cloud-status connected">
              <span class="status-dot"></span>
              <span>Connected to Google Drive</span>
            </div>
            <p class="setting-description">
              Last synced: {formatLastSync(syncStatus.lastSync)}
            </p>
            <div class="cloud-actions">
              <button
                class="btn-cloud-sync"
                onclick={syncNow}
                disabled={syncing}
              >
                {#if syncing}
                  <span class="spinner"></span> Syncing…
                {:else}
                  Sync Now
                {/if}
              </button>
              <button class="btn-cloud-disconnect" onclick={disconnectGoogleDrive}>
                Disconnect
              </button>
            </div>
          {:else}
            <div class="cloud-status disconnected">
              <span class="status-dot"></span>
              <span>Not connected</span>
            </div>
            <p class="setting-description">
              Connect your Google Drive to sync your encrypted vault across all your devices.
              Only files created by SecurePass are accessible to the app.
            </p>
            <button
              class="btn-cloud-connect"
              onclick={connectGoogleDrive}
              disabled={cloudLoading}
            >
              {#if cloudLoading}
                <span class="spinner"></span> Waiting for Google sign-in…
              {:else}
                Connect Google Drive
              {/if}
            </button>
          {/if}

          {#if cloudError}
            <div class="error-message" style="margin-top:12px">{cloudError}</div>
          {/if}
          {#if cloudSuccess}
            <div class="success-message" style="margin-top:12px">{cloudSuccess}</div>
          {/if}
        </div>
      </div>

      <!-- Status Messages -->
      {#if error}
        <div class="error-message">{error}</div>
      {/if}

      {#if success}
        <div class="success-message">{success}</div>
      {/if}
    </div>

    <!-- Modal Actions -->
    <div class="modal-footer">
      <button type="button" class="btn-secondary" onclick={onClose}>
        Cancel
      </button>
      <button type="button" class="btn-primary" onclick={saveSettings} disabled={loading}>
        {#if loading}
          <span class="spinner"></span>
          Saving...
        {:else}
          💾 Save Settings
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    max-width: 500px;
    width: 90%;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h2 {
    margin: 0;
    color: #2d3748;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #718096;
    padding: 4px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background-color: #f7fafc;
    color: #2d3748;
  }

  .settings-content {
    padding: 20px;
  }

  .settings-section {
    margin-bottom: 24px;
  }

  .settings-section h3 {
    margin: 0 0 16px 0;
    color: #2d3748;
    font-size: 1rem;
    font-weight: 600;
  }

  .setting-group {
    margin-bottom: 20px;
    padding: 16px;
    background: #f8f9fa;
    border-radius: 8px;
    border: 1px solid #e2e8f0;
  }

  .setting-group label {
    display: block;
    margin-bottom: 8px;
    color: #4a5568;
    font-weight: 500;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .setting-value {
    color: #667eea;
    font-weight: 600;
    font-size: 14px;
  }

  .setting-group input[type="range"] {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: #e2e8f0;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
    margin: 8px 0;
  }

  .setting-group input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #667eea;
    cursor: pointer;
  }

  .setting-group input[type="range"]::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #667eea;
    cursor: pointer;
    border: none;
  }

  .range-hints {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #718096;
    margin-top: 4px;
  }

  .setting-description {
    margin: 8px 0 0 0;
    font-size: 14px;
    color: #718096;
    line-height: 1.4;
  }

  .error-message {
    background: #fed7d7;
    color: #c53030;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 16px;
    border: 1px solid #feb2b2;
  }

  .success-message {
    background: #c6f6d5;
    color: #22543d;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 16px;
    border: 1px solid #9ae6b4;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 20px;
    border-top: 1px solid #e2e8f0;
  }

  .btn-secondary {
    padding: 10px 16px;
    background-color: #e2e8f0;
    color: #4a5568;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .btn-secondary:hover {
    background-color: #cbd5e0;
  }

  .btn-primary {
    padding: 10px 16px;
    background-color: #667eea;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: #5a67d8;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid transparent;
    border-top: 2px solid currentColor;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .cloud-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    font-size: 0.875rem;
    margin-bottom: 8px;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .connected .status-dot { background: #38a169; }
  .disconnected .status-dot { background: #a0aec0; }

  .cloud-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  .btn-cloud-connect {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 10px 16px;
    background: #4285f4;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-cloud-connect:hover:not(:disabled) { background: #3367d6; }
  .btn-cloud-connect:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-cloud-sync {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: #667eea;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-cloud-sync:hover:not(:disabled) { background: #5a67d8; }
  .btn-cloud-sync:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-cloud-disconnect {
    padding: 8px 14px;
    background: #edf2f7;
    color: #718096;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-cloud-disconnect:hover { background: #fed7d7; color: #c53030; border-color: #feb2b2; }

  .pw-field {
    margin-bottom: 14px;
  }

  .pw-field label {
    display: block;
    margin-bottom: 6px;
    color: #4a5568;
    font-weight: 500;
    font-size: 0.875rem;
  }

  .pw-field input[type="password"] {
    width: 100%;
    padding: 10px 12px;
    border: 1.5px solid #e2e8f0;
    border-radius: 6px;
    font-size: 14px;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }

  .pw-field input[type="password"]:focus {
    outline: none;
    border-color: #667eea;
  }

  .pw-field input[type="password"]:disabled {
    background: #f0f0f0;
    cursor: not-allowed;
  }

  .btn-change-pw {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: #667eea;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-change-pw:hover:not(:disabled) { background: #5a67d8; }
  .btn-change-pw:disabled { opacity: 0.6; cursor: not-allowed; }

  @media (max-width: 768px) {
    .modal-content {
      margin: 10px;
      max-height: calc(100vh - 20px);
    }

    .modal-header,
    .settings-content {
      padding: 16px;
    }

    .modal-footer {
      padding: 16px;
      flex-direction: column-reverse;
    }

    .btn-primary,
    .btn-secondary {
      width: 100%;
    }
  }
</style>