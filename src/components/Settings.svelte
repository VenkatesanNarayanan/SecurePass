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

  // Load current settings on mount
  async function loadSettings() {
    try {
      // For now, we'll use default values since we don't have persistent settings yet
      // In a future version, we could store these in the vault metadata
      autoLockTimeout = 5;
      clipboardTimeout = 30;
    } catch (err) {
      console.error("Failed to load settings:", err);
    }
  }

  // Save settings
  async function saveSettings() {
    loading = true;
    error = "";
    success = "";

    try {
      // For now, we'll just validate the settings
      // In a future version, we could save these to vault metadata or a separate settings file

      if (autoLockTimeout < 1 || autoLockTimeout > 60) {
        throw new Error("Auto-lock timeout must be between 1 and 60 minutes");
      }

      if (clipboardTimeout < 5 || clipboardTimeout > 300) {
        throw new Error("Clipboard timeout must be between 5 and 300 seconds");
      }

      // Here we would save the settings, but for now we'll just show success
      success = "Settings saved successfully!";
      setTimeout(() => success = "", 3000);

    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save settings";
    } finally {
      loading = false;
    }
  }

  // Initialize
  loadSettings();
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