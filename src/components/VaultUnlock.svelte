<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";
  import Icon from "./Icon.svelte";
  import { passwordStrength } from "$lib/strength";

  const dispatch = createEventDispatcher();

  interface VaultStatus {
    has_vault: boolean;
    is_locked: boolean;
    entry_count: number;
  }

  let masterPassword = $state("");
  let confirmPassword = $state("");
  let showPassword = $state(false);
  let capsLock = $state(false);
  let loading = $state(false);
  let error = $state("");
  let statusLoaded = $state(false);
  let vaultExists = $state(false);
  let appVersion = $state("");

  const strength = $derived(passwordStrength(masterPassword));

  async function checkVaultStatus() {
    try {
      const status = await invoke<VaultStatus>("get_vault_status");
      vaultExists = status.has_vault;
    } catch (err) {
      console.error("Failed to check vault status:", err);
      if (String((err as any)?.message ?? err).includes("invoke")) {
        error = "Tauri API not available. Run 'npm run tauri dev' to start the SecurePass application.";
      }
    } finally {
      statusLoaded = true;
    }
  }

  checkVaultStatus();

  import("@tauri-apps/api/app")
    .then(({ getVersion }) => getVersion())
    .then((v) => (appVersion = v))
    .catch(() => { /* not running inside Tauri */ });

  function trackCapsLock(event: KeyboardEvent) {
    if (typeof event.getModifierState === "function") {
      capsLock = event.getModifierState("CapsLock");
    }
  }

  function friendlyError(err: unknown): string {
    const msg = err instanceof Error ? err.message : typeof err === "string" ? err : "";
    if (!msg) return "An unexpected error occurred.";
    if (msg.includes("invoke")) {
      return "Tauri API not available. Run 'npm run tauri dev' to start the SecurePass application.";
    }
    if (msg.includes("Incorrect master password") || msg.includes("Decryption failed") || msg.includes("Failed to load vault")) {
      return "That password didn't unlock the vault. Check it and try again.";
    }
    if (msg.includes("Invalid vault file") || msg.includes("Invalid salt") || msg.includes("Invalid nonce")) {
      return "The vault file looks damaged. Restore it from a .spvault backup or Google Drive.";
    }
    return msg;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = "";

    if (!vaultExists) {
      if (masterPassword.length < 8) { error = "Use at least 8 characters for your master password."; return; }
      if (masterPassword !== confirmPassword) { error = "The two passwords don't match."; return; }
    } else if (!masterPassword) {
      error = "Enter your master password.";
      return;
    }

    loading = true;
    try {
      if (!vaultExists) {
        await invoke("create_vault", { request: { masterPassword } });
      } else {
        await invoke("unlock_vault", { request: { masterPassword } });
      }
      masterPassword = "";
      confirmPassword = "";
      dispatch("unlock");
    } catch (err) {
      console.error("Vault operation failed");
      error = friendlyError(err);
    } finally {
      loading = false;
    }
  }

  // ── Google Drive restore ──────────────────────────────────────────────────
  // Step 1: connect_google_drive (opens the browser, waits for OAuth)
  // Step 2: restore_vault_from_drive(password) (download + decrypt + save)
  type DriveStep = "idle" | "connecting" | "password" | "restoring";
  let driveStep = $state<DriveStep>("idle");
  let drivePassword = $state("");
  let driveError = $state("");

  async function startDriveRestore() {
    driveStep = "connecting";
    driveError = "";
    try {
      await invoke("connect_google_drive");
      driveStep = "password";
    } catch (e) {
      driveError = e instanceof Error ? e.message : String(e);
      driveStep = "idle";
    }
  }

  async function handleDriveRestore(event?: Event) {
    event?.preventDefault();
    if (!drivePassword) { driveError = "Enter your master password."; return; }
    driveStep = "restoring";
    driveError = "";
    try {
      await invoke("restore_vault_from_drive", { masterPassword: drivePassword });
      drivePassword = "";
      dispatch("unlock");
    } catch (e) {
      driveError = friendlyError(e);
      driveStep = "password";
    }
  }

  function cancelDriveRestore() {
    driveStep = "idle";
    drivePassword = "";
    driveError = "";
  }
