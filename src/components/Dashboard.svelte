<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Sidebar from "./Sidebar.svelte";
  import EntryList from "./EntryList.svelte";
  import EntryForm from "./EntryForm.svelte";
  import EntryView from "./EntryView.svelte";
  import Settings from "./Settings.svelte";
  import ConflictResolutionModal from "./ConflictResolutionModal.svelte";
  import Icon from "./Icon.svelte";
  import { categoryMeta } from "$lib/categories";

  interface Props {
    vaultStatus: any;
    onLock: () => void;
  }

  let { onLock }: Props = $props();

  // ── Entries ────────────────────────────────────────────────────────────────
  // allEntries feeds the sidebar counts; entries is what the list shows
  // (the whole vault, or search / category results from the backend).
  let allEntries = $state<any[]>([]);
  let entries = $state<any[]>([]);
  let searchQuery = $state("");
  let selectedCategory = $state("");
  let loading = $state(false);
  let searchInput = $state<HTMLInputElement | null>(null);

  // ── Detail pane ────────────────────────────────────────────────────────────
  type PaneMode = "empty" | "view" | "edit" | "new";
  let paneMode = $state<PaneMode>("empty");
  let selectedId = $state<string | null>(null);
  let selectedEntry = $state<any>(null);
  let paneError = $state("");

  // ── Modals / banners ───────────────────────────────────────────────────────
  let showSettings = $state(false);
  let updateInfo = $state<{ latestVersion: string; currentVersion?: string; downloadUrl: string } | null>(null);
  let notice = $state<{ kind: "success" | "error"; text: string } | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout> | null = null;

  // Clipboard toast with a countdown until the clipboard is cleared.
  let toast = $state<{ label: string; total: number; left: number } | null>(null);
  let toastTimer: ReturnType<typeof setInterval> | null = null;

  // ── Sync ───────────────────────────────────────────────────────────────────
  let syncStatus = $state<{ connected: boolean; lastSync: string | null; hasPendingConflicts: boolean } | null>(null);
  let syncing = $state(false);
  let syncError = $state("");
  let pendingConflicts = $state<any[]>([]);
  let showConflictModal = $state(false);
  let needsSync = $state(false);
  let syncCheckReason = $state<{ localChanged: boolean; remoteChanged: boolean }>({ localChanged: false, remoteChanged: false });

  // Re-auth (master password changed on another device)
  let showReauthModal = $state(false);
  let reauthPassword = $state("");
  let reauthError = $state("");
  let reauthLoading = $state(false);

  // Import .spvault
  let showImportModal = $state(false);
  let importFilePath = $state("");
  let importPassword = $state("");
  let importError = $state("");
  let importLoading = $state(false);

  const categoryCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const e of allEntries) {
      const c = e.category || "generic";
      counts[c] = (counts[c] ?? 0) + 1;
    }
    return counts;
  });

  const heading = $derived.by(() => {
    if (searchQuery) return `Results for “${searchQuery}”`;
    if (selectedCategory) return categoryMeta(selectedCategory).label;
    return "All items";
  });

  const syncTitle = $derived(
    syncStatus?.lastSync
      ? `Last synced: ${new Date(syncStatus.lastSync).toLocaleString()}` +
          (syncCheckReason.remoteChanged ? " · Google Drive has changes" : "") +
          (syncCheckReason.localChanged ? " · This device has changes" : "")
      : "Never synced"
  );

  // ── Notices ────────────────────────────────────────────────────────────────
  function showNotice(kind: "success" | "error", text: string) {
    notice = { kind, text };
    if (noticeTimer) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = null), kind === "error" ? 8000 : 5000);
  }

  function handleCopied(label: string, seconds: number) {
    if (toastTimer) clearInterval(toastTimer);
    toast = { label, total: seconds, left: seconds };
    toastTimer = setInterval(() => {
      if (!toast) return;
      const left = toast.left - 1;
      if (left <= 0) {
        toast = null;
        if (toastTimer) clearInterval(toastTimer);
        toastTimer = null;
      } else {
        toast = { ...toast, left };
      }
    }, 1000);
  }

  // ── Data loading ───────────────────────────────────────────────────────────
  async function loadAll() {
    try {
      allEntries = await invoke<any[]>("get_entries");
    } catch (error) {
      console.error("Failed to load entries:", error);
    }
  }

  async function refreshList() {
    loading = true;
    try {
      if (searchQuery || selectedCategory) {
        entries = await invoke<any[]>("search_entries", {
          request: { query: searchQuery, categoryFilter: selectedCategory || null },
        });
      } else {
        entries = await invoke<any[]>("get_entries");
      }
    } catch (error) {
      console.error("Failed to load entries:", error);
    } finally {
      loading = false;
    }
  }

  async function reloadData() {
    await Promise.all([loadAll(), refreshList()]);
  }

  // Re-query whenever the search text or category filter changes.
  $effect(() => {
    void searchQuery;
    void selectedCategory;
    refreshList();
  });

  // ── Selection / pane ───────────────────────────────────────────────────────
  async function selectEntry(entry: { id: string }) {
    if ((paneMode === "edit" || paneMode === "new") && !confirmDiscard()) return;
    selectedId = entry.id;
    paneError = "";
    try {
      selectedEntry = await invoke<any>("get_entry", { request: { entryId: entry.id } });
      paneMode = "view";
    } catch (error) {
      console.error("Failed to load entry:", error);
      paneError = "Couldn't open that entry: " + (error instanceof Error ? error.message : String(error));
      selectedEntry = null;
      paneMode = "empty";
    }
  }

  function confirmDiscard(): boolean {
    return confirm("Discard your unsaved changes?");
  }

  function handleCreateEntry() {
    if ((paneMode === "edit" || paneMode === "new") && !confirmDiscard()) return;
    selectedEntry = null;
    paneMode = "new";
  }

  function handleEditEntry() {
    if (selectedEntry) paneMode = "edit";
  }

  function handleFormCancel() {
    paneMode = selectedEntry ? "view" : "empty";
  }

  async function handleSaved(entryId: string) {
    await reloadData();
    paneMode = "empty";
    await selectEntry({ id: entryId });
    autoSync();
  }

  async function handleDeleted() {
    selectedEntry = null;
    selectedId = null;
    paneMode = "empty";
    await reloadData();
    autoSync();
  }

  // ── Import / export ────────────────────────────────────────────────────────
  async function handleExport() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        title: "Export SecurePass Vault",
        filters: [{ name: "SecurePass Vault", extensions: ["spvault"] }],
        defaultPath: `securepass-backup-${new Date().toISOString().split("T")[0]}.spvault`,
      });
      if (filePath) {
        await invoke("export_vault", { request: { filePath } });
        showNotice("success", "Vault exported. Keep the backup somewhere safe — it opens with your master password.");
      }
    } catch (error) {
      console.error("Failed to export vault:", error);
      showNotice("error", "Export failed: " + error);
    }
  }

  // Supports .spvault (SecurePass) and .json (PasswordLocker export).
  async function handleImport() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({
        title: "Import Vault",
        filters: [
          { name: "SecurePass Vault", extensions: ["spvault"] },
          { name: "PasswordLocker Export", extensions: ["json"] },
        ],
        multiple: false,
      });
      if (!filePath) return;

      if ((filePath as string).endsWith(".json")) {
        const count = await invoke("import_from_password_locker", { request: { filePath } });
        await reloadData();
        showNotice("success", `Imported ${count} entries from PasswordLocker.`);
        autoSync();
      } else {
        importFilePath = filePath as string;
        importPassword = "";
        importError = "";
        showImportModal = true;
      }
    } catch (error) {
      console.error("Failed to import vault:", error);
      showNotice("error", "Import failed: " + error);
    }
  }

  function closeImportModal() {
    showImportModal = false;
    importPassword = "";
    importError = "";
  }

  async function handleImportConfirm() {
    if (!importPassword) { importError = "Enter the master password for the vault file."; return; }
    importLoading = true;
    importError = "";
    try {
      const result: any = await invoke("import_vault", { request: { filePath: importFilePath, masterPassword: importPassword } });
      showImportModal = false;
      importPassword = "";
      importFilePath = "";
      await reloadData();
      showNotice(
        "success",
        result.entriesAdded > 0
          ? `Import complete. ${result.entriesAdded} new ${result.entriesAdded === 1 ? "entry" : "entries"} added.`
          : "Import complete. Nothing new — your vault already has everything from that file."
      );
      autoSync();
    } catch (e) {
      importError = e instanceof Error ? e.message : String(e);
    } finally {
      importLoading = false;
    }
  }

  // ── Sync ───────────────────────────────────────────────────────────────────
  async function loadSyncStatus() {
    try {
      syncStatus = await invoke("get_sync_status");
      if (syncStatus?.hasPendingConflicts) {
        syncError = "";
      }
    } catch { /* non-fatal */ }
  }

  async function checkSyncNeeded() {
    if (!syncStatus?.connected) return;
    try {
      const result: any = await invoke("check_sync_needed");
      needsSync = result.needsSync;
      syncCheckReason = { localChanged: result.localChanged, remoteChanged: result.remoteChanged };
    } catch { /* silent — network may be unavailable */ }
  }

  async function handleSyncResult(result: any, reload: boolean) {
    if (result.status === "password_changed") {
      showReauthModal = true;
    } else if (result.status === "conflicts") {
      pendingConflicts = result.conflicts;
      showConflictModal = true;
      await loadSyncStatus();
    } else {
      needsSync = false;
      if (reload) await reloadData();
      await loadSyncStatus();
    }
  }

  async function handleSync() {
    syncing = true;
    syncError = "";
    try {
      const result: any = await invoke("sync_vault");
      await handleSyncResult(result, true);
    } catch (e) {
      syncError = e instanceof Error ? e.message : String(e);
    } finally {
      syncing = false;
    }
  }

  // Background sync after local writes. Only when Drive is connected; errors
  // are silent because the manual "Sync now" button is always available.
  async function autoSync() {
    if (!syncStatus?.connected || syncing) return;
    syncing = true;
    try {
      const result: any = await invoke("sync_vault");
      await handleSyncResult(result, false);
    } catch {
      /* silent */
    } finally {
      syncing = false;
    }
  }

  function closeReauthModal() {
    showReauthModal = false;
    reauthPassword = "";
    reauthError = "";
  }

  async function handleReauthAndSync() {
    if (!reauthPassword) { reauthError = "Enter the new master password."; return; }
    reauthLoading = true;
    reauthError = "";
    try {
      const result: any = await invoke("reauth_and_sync", { newMasterPassword: reauthPassword });
      showReauthModal = false;
      reauthPassword = "";
      await handleSyncResult(result, true);
    } catch (e) {
      reauthError = e instanceof Error ? e.message : String(e);
    } finally {
      reauthLoading = false;
    }
  }

  async function handleConflictsResolved() {
    showConflictModal = false;
    pendingConflicts = [];
    syncError = "";
    needsSync = false;
    await reloadData();
    await loadSyncStatus();
    // Show the kept version of the entry that is open, if any.
    if (paneMode === "view" && selectedId) await selectEntry({ id: selectedId });
  }

  function handleSettingsClose() {
    showSettings = false;
    // Drive may have been connected/disconnected or the vault re-keyed.
    loadSyncStatus().then(() => checkSyncNeeded());
  }

  // ── Startup ────────────────────────────────────────────────────────────────
  loadAll();
  loadSyncStatus().then(() => checkSyncNeeded());

  invoke("check_for_update")
    .then((result: any) => { if (result.hasUpdate) updateInfo = result; })
    .catch(() => { /* offline — ignore */ });

  // Poll Drive for remote changes every 2 minutes when connected.
  $effect(() => {
    if (!syncStatus?.connected) return;
    const id = setInterval(checkSyncNeeded, 2 * 60 * 1000);
    return () => clearInterval(id);
  });

  $effect(() => () => {
    if (toastTimer) clearInterval(toastTimer);
    if (noticeTimer) clearTimeout(noticeTimer);
  });

  // ── Keyboard shortcuts ─────────────────────────────────────────────────────
  function anyModalOpen() {
    return showSettings || showConflictModal || showImportModal || showReauthModal;
  }

  function handleKeydown(event: KeyboardEvent) {
    const mod = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();

    // Search focus works everywhere, even from inside an input.
    if (mod && (key === "k" || key === "f") && !anyModalOpen()) {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }

    const inField =
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      event.target instanceof HTMLSelectElement;

    if (event.key === "Escape" && event.target === searchInput && searchQuery) {
      searchQuery = "";
      return;
    }
    if (inField) return;

    if (mod && key === "n") {
      event.preventDefault();
      handleCreateEntry();
    } else if (mod && key === "l") {
      event.preventDefault();
      onLock();
    } else if (event.key === "Escape" && !anyModalOpen()) {
      if (paneMode === "edit" || paneMode === "new") handleFormCancel();
    }
  }

  $effect(() => {
    document.addEventListener("keydown", handleKeydown);
    return () => document.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="app">
  <Sidebar
    bind:query={searchQuery}
    bind:selectedCategory
    bind:searchInput
    {categoryCounts}
    totalCount={allEntries.length}
    {syncStatus}
    {syncing}
    {needsSync}
    {syncTitle}
    {syncError}
    onSync={handleSync}
    onResolveConflicts={() => (pendingConflicts.length > 0 ? (showConflictModal = true) : handleSync())}
    onOpenSettings={() => (showSettings = true)}
    onImport={handleImport}
    onExport={handleExport}
    onLock={onLock}
  />

  <div class="main">
    {#if updateInfo}
      <div class="banner" role="status">
        <Icon name="sparkle" size={16} />
        <span class="banner-text">
          SecurePass <strong>v{updateInfo.latestVersion}</strong> is available — you're on v{updateInfo.currentVersion ?? "…"}.
        </span>
        <a href={updateInfo.downloadUrl} target="_blank" rel="noopener" class="btn btn-primary banner-btn">Download</a>
        <button class="icon-btn" onclick={() => (updateInfo = null)} aria-label="Dismiss update notice">
          <Icon name="close" size={15} />
        </button>
      </div>
    {/if}

    <div class="panes">
      <EntryList
        {entries}
        {heading}
        {selectedId}
        {loading}
        filtered={!!(searchQuery || selectedCategory)}
        showCategory={!selectedCategory}
        onSelect={selectEntry}
        onCreate={handleCreateEntry}
      />

      <section class="detail" aria-label="Entry details">
        {#if paneMode === "new"}
          <EntryForm onSaved={handleSaved} onCancel={handleFormCancel} />
        {:else if paneMode === "edit" && selectedEntry}
          {#key selectedEntry.id}
            <EntryForm entry={selectedEntry} onSaved={handleSaved} onCancel={handleFormCancel} />
          {/key}
        {:else if paneMode === "view" && selectedEntry}
          <EntryView entry={selectedEntry} onEdit={handleEditEntry} onDeleted={handleDeleted} onCopied={handleCopied} />
        {:else}
          <div class="empty">
            <span class="empty-mark"><Icon name="shield" size={26} strokeWidth={1.8} /></span>
            {#if paneError}
              <p class="msg msg-error">{paneError}</p>
            {:else if allEntries.length === 0}
              <h2>Add your first password</h2>
              <p>Everything you save is encrypted on this device before it's written to disk.</p>
              <button class="btn btn-primary" onclick={handleCreateEntry}><Icon name="plus" size={14} /> New entry</button>
            {:else}
              <h2>Select an entry</h2>
              <p>Pick an item from the list to see its details, or create a new one.</p>
              <div class="hints">
                <span><span class="kbd">Ctrl K</span> search</span>
                <span><span class="kbd">Ctrl N</span> new entry</span>
                <span><span class="kbd">Ctrl L</span> lock</span>
              </div>
            {/if}
          </div>
        {/if}

        {#if notice}
          <div class="notice msg {notice.kind === 'error' ? 'msg-error' : 'msg-success'}" role="status">
            <span>{notice.text}</span>
            <button class="icon-btn" onclick={() => (notice = null)} aria-label="Dismiss"><Icon name="close" size={14} /></button>
          </div>
        {/if}

        {#if toast}
          <div class="toast" role="status">
            <Icon name="check" size={15} />
            <span>{toast.label} copied</span>
            <span class="track" aria-hidden="true"><span style="width: {(toast.left / toast.total) * 100}%"></span></span>
            <span class="toast-muted">clears in {toast.left}s</span>
          </div>
        {/if}
      </section>
    </div>
  </div>
</div>

{#if showSettings}
  <Settings onClose={handleSettingsClose} />
{/if}

{#if showConflictModal && pendingConflicts.length > 0}
  <ConflictResolutionModal
    conflicts={pendingConflicts}
    onResolved={handleConflictsResolved}
    onCancel={() => (showConflictModal = false)}
  />
{/if}

{#if showImportModal}
  <div class="modal-overlay" role="presentation" onclick={closeImportModal} onkeydown={(e) => e.key === "Escape" && closeImportModal()}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="import-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <div>
          <h2 id="import-title">Import vault backup</h2>
          <p class="subtitle">Entries from the file are merged into this vault. Nothing here is removed.</p>
        </div>
      </div>
      <div class="modal-body">
        <div class="field">
          <label class="label" for="import-pw">Master password of the backup</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="import-pw"
            class="input"
            type="password"
            bind:value={importPassword}
            disabled={importLoading}
            autofocus
            onkeydown={(e) => { if (e.key === "Enter") handleImportConfirm(); if (e.key === "Escape") closeImportModal(); }}
          />
        </div>
        {#if importError}
          <div class="msg msg-error">{importError}</div>
        {/if}
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={closeImportModal} disabled={importLoading}>Cancel</button>
        <button class="btn btn-primary" onclick={handleImportConfirm} disabled={importLoading}>
          {#if importLoading}<span class="spinner"></span> Importing…{:else}Import{/if}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showReauthModal}
  <div class="modal-overlay" role="presentation" onclick={closeReauthModal} onkeydown={(e) => e.key === "Escape" && closeReauthModal()}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="reauth-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <div>
          <h2 id="reauth-title">Master password changed</h2>
          <p class="subtitle">
            The vault on Google Drive was re-encrypted with a new master password on another device.
            Enter the new password to sync and update this device.
          </p>
        </div>
      </div>
      <div class="modal-body">
        <div class="field">
          <label class="label" for="reauth-pw">New master password</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="reauth-pw"
            class="input"
            type="password"
            bind:value={reauthPassword}
            disabled={reauthLoading}
            autofocus
            onkeydown={(e) => { if (e.key === "Enter") handleReauthAndSync(); if (e.key === "Escape") closeReauthModal(); }}
          />
        </div>
        {#if reauthError}
          <div class="msg msg-error">{reauthError}</div>
        {/if}
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={closeReauthModal} disabled={reauthLoading}>Cancel</button>
        <button class="btn btn-primary" onclick={handleReauthAndSync} disabled={reauthLoading}>
          {#if reauthLoading}<span class="spinner"></span> Syncing…{:else}Sync and update password{/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .app { display: flex; height: 100vh; overflow: hidden; background: var(--bg); }

  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; }

  .banner {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 12px 8px 16px;
    background: var(--accent-soft); color: var(--text); border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .banner :global(svg) { color: var(--accent-text); }
  .banner-text { flex: 1; }
  .banner-btn { min-height: 30px; font-size: 12px; text-decoration: none; }

  .panes { flex: 1; min-height: 0; display: flex; }

  .detail { flex: 1; min-width: 0; position: relative; overflow-y: auto; background: var(--bg); }

  .empty {
    height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px;
    padding: 32px; text-align: center; color: var(--text-muted);
  }
  .empty-mark {
    width: 56px; height: 56px; border-radius: 16px; margin-bottom: 8px;
    display: flex; align-items: center; justify-content: center;
    background: var(--chip); color: var(--text-2);
  }
  .empty h2 { margin: 0; font-size: 17px; color: var(--text); }
  .empty p { margin: 0; max-width: 340px; font-size: 13px; }
  .empty .btn { margin-top: 10px; }
  .hints { display: flex; gap: 16px; margin-top: 14px; font-size: 12px; flex-wrap: wrap; justify-content: center; }
  .hints .kbd { margin-right: 4px; }

  .notice {
    position: sticky; bottom: 16px; margin: 0 24px 16px; z-index: 5;
    display: flex; align-items: center; gap: 8px; box-shadow: var(--shadow-lg);
  }
  .notice span { flex: 1; }
  .notice .icon-btn { width: 28px; height: 28px; color: inherit; }

  .toast {
    position: fixed; right: 24px; bottom: 24px; z-index: 900;
    display: flex; align-items: center; gap: 10px;
    padding: 10px 14px; border-radius: 10px;
    background: var(--toast-bg); color: var(--toast-text);
    font-size: 13px; font-weight: 500; box-shadow: var(--shadow-lg);
  }
  .track { width: 60px; height: 3px; border-radius: 2px; background: var(--toast-track); overflow: hidden; }
  .track span { display: block; height: 100%; background: var(--toast-text); transition: width 1s linear; }
  .toast-muted { color: var(--toast-muted); font-weight: 400; font-variant-numeric: tabular-nums; }
</style>
