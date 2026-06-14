<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";
  import EntryList from "./EntryList.svelte";
  import EntryForm from "./EntryForm.svelte";
  import EntryView from "./EntryView.svelte";
  import SearchBar from "./SearchBar.svelte";
  import Settings from "./Settings.svelte";
  import ConflictResolutionModal from "./ConflictResolutionModal.svelte";

  const dispatch = createEventDispatcher();

  // Props
  interface Props {
    vaultStatus: any;
    onLock: () => void;
  }

  let { vaultStatus, onLock }: Props = $props();

  // State
  let entries = $state<any[]>([]);
  let categories = $state<any[]>([]);
  let tags = $state<any[]>([]);
  let searchQuery = $state("");
  let selectedCategory = $state("");
  let showEntryForm = $state(false);
  let showEntryView = $state(false);
  let showSettings = $state(false);
  let editingEntry = $state<any>(null);
  let viewingEntry = $state<any>(null);
  let loading = $state(false);

  // Sync state
  let syncStatus = $state<{ connected: boolean; lastSync: string | null; hasPendingConflicts: boolean } | null>(null);
  let syncing = $state(false);
  let syncError = $state("");
  let pendingConflicts = $state<any[]>([]);
  let showConflictModal = $state(false);
  let needsSync = $state(false);
  let syncCheckReason = $state<{ localChanged: boolean; remoteChanged: boolean }>({ localChanged: false, remoteChanged: false });

  // Re-auth state (password changed on another device)
  let showReauthModal = $state(false);
  let reauthPassword = $state("");
  let reauthError = $state("");
  let reauthLoading = $state(false);

  // Load data on mount
  async function loadData() {
    loading = true;
    try {
      [entries, categories, tags] = await Promise.all([
        invoke<any[]>("get_entries"),
        invoke<any[]>("get_categories"),
        invoke<any[]>("get_tags")
      ]);
    } catch (error) {
      console.error("Failed to load data:", error);
    } finally {
      loading = false;
    }
  }

  // Handle search
  async function handleSearch() {
    loading = true;
    try {
      entries = await invoke<any[]>("search_entries", {
        request: {
          query: searchQuery,
          categoryFilter: selectedCategory || null
        }
      });
    } catch (error) {
      console.error("Search failed:", error);
    } finally {
      loading = false;
    }
  }

  // Handle entry creation
  function handleCreateEntry() {
    editingEntry = null;
    showEntryForm = true;
  }

  // Handle entry viewing
  async function handleViewEntry(entry: any) {
    try {
      const fullEntry = await invoke<any>("get_entry", { request: { entryId: entry.id } });
      viewingEntry = fullEntry;
      showEntryView = true;
    } catch (error) {
      console.error("Failed to load entry:", error);
    }
  }

  // Handle entry editing
  async function handleEditEntry(entry: any) {
    try {
      const fullEntry = await invoke<any>("get_entry", { request: { entryId: entry.id } });
      editingEntry = fullEntry;
      showEntryForm = true;
    } catch (error) {
      console.error("Failed to load entry:", error);
    }
  }

  // Handle entry view close
  function handleEntryViewClose() {
    showEntryView = false;
    viewingEntry = null;
  }

  // Handle entry deletion
  function handleEntryDelete() {
    showEntryView = false;
    viewingEntry = null;
    loadData(); // Refresh data after deletion
  }

  // Handle entry form close
  function handleEntryFormClose() {
    showEntryForm = false;
    editingEntry = null;
    loadData(); // Refresh data
  }

  // Handle lock vault
  function handleLock() {
    onLock();
  }

  // Handle export vault
  async function handleExport() {
    try {
      // Use Tauri's dialog API to select save location
      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        title: "Export SecurePass Vault",
        filters: [{
          name: "SecurePass Vault",
          extensions: ["spvault"]
        }],
        defaultPath: `securepass-backup-${new Date().toISOString().split('T')[0]}.spvault`
      });

      if (filePath) {
        await invoke("export_vault", { request: { filePath } });
        alert("Vault exported successfully!");
      }
    } catch (error) {
      console.error("Failed to export vault:", error);
      alert("Failed to export vault: " + error);
    }
  }

  // Handle import from PasswordLocker JSON export
  async function handleImportFromPasswordLocker() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({
        title: "Import from PasswordLocker",
        filters: [{ name: "PasswordLocker Export", extensions: ["json"] }],
        multiple: false
      });

      if (filePath) {
        const count = await invoke("import_from_password_locker", { request: { filePath } });
        await loadData();
        alert(`Imported ${count} entries from PasswordLocker!`);
      }
    } catch (error) {
      console.error("Failed to import from PasswordLocker:", error);
      alert("Import failed: " + error);
    }
  }

  // Handle import vault — supports .spvault (SecurePass) and .json (PasswordLocker export)
  async function handleImport() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({
        title: "Import Vault",
        filters: [
          { name: "SecurePass Vault", extensions: ["spvault"] },
          { name: "PasswordLocker Export", extensions: ["json"] }
        ],
        multiple: false
      });

      if (!filePath) return;

      if ((filePath as string).endsWith(".json")) {
        // PasswordLocker JSON export
        const count = await invoke("import_from_password_locker", { request: { filePath } });
        await loadData();
        alert(`Imported ${count} entries from PasswordLocker!`);
      } else {
        // SecurePass .spvault
        const masterPassword = prompt("Enter the master password for the vault you want to import:");
        if (!masterPassword) return;

        await invoke("import_vault", { request: { filePath, masterPassword } });
        await loadData();
        alert("Vault imported successfully!");
      }
    } catch (error) {
      console.error("Failed to import vault:", error);
      alert("Failed to import vault: " + error);
    }
  }

  // Sync helpers
  async function loadSyncStatus() {
    try {
      syncStatus = await invoke("get_sync_status");
      if (syncStatus?.hasPendingConflicts) {
        syncError = "Unresolved sync conflicts — click Resolve to fix them.";
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

  async function handleSync() {
    syncing = true;
    syncError = "";
    try {
      const result: any = await invoke("sync_vault");
      if (result.status === "password_changed") {
        showReauthModal = true;
      } else if (result.status === "conflicts") {
        pendingConflicts = result.conflicts;
        showConflictModal = true;
      } else {
        needsSync = false;
        await loadData();
        await loadSyncStatus();
      }
    } catch (e) {
      syncError = e instanceof Error ? e.message : String(e);
    } finally {
      syncing = false;
    }
  }

  async function handleReauthAndSync() {
    if (!reauthPassword) { reauthError = "Enter the new master password."; return; }
    reauthLoading = true;
    reauthError = "";
    try {
      const result: any = await invoke("reauth_and_sync", { newMasterPassword: reauthPassword });
      showReauthModal = false;
      reauthPassword = "";
      if (result.status === "conflicts") {
        pendingConflicts = result.conflicts;
        showConflictModal = true;
      } else {
        needsSync = false;
        await loadData();
        await loadSyncStatus();
      }
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
    await loadData();
    await loadSyncStatus();
  }

  function formatLastSync(iso: string | null) {
    if (!iso) return "";
    const d = new Date(iso);
    const diffMs = Date.now() - d.getTime();
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    return d.toLocaleDateString();
  }

  // Initialize
  loadData();
  loadSyncStatus().then(() => checkSyncNeeded());

  // Poll Drive for remote changes every 2 minutes when connected.
  $effect(() => {
    if (!syncStatus?.connected) return;
    const id = setInterval(checkSyncNeeded, 2 * 60 * 1000);
    return () => clearInterval(id);
  });

  // Keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    // Don't trigger shortcuts when typing in inputs
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      switch (event.key.toLowerCase()) {
        case 'n':
          event.preventDefault();
          handleCreateEntry();
          break;
        case 'f':
          event.preventDefault();
          // Focus search input (will be implemented when SearchBar exposes focus method)
          break;
        case 'l':
          event.preventDefault();
          handleLock();
          break;
      }
    } else if (event.key === 'Escape') {
      // Close any open modals
      if (showEntryView) {
        handleEntryViewClose();
      } else if (showEntryForm) {
        handleEntryFormClose();
      } else if (showSettings) {
        showSettings = false;
      }
    }
  }

  // Reactive search
  $effect(() => {
    if (searchQuery || selectedCategory) {
      handleSearch();
    } else {
      loadData();
    }
  });

  // Add keyboard event listener
  $effect(() => {
    document.addEventListener('keydown', handleKeydown);
    return () => document.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="dashboard">
  <!-- Header -->
  <header class="dashboard-header">
    <div class="header-left">
      <h1>🔐 SecurePass</h1>
      <span class="vault-status">
        {vaultStatus?.entry_count || 0} entries
      </span>
    </div>
    <div class="header-right">
      <button class="btn-outline" onclick={() => showSettings = true} title="Settings">
        ⚙️ Settings
      </button>
      {#if syncStatus?.connected}
        <button
          class="btn-sync"
          class:needs-sync={needsSync && !syncing}
          onclick={handleSync}
          disabled={syncing}
          title={syncStatus.lastSync
            ? `Last synced: ${new Date(syncStatus.lastSync).toLocaleString()}${syncCheckReason.remoteChanged ? ' · Remote has changes' : ''}${syncCheckReason.localChanged ? ' · Local has changes' : ''}`
            : 'Never synced'}
        >
          {#if syncing}
            <span class="sync-spin">⟳</span> Syncing…
          {:else if needsSync}
            ↑↓ Sync needed
            <span class="needs-sync-dot"></span>
          {:else if syncStatus.lastSync}
            ☁️ {formatLastSync(syncStatus.lastSync)}
          {:else}
            ☁️ Sync
          {/if}
        </button>
        {#if syncStatus?.hasPendingConflicts}
          <button class="btn-outline resolve-btn" onclick={() => showConflictModal = true}>
            ⚠️ Resolve
          </button>
        {/if}
      {/if}
      <button class="btn-outline" onclick={handleExport} title="Export Vault">
        📤 Export
      </button>
      <button class="btn-outline" onclick={handleImport} title="Import Vault">
        📥 Import
      </button>
      <button class="btn-secondary" onclick={handleLock}>
        🔒 Lock Vault
      </button>
    </div>
  </header>

  <!-- Search and Filters -->
  <div class="dashboard-controls">
    <SearchBar
      bind:query={searchQuery}
      {categories}
      bind:selectedCategory
    />
    <button class="btn-primary" onclick={handleCreateEntry}>
      ➕ New Entry
    </button>
  </div>

  <!-- Main Content -->
  <main class="dashboard-main">
    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Loading...</p>
      </div>
    {:else}
       <EntryList
         entries={entries}
         onViewEntry={handleViewEntry}
         onEditEntry={handleEditEntry}
         groupByCategory={!searchQuery && !selectedCategory}
       />
    {/if}
  </main>

   <!-- Entry View Modal -->
   {#if showEntryView}
       <EntryView
         entry={viewingEntry}
         onClose={handleEntryViewClose}
         onEdit={() => {
           showEntryView = false;
           showEntryForm = true;
           editingEntry = viewingEntry;
           viewingEntry = null;
         }}
         onDelete={handleEntryDelete}
       />
   {/if}

    <!-- Entry Form Modal -->
    {#if showEntryForm}
        <EntryForm
          entry={editingEntry}
          categories={categories}
          onClose={handleEntryFormClose}
        />
    {/if}

    <!-- Settings Modal -->
    {#if showSettings}
        <Settings onClose={() => showSettings = false} />
    {/if}

    <!-- Conflict Resolution Modal -->
    {#if showConflictModal && pendingConflicts.length > 0}
        <ConflictResolutionModal
          conflicts={pendingConflicts}
          onResolved={handleConflictsResolved}
          onCancel={() => showConflictModal = false}
        />
    {/if}

    <!-- Re-auth Modal: master password changed on another device -->
    {#if showReauthModal}
      <div
        class="modal-overlay"
        role="presentation"
        onclick={() => { showReauthModal = false; reauthPassword = ""; reauthError = ""; }}
        onkeydown={(e) => e.key === 'Escape' && (showReauthModal = false)}
      >
        <div
          class="reauth-modal"
          role="dialog"
          aria-modal="true"
          aria-label="Master password changed"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
          tabindex="-1"
        >
          <div class="reauth-icon">🔑</div>
          <h3>Master Password Changed</h3>
          <p class="reauth-desc">
            The vault on Google Drive was re-encrypted with a new master password
            (changed on another device). Enter the new master password to sync
            and update this device.
          </p>

          <div class="reauth-field">
            <label for="reauth-pw">New Master Password</label>
            <input
              id="reauth-pw"
              type="password"
              bind:value={reauthPassword}
              placeholder="Enter the new master password"
              disabled={reauthLoading}
              onkeydown={(e) => e.key === 'Enter' && handleReauthAndSync()}
            />
          </div>

          {#if reauthError}
            <div class="reauth-error">{reauthError}</div>
          {/if}

          <div class="reauth-actions">
            <button
              class="btn-reauth-cancel"
              onclick={() => { showReauthModal = false; reauthPassword = ""; reauthError = ""; }}
              disabled={reauthLoading}
            >
              Cancel
            </button>
            <button
              class="btn-reauth-confirm"
              onclick={handleReauthAndSync}
              disabled={reauthLoading}
            >
              {#if reauthLoading}
                <span class="sync-spin">⟳</span> Syncing…
              {:else}
                Sync & Update Password
              {/if}
            </button>
          </div>
        </div>
      </div>
    {/if}
 </div>

<style>
  .dashboard {
    min-height: 100vh;
    background: #f8fafc;
    display: flex;
    flex-direction: column;
  }

  .dashboard-header {
    background: white;
    border-bottom: 1px solid #e2e8f0;
    padding: 16px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-left h1 {
    margin: 0;
    color: #1a202c;
    font-size: 1.5rem;
  }

  .vault-status {
    background: #edf2f7;
    color: #4a5568;
    padding: 4px 8px;
    border-radius: 12px;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .header-right {
    display: flex;
    gap: 12px;
  }

  .dashboard-controls {
    background: white;
    border-bottom: 1px solid #e2e8f0;
    padding: 16px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .dashboard-main {
    flex: 1;
    padding: 24px;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px;
    color: #718096;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e2e8f0;
    border-top: 3px solid #667eea;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .btn-primary {
    background: #667eea;
    color: white;
    border: none;
    padding: 10px 16px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-primary:hover {
    background: #5a67d8;
  }

  .btn-secondary {
    background: white;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 8px 14px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background: #f7fafc;
    border-color: #cbd5e0;
  }

  .btn-outline {
    background: transparent;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 8px 14px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 14px;
  }

  .btn-outline:hover {
    background: #f7fafc;
    border-color: #cbd5e0;
  }

  .btn-sync {
    background: white;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }

  .btn-sync:hover:not(:disabled) {
    background: #f7fafc;
    border-color: #cbd5e0;
  }

  .btn-sync:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-sync.needs-sync {
    background: #fffbeb;
    border-color: #f6ad55;
    color: #c05621;
    font-weight: 600;
  }

  .btn-sync.needs-sync:hover:not(:disabled) { background: #fef3c7; }

  .needs-sync-dot {
    width: 7px;
    height: 7px;
    background: #ed8936;
    border-radius: 50%;
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.8); }
  }

  .sync-spin {
    display: inline-block;
    animation: spin 0.8s linear infinite;
  }

  .resolve-btn { color: #c53030; border-color: #feb2b2; }

  /* Re-auth modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .reauth-modal {
    background: white;
    border-radius: 12px;
    padding: 32px;
    width: 100%;
    max-width: 420px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15);
    text-align: center;
  }

  .reauth-icon {
    font-size: 2.5rem;
    margin-bottom: 12px;
  }

  .reauth-modal h3 {
    margin: 0 0 10px;
    color: #1a202c;
    font-size: 1.2rem;
  }

  .reauth-desc {
    color: #4a5568;
    font-size: 0.875rem;
    line-height: 1.5;
    margin: 0 0 20px;
  }

  .reauth-field {
    text-align: left;
    margin-bottom: 16px;
  }

  .reauth-field label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: #4a5568;
    margin-bottom: 6px;
  }

  .reauth-field input {
    width: 100%;
    padding: 10px 12px;
    border: 1.5px solid #e2e8f0;
    border-radius: 6px;
    font-size: 14px;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }

  .reauth-field input:focus {
    outline: none;
    border-color: #667eea;
  }

  .reauth-field input:disabled {
    background: #f0f0f0;
    cursor: not-allowed;
  }

  .reauth-error {
    background: #fed7d7;
    color: #c53030;
    border: 1px solid #feb2b2;
    border-radius: 6px;
    padding: 10px 12px;
    font-size: 0.875rem;
    text-align: left;
    margin-bottom: 16px;
  }

  .reauth-actions {
    display: flex;
    gap: 10px;
    margin-top: 4px;
  }

  .btn-reauth-cancel {
    flex: 0 0 auto;
    padding: 10px 16px;
    background: #edf2f7;
    color: #4a5568;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-reauth-cancel:hover:not(:disabled) { background: #e2e8f0; }
  .btn-reauth-cancel:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-reauth-confirm {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 16px;
    background: #667eea;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-reauth-confirm:hover:not(:disabled) { background: #5a67d8; }
  .btn-reauth-confirm:disabled { opacity: 0.6; cursor: not-allowed; }

  @media (max-width: 768px) {
    .dashboard-header,
    .dashboard-controls {
      padding: 12px 16px;
    }

    .dashboard-main {
      padding: 16px;
    }

    .dashboard-controls {
      flex-direction: column;
      gap: 12px;
      align-items: stretch;
    }

    .header-left {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }
  }
</style>