</script>

<div class="unlock">
  <aside class="brand-panel">
    <div class="brand">
      <span class="mark"><Icon name="brand" size={22} /></span>
      <span class="name">SecurePass</span>
    </div>

    <div class="pitch">
      <h2>Your passwords. Your device. Your cloud.</h2>
      <ul>
        <li>
          <Icon name="shield" size={20} />
          <div>
            <strong>Encrypted on this device</strong>
            <span>ChaCha20-Poly1305 with an Argon2id key. Nothing is stored unencrypted.</span>
          </div>
        </li>
        <li>
          <Icon name="cloud" size={20} />
          <div>
            <strong>Synced through your own Drive</strong>
            <span>Only an encrypted file. Changes merge entry by entry.</span>
          </div>
        </li>
        <li>
          <Icon name="clock" size={20} />
          <div>
            <strong>Locks itself</strong>
            <span>Auto-lock after inactivity; copied secrets clear from the clipboard.</span>
          </div>
        </li>
      </ul>
    </div>

    <div class="version">{appVersion ? `Version ${appVersion}` : ""}</div>
  </aside>

  <main class="form-panel">
    {#if !statusLoaded}
      <span class="spinner big" aria-label="Loading"></span>
    {:else if driveStep === "password" || driveStep === "restoring"}
      <form class="card" onsubmit={handleDriveRestore}>
        <div>
          <h1>Restore from Google Drive</h1>
          <p class="lead">Google Drive is connected. Enter the master password you use on your other device.</p>
        </div>
        <div class="field">
          <label class="label" for="drive-password">Master password</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="drive-password"
            class="input big-input"
            type="password"
            bind:value={drivePassword}
            disabled={driveStep === "restoring"}
            autocomplete="current-password"
            autofocus
            onkeydown={trackCapsLock}
            onkeyup={trackCapsLock}
          />
          {#if capsLock}<div class="caps">Caps Lock is on</div>{/if}
        </div>
        {#if driveError}<div class="msg msg-error" role="alert">{driveError}</div>{/if}
        <button type="submit" class="btn btn-primary btn-lg" disabled={driveStep === "restoring"}>
          {#if driveStep === "restoring"}<span class="spinner"></span> Restoring…{:else}Restore vault{/if}
        </button>
        <button type="button" class="btn btn-ghost" onclick={cancelDriveRestore} disabled={driveStep === "restoring"}>Cancel</button>
      </form>
    {:else}
      <form class="card" onsubmit={handleSubmit}>
        <div>
          <h1>{vaultExists ? "Unlock your vault" : "Create your vault"}</h1>
          <p class="lead">
            {vaultExists
              ? "Enter your master password to continue."
              : "Choose a master password. It encrypts everything you store, and it can't be recovered — so make it strong and memorable."}
          </p>
        </div>

        <div class="field">
          <label class="label" for="master-password">Master password</label>
          <div class="input-group big">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              id="master-password"
              type={showPassword ? "text" : "password"}
              bind:value={masterPassword}
              disabled={loading}
              autocomplete={vaultExists ? "current-password" : "new-password"}
              autofocus
              onkeydown={trackCapsLock}
              onkeyup={trackCapsLock}
            />
            <button
              type="button"
              class="icon-btn"
              onclick={() => (showPassword = !showPassword)}
              aria-label={showPassword ? "Hide password" : "Show password"}
              title={showPassword ? "Hide password" : "Show password"}
            >
              <Icon name={showPassword ? "eye-off" : "eye"} size={18} />
            </button>
          </div>
          {#if capsLock}<div class="caps">Caps Lock is on</div>{/if}
          {#if !vaultExists && masterPassword}
            <div class="bars" aria-hidden="true">
              {#each [1, 2, 3, 4] as i}
                <span style="background: {i <= strength.score ? strength.color : 'var(--strength-empty)'}"></span>
              {/each}
            </div>
            <div class="strength" style="color: {strength.color}">{strength.label}</div>
          {/if}
        </div>

        {#if !vaultExists}
          <div class="field">
            <label class="label" for="confirm-password">Confirm master password</label>
            <input
              id="confirm-password"
              class="input big-input"
              type={showPassword ? "text" : "password"}
              bind:value={confirmPassword}
              disabled={loading}
              autocomplete="new-password"
            />
          </div>
        {/if}

        {#if error}<div class="msg msg-error" role="alert">{error}</div>{/if}

        <button type="submit" class="btn btn-primary btn-lg" disabled={loading}>
          {#if loading}
            <span class="spinner"></span> {vaultExists ? "Unlocking…" : "Creating…"}
          {:else}
            {vaultExists ? "Unlock" : "Create vault"}
          {/if}
        </button>

        {#if !vaultExists}
          <div class="divider"><span>or</span></div>
          <button type="button" class="btn btn-secondary restore" onclick={startDriveRestore} disabled={driveStep === "connecting"}>
            {#if driveStep === "connecting"}
              <span class="spinner"></span> Waiting for Google sign-in…
            {:else}
              <Icon name="cloud-restore" size={17} /> Restore from Google Drive
            {/if}
          </button>
          <p class="foot">
            {driveStep === "connecting"
              ? "Finish signing in in your browser, then come back here."
              : "Already use SecurePass on another device? Restore your vault from Drive."}
          </p>
          {#if driveError}<div class="msg msg-error">{driveError}</div>{/if}
        {:else}
          <p class="foot">There is no password recovery. Keep an exported .spvault backup somewhere safe.</p>
        {/if}
      </form>
    {/if}
  </main>
</div>

<style>
  .unlock { display: flex; min-height: 100vh; }

  .brand-panel {
    flex: 0 0 42%;
    max-width: 560px;
    background: var(--brand-panel);
    color: var(--brand-panel-text);
    padding: 48px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 40px;
    border-right: 1px solid var(--border-subtle);
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .mark {
    width: 34px; height: 34px; border-radius: 9px;
    background: linear-gradient(#2c313b, #1a1d23);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #c9d5ff;
    --brand-keyhole: #1a1d23;
    display: flex; align-items: center; justify-content: center;
  }
  .name { font-weight: 700; font-size: 17px; }

  .pitch { display: flex; flex-direction: column; gap: 28px; max-width: 420px; }
  .pitch h2 { margin: 0; font-size: 32px; line-height: 1.15; font-weight: 700; letter-spacing: -0.02em; }
  .pitch ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 18px; }
  .pitch li { display: flex; gap: 12px; }
  .pitch li :global(svg) { color: var(--brand-panel-accent); margin-top: 2px; }
  .pitch li div { display: flex; flex-direction: column; }
  .pitch li span { color: var(--brand-panel-muted); font-size: 13px; }

  .version { font-size: 12px; color: var(--brand-panel-muted); min-height: 1em; }

  .form-panel {
    flex: 1;
    background: var(--bg-sidebar);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
  }

  .card { width: 100%; max-width: 380px; display: flex; flex-direction: column; gap: 18px; }
  h1 { margin: 0 0 6px; font-size: 24px; font-weight: 700; letter-spacing: -0.015em; }
  .lead { margin: 0; color: var(--text-muted); }

  .input-group.big { border-radius: 10px; }
  .input-group.big input { min-height: 46px; font-size: 16px; padding: 0 14px; }
  .big-input { min-height: 48px; font-size: 16px; border-radius: 10px; padding: 0 14px; }

  .caps { font-size: 12px; color: var(--warning-text); font-weight: 500; }

  .bars { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; margin-top: 4px; }
  .bars span { height: 4px; border-radius: 2px; }
  .strength { font-size: 12px; font-weight: 600; }

  .divider { display: flex; align-items: center; gap: 12px; color: var(--text-muted); font-size: 12px; }
  .divider::before, .divider::after { content: ""; flex: 1; height: 1px; background: var(--border-strong); }

  .restore { min-height: 44px; border-radius: 10px; font-size: 14px; }
  .foot { margin: 0; font-size: 12px; color: var(--text-muted); }

  .spinner.big { width: 28px; height: 28px; color: var(--text-muted); }

  @media (max-width: 860px) {
    .brand-panel { display: none; }
  }
</style>
