<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  let masterPassword = $state("");
  let confirmPassword = $state("");
  let isCreating = $state(false);
  let loading = $state(false);
  let error = $state("");

  // Check if vault exists
  let vaultExists = $state(false);
  let vaultStatus = $state(null);

  async function checkVaultStatus() {
    try {
      vaultStatus = await invoke("get_vault_status");
      vaultExists = vaultStatus.has_vault;
    } catch (err) {
      console.error("Failed to check vault status:", err);
      // If Tauri is not available, show a helpful message
      if (err.message?.includes("invoke")) {
        error = "Tauri API not available. Please run 'npm run tauri dev' to start the SecurePass application.";
      }
    }
  }

  // Initialize
  checkVaultStatus();

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    loading = true;
    error = "";

    try {
      if (!vaultExists) {
        // Create new vault
        if (masterPassword.length < 8) {
          throw new Error("Master password must be at least 8 characters long");
        }
        if (masterPassword !== confirmPassword) {
          throw new Error("Passwords do not match");
        }

         await invoke("create_vault", {
           request: {
             masterPassword
           }
         });
      } else {
         // Unlock existing vault
         await invoke("unlock_vault", {
           request: {
             masterPassword
           }
         });
      }

      dispatch("unlock");
    } catch (err) {
      console.error("Vault operation failed:", err);
      if (err instanceof Error) {
        const msg = err.message;
        if (msg.includes("invoke")) {
          error = "Tauri API not available. Please run 'npm run tauri dev' to start the SecurePass application.";
        } else if (msg.includes("Incorrect master password") || msg.includes("Decryption failed") || msg.includes("Failed to load vault")) {
          error = "Incorrect master password. Please try again.";
        } else if (msg.includes("Invalid vault file") || msg.includes("Invalid salt") || msg.includes("Invalid nonce")) {
          error = "Vault file appears to be corrupted. Please check your vault file or create a new one.";
        } else {
          error = msg;
        }
      } else if (typeof err === 'string') {
        // Handle string errors from Tauri
        if (err.includes("Incorrect master password") || err.includes("Decryption failed") || err.includes("Failed to load vault")) {
          error = "Incorrect master password. Please try again.";
        } else if (err.includes("Invalid vault file") || err.includes("Invalid salt") || err.includes("Invalid nonce")) {
          error = "Vault file appears to be corrupted. Please check your vault file or create a new one.";
        } else {
          error = err;
        }
      } else {
        error = "An unexpected error occurred";
      }
    } finally {
      loading = false;
    }
  }

  function toggleMode() {
    isCreating = !isCreating;
    masterPassword = "";
    confirmPassword = "";
    error = "";
  }
</script>

<div class="unlock-container">
  <div class="unlock-card">
    <div class="logo">
      <h1>🔐 SecurePass</h1>
      <p>Encrypted Password Manager</p>
    </div>

    <form onsubmit={handleSubmit} class="unlock-form">
      {#if !vaultExists}
        <div class="mode-toggle">
          <button type="button" class="mode-btn {isCreating ? 'active' : ''}" onclick={toggleMode}>
            Create Vault
          </button>
        </div>
      {/if}

      <div class="form-group">
        <label for="master-password">Master Password</label>
        <input
          id="master-password"
          type="password"
          bind:value={masterPassword}
          placeholder="Enter your master password"
          required
          disabled={loading}
        />
      </div>

      {#if !vaultExists}
        <div class="form-group">
          <label for="confirm-password">Confirm Password</label>
          <input
            id="confirm-password"
            type="password"
            bind:value={confirmPassword}
            placeholder="Confirm your master password"
            required
            disabled={loading}
          />
        </div>
      {/if}

      {#if error}
        <div class="error-message">
          {error}
        </div>
      {/if}

      <button type="submit" class="unlock-btn" disabled={loading}>
        {#if loading}
          <span class="spinner"></span>
          {vaultExists ? "Unlocking..." : "Creating..."}
        {:else}
          {vaultExists ? "Unlock Vault" : "Create Vault"}
        {/if}
      </button>
    </form>

    {#if vaultExists && !vaultStatus?.is_locked}
      <div class="vault-info">
        <p>Vault contains {vaultStatus?.entry_count || 0} entries</p>
      </div>
    {/if}

    {#if !vaultExists}
      <div class="security-notice">
        <p><strong>Security Notice:</strong> Your master password is used to encrypt all your data.
        Choose a strong, memorable password and keep it safe.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .unlock-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    padding: 20px;
  }

  .unlock-card {
    background: white;
    border-radius: 12px;
    padding: 40px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
    width: 100%;
    max-width: 400px;
    text-align: center;
  }

  .logo h1 {
    color: #333;
    margin-bottom: 8px;
    font-size: 2rem;
  }

  .logo p {
    color: #666;
    margin-bottom: 30px;
  }

  .mode-toggle {
    margin-bottom: 20px;
  }

  .mode-btn {
    background: none;
    border: 2px solid #667eea;
    color: #667eea;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .mode-btn.active {
    background: #667eea;
    color: white;
  }

  .form-group {
    margin-bottom: 20px;
    text-align: left;
  }

  label {
    display: block;
    margin-bottom: 6px;
    color: #333;
    font-weight: 500;
  }

  input {
    width: 100%;
    padding: 12px;
    border: 2px solid #e1e5e9;
    border-radius: 6px;
    font-size: 16px;
    transition: border-color 0.2s;
    box-sizing: border-box;
  }

  input:focus {
    outline: none;
    border-color: #667eea;
  }

  input:disabled {
    background: #f5f5f5;
    cursor: not-allowed;
  }

  .error-message {
    background: #fee;
    color: #c33;
    padding: 10px;
    border-radius: 6px;
    margin-bottom: 20px;
    border: 1px solid #fcc;
  }

  .unlock-btn {
    width: 100%;
    background: #667eea;
    color: white;
    border: none;
    padding: 14px;
    border-radius: 6px;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .unlock-btn:hover:not(:disabled) {
    background: #5a67d8;
  }

  .unlock-btn:disabled {
    background: #a0aec0;
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

  .vault-info {
    margin-top: 20px;
    padding: 15px;
    background: #f7fafc;
    border-radius: 6px;
    border: 1px solid #e2e8f0;
  }

  .vault-info p {
    margin: 0;
    color: #4a5568;
  }

  .security-notice {
    margin-top: 20px;
    padding: 15px;
    background: #fff5f5;
    border: 1px solid #feb2b2;
    border-radius: 6px;
  }

  .security-notice p {
    margin: 0;
    color: #c53030;
    font-size: 14px;
    line-height: 1.4;
  }

  @media (max-width: 480px) {
    .unlock-card {
      padding: 30px 20px;
    }

    .logo h1 {
      font-size: 1.5rem;
    }
  }
</style>