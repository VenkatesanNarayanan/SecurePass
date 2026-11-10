<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import VaultUnlock from "../components/VaultUnlock.svelte";
  import Dashboard from "../components/Dashboard.svelte";

  // App state
  let isUnlocked = $state(false);
  let vaultStatus = $state(null);
  let loading = $state(true);

  // Auto-lock settings
  function getAutoLockTimeout(): number {
    const stored = localStorage.getItem("sp_auto_lock_timeout");
    const minutes = stored ? parseInt(stored, 10) : 5;
    return minutes * 60 * 1000;
  }
  let autoLockTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  // Check vault status on mount
  async function checkVaultStatus() {
    try {
      vaultStatus = await invoke("get_vault_status");
    } catch (error) {
      console.error("Failed to check vault status:", error);
    } finally {
      loading = false;
    }
  }

  // Handle successful unlock
  function handleUnlock() {
    isUnlocked = true;
    checkVaultStatus();
    setupAutoLock();
  }

  // Handle lock
  async function handleLock() {
    try {
      await invoke("lock_vault");
      isUnlocked = false;
      vaultStatus = await invoke("get_vault_status");
      cleanupAutoLock();
    } catch (error) {
      console.error("Failed to lock vault:", error);
    }
  }

  // Auto-lock functionality
  function startAutoLockTimer() {
    clearAutoLockTimer();
    autoLockTimer = setTimeout(async () => {
      if (isUnlocked) {
        await handleLock();
      }
    }, getAutoLockTimeout());
  }

  function clearAutoLockTimer() {
    if (autoLockTimer) {
      clearTimeout(autoLockTimer);
      autoLockTimer = null;
    }
  }

  function handleUserActivity() {
    if (isUnlocked) startAutoLockTimer();
  }

  // Initialize
  checkVaultStatus();

  // Activity events reset the auto-lock timer. `scroll` does not bubble, so it
  // is captured to catch scrolling inside the list and detail panes.
  function setupAutoLock() {
    startAutoLockTimer();
    window.addEventListener("focus", handleUserActivity);
    window.addEventListener("mousedown", handleUserActivity);
    window.addEventListener("keydown", handleUserActivity);
    window.addEventListener("scroll", handleUserActivity, true);
    window.addEventListener("touchstart", handleUserActivity);
  }

  function cleanupAutoLock() {
    window.removeEventListener("focus", handleUserActivity);
    window.removeEventListener("mousedown", handleUserActivity);
    window.removeEventListener("keydown", handleUserActivity);
    window.removeEventListener("scroll", handleUserActivity, true);
    window.removeEventListener("touchstart", handleUserActivity);
    clearAutoLockTimer();
  }
</script>

<main class="securepass-app">
  {#if loading}
    <div class="loading" aria-label="Loading SecurePass"><span class="spinner"></span></div>
  {:else if !isUnlocked}
    <VaultUnlock on:unlock={handleUnlock} />
  {:else}
    <Dashboard {vaultStatus} onLock={handleLock} />
  {/if}
</main>

<style>
  .securepass-app {
    width: 100%;
    height: 100vh;
    margin: 0;
    padding: 0;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: var(--text-muted);
  }

  .loading .spinner { width: 28px; height: 28px; }
</style>
