<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";
  import EntryList from "./EntryList.svelte";
  import EntryForm from "./EntryForm.svelte";
  import EntryView from "./EntryView.svelte";
  import SearchBar from "./SearchBar.svelte";
  import Settings from "./Settings.svelte";

  const dispatch = createEventDispatcher();

  // Props
  interface Props {
    vaultStatus: any;
    onLock: () => void;
  }

  let { vaultStatus, onLock }: Props = $props();

  // State
  let entries = $state([]);
  let categories = $state([]);
  let tags = $state([]);
  let searchQuery = $state("");
  let selectedCategory = $state("");
  let showEntryForm = $state(false);
  let showEntryView = $state(false);
  let showSettings = $state(false);
  let editingEntry = $state(null);
  let viewingEntry = $state(null);
  let loading = $state(false);

  // Load data on mount
  async function loadData() {
    loading = true;
    try {
      [entries, categories, tags] = await Promise.all([
        invoke("get_entries"),
        invoke("get_categories"),
        invoke("get_tags")
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
      entries = await invoke("search_entries", {
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
      const fullEntry = await invoke("get_entry", { request: { entryId: entry.id } });
      viewingEntry = fullEntry;
      showEntryView = true;
    } catch (error) {
      console.error("Failed to load entry:", error);
    }
  }

  // Handle entry editing
  async function handleEditEntry(entry: any) {
    try {
      const fullEntry = await invoke("get_entry", { request: { entryId: entry.id } });
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

  // Handle import vault
  async function handleImport() {
    try {
      // Use Tauri's dialog API to select file
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({
        title: "Import SecurePass Vault",
        filters: [{
          name: "SecurePass Vault",
          extensions: ["spvault"]
        }],
        multiple: false
      });

      if (filePath) {
        // Prompt for master password
        const masterPassword = prompt("Enter the master password for the vault you want to import:");
        if (!masterPassword) return;

        await invoke("import_vault", {
          request: {
            filePath,
            masterPassword
          }
        });

        // Refresh data after import
        await loadData();
        alert("Vault imported successfully!");
      }
    } catch (error) {
      console.error("Failed to import vault:", error);
      alert("Failed to import vault: " + error);
    }
  }

  // Initialize
  loadData();

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