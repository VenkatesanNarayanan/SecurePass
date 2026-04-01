<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import VaultUnlock from "../components/VaultUnlock.svelte";
  import Dashboard from "../components/Dashboard.svelte";

  // App state
  let isUnlocked = $state(false);
  let vaultStatus = $state(null);
  let loading = $state(true);

  // Auto-lock settings
  const AUTO_LOCK_TIMEOUT = 5 * 60 * 1000; // 5 minutes in milliseconds
  let autoLockTimer = $state<ReturnType<typeof setTimeout> | null>(null);
  let lastActivity = $state(Date.now());

  // Check vault status on mount
  async function checkVaultStatus() {
    try {
      vaultStatus = await invoke("get_vault_status");
      loading = false;
    } catch (error) {
      console.error("Failed to check vault status:", error);
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
        console.log("Auto-locking vault due to inactivity");
        await handleLock();
      }
    }, AUTO_LOCK_TIMEOUT);
  }

  function clearAutoLockTimer() {
    if (autoLockTimer) {
      clearTimeout(autoLockTimer);
      autoLockTimer = null;
    }
  }

  function resetActivityTimer() {
    if (isUnlocked) {
      lastActivity = Date.now();
      startAutoLockTimer();
    }
  }

  function handleWindowFocus() {
    // Reset timer when window gains focus
    if (isUnlocked) {
      resetActivityTimer();
    }
  }

  function handleUserActivity() {
    resetActivityTimer();
  }

  // Initialize
  checkVaultStatus();

  // Set up auto-lock event listeners
  function setupAutoLock() {
    // Start auto-lock timer when unlocked
    startAutoLockTimer();

    // Add event listeners for user activity
    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('mousedown', handleUserActivity);
    window.addEventListener('keydown', handleUserActivity);
    window.addEventListener('scroll', handleUserActivity);
    window.addEventListener('touchstart', handleUserActivity);
  }

  function cleanupAutoLock() {
    // Cleanup event listeners
    window.removeEventListener('focus', handleWindowFocus);
    window.removeEventListener('mousedown', handleUserActivity);
    window.removeEventListener('keydown', handleUserActivity);
    window.removeEventListener('scroll', handleUserActivity);
    window.removeEventListener('touchstart', handleUserActivity);
    clearAutoLockTimer();
  }
</script>

<main class="securepass-app">
  {#if loading}
    <div class="loading">
      <h2>Loading SecurePass...</h2>
    </div>
  {:else if !isUnlocked}
    <VaultUnlock on:unlock={handleUnlock} />
  {:else}
    <Dashboard vaultStatus={vaultStatus} onLock={handleLock} />
  {/if}
</main>

<style>
/* SecurePass specific styles */
.securepass-app {
  width: 100%;
  height: 100vh;
  margin: 0;
  padding: 0;
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.loading h2 {
  color: #4a5568;
  margin-bottom: 16px;
}
</style>
