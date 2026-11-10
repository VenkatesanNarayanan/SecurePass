<script lang="ts">
  import Icon from "./Icon.svelte";
  import { categoryMeta, sortCategories } from "$lib/categories";

  interface SyncStatus {
    connected: boolean;
    lastSync: string | null;
    hasPendingConflicts: boolean;
  }

  interface Props {
    query: string;
    selectedCategory: string;
    /** Entry count per category key, over the whole vault. */
    categoryCounts: Record<string, number>;
    totalCount: number;
    syncStatus: SyncStatus | null;
    syncing: boolean;
    needsSync: boolean;
    syncTitle: string;
    syncError: string;
    searchInput?: HTMLInputElement | null;
    onSync: () => void;
    onResolveConflicts: () => void;
    onOpenSettings: () => void;
    onImport: () => void;
    onExport: () => void;
    onLock: () => void;
  }

  let {
    query = $bindable(""),
    selectedCategory = $bindable(""),
    categoryCounts,
    totalCount,
    syncStatus,
    syncing,
    needsSync,
    syncTitle,
    syncError,
    searchInput = $bindable(null),
    onSync,
    onResolveConflicts,
    onOpenSettings,
    onImport,
    onExport,
    onLock,
  }: Props = $props();

  const categories = $derived(sortCategories(Object.keys(categoryCounts)).map(categoryMeta));

  function formatLastSync(iso: string | null) {
    if (!iso) return "Never synced";
    const mins = Math.floor((Date.now() - new Date(iso).getTime()) / 60000);
    if (mins < 1) return "Synced just now";
    if (mins < 60) return `Synced ${mins} min ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `Synced ${hrs} h ago`;
    return `Synced ${new Date(iso).toLocaleDateString()}`;
  }

  const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);
</script>

<nav class="sidebar" aria-label="Vault navigation">
  <div class="brand">
    <span class="brand-mark"><Icon name="brand" size={20} /></span>
    <span class="brand-name">SecurePass</span>
  </div>

  <div class="search">
    <Icon name="search" size={15} />
    <label for="vault-search" class="sr-only">Search vault</label>
    <input
      id="vault-search"
      type="search"
      placeholder="Search vault"
      autocomplete="off"
      spellcheck="false"
      bind:value={query}
      bind:this={searchInput}
    />
    {#if query}
      <button class="clear" onclick={() => (query = "")} aria-label="Clear search">
        <Icon name="close" size={14} />
      </button>
    {:else}
      <span class="kbd">{isMac ? "⌘K" : "Ctrl K"}</span>
    {/if}
  </div>

  <div class="group">
    <button class="nav-item" class:active={selectedCategory === ""} onclick={() => (selectedCategory = "")}>
      <Icon name="list" size={15} />
      <span class="nav-label">All items</span>
      <span class="count">{totalCount}</span>
    </button>
  </div>

  {#if categories.length > 0}
    <div class="group">
      <div class="group-title">Categories</div>
      {#each categories as cat (cat.key)}
        <button
          class="nav-item"
          class:active={selectedCategory === cat.key}
          onclick={() => (selectedCategory = selectedCategory === cat.key ? "" : cat.key)}
        >
          <span class="dot" style="background: {cat.dot}"></span>
          <span class="nav-label">{cat.label}</span>
          <span class="count">{categoryCounts[cat.key]}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="bottom">
    {#if syncStatus?.connected}
      <div class="sync-card" class:warn={(needsSync && !syncing) || syncStatus.hasPendingConflicts} title={syncTitle}>
        <div class="sync-head">
          {#if syncing}
            <span class="spinner sync-spinner"></span>
            <span class="sync-state">Syncing…</span>
          {:else if syncStatus.hasPendingConflicts}
            <span class="sync-dot warn"></span>
            <span class="sync-state">Conflicts to resolve</span>
          {:else if needsSync}
            <span class="sync-dot warn"></span>
            <span class="sync-state">Sync needed</span>
          {:else}
            <span class="sync-dot ok"></span>
            <span class="sync-state">{formatLastSync(syncStatus.lastSync)}</span>
          {/if}
        </div>
        <div class="sync-sub">
          {#if needsSync && !syncing && syncStatus.lastSync}
            {formatLastSync(syncStatus.lastSync).replace("Synced", "Last synced")}
          {:else}
            Google Drive · encrypted copy
          {/if}
        </div>
        {#if syncError}
          <div class="sync-error">{syncError}</div>
        {/if}
        {#if syncStatus.hasPendingConflicts}
          <button class="btn btn-secondary sync-btn" onclick={onResolveConflicts}>Resolve conflicts</button>
        {:else}
          <button class="btn btn-secondary sync-btn" onclick={onSync} disabled={syncing}>Sync now</button>
        {/if}
      </div>
    {:else}
      <div class="sync-card">
        <div class="sync-head">
          <Icon name="cloud" size={15} />
          <span class="sync-state">Not syncing</span>
        </div>
        <div class="sync-sub">Keep an encrypted copy in your Google Drive.</div>
        <button class="btn btn-secondary sync-btn" onclick={onOpenSettings}>Connect Google Drive</button>
      </div>
    {/if}

    <div class="actions">
      <button class="btn btn-ghost" onclick={onImport} title="Import a .spvault backup or PasswordLocker JSON">
        <Icon name="import" size={15} /> Import
      </button>
      <button class="btn btn-ghost" onclick={onExport} title="Export an encrypted .spvault backup">
        <Icon name="export" size={15} /> Export
      </button>
    </div>
    <div class="actions">
      <button class="btn btn-ghost" onclick={onOpenSettings}>
        <Icon name="settings" size={15} /> Settings
      </button>
      <button class="btn btn-ghost" onclick={onLock} title="Lock vault ({isMac ? '⌘' : 'Ctrl+'}L)">
        <Icon name="lock" size={15} /> Lock
      </button>
    </div>
  </div>
</nav>

<style>
  .sidebar {
    width: 232px;
    flex: none;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .brand { display: flex; align-items: center; gap: 10px; padding: 2px 8px; }
  .brand-mark {
    width: 30px; height: 30px; border-radius: 8px;
    /* Same look as the app icon in both themes: dark tile, light-blue shield. */
    background: linear-gradient(#272b34, #121418);
    color: #c9d5ff;
    --brand-keyhole: #121418;
    display: flex; align-items: center; justify-content: center;
  }
  .brand-name { font-weight: 700; font-size: 15px; letter-spacing: -0.01em; }

  .search {
    display: flex; align-items: center; gap: 8px;
    min-height: 38px; padding: 0 8px 0 10px;
    border: 1px solid var(--border-strong); border-radius: 8px;
    background: var(--surface); color: var(--text-muted);
  }
  .search:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px var(--focus-ring); }
  .search input {
    flex: 1; min-width: 0; border: 0; outline: none; background: transparent;
    color: var(--text); font-size: 13px; height: 36px;
  }
  .search input::-webkit-search-cancel-button { display: none; }
  .search input::placeholder { color: var(--text-muted); }
  .search .clear {
    border: 0; background: transparent; color: var(--text-muted); cursor: pointer;
    width: 24px; height: 24px; border-radius: 6px; display: flex; align-items: center; justify-content: center;
  }
  .search .clear:hover { background: var(--surface-hover); color: var(--text); }

  .group { display: flex; flex-direction: column; gap: 2px; }
  .group-title {
    font-size: 11px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase;
    color: var(--text-muted); padding: 0 10px 6px;
  }

  .nav-item {
    display: flex; align-items: center; gap: 10px;
    min-height: 34px; padding: 0 10px;
    border: 0; border-radius: 7px; background: transparent;
    color: var(--text); font-size: 13px; text-align: left; cursor: pointer;
  }
  .nav-item:hover { background: var(--surface-hover); }
  .nav-item.active { background: var(--nav-active); font-weight: 600; }
  .nav-item :global(svg) { color: var(--text-muted); }
  .nav-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .count { font-size: 12px; color: var(--text-muted); font-weight: 400; }
  .dot { width: 8px; height: 8px; border-radius: 2px; flex: none; margin: 0 3px; }

  .bottom { margin-top: auto; display: flex; flex-direction: column; gap: 6px; }

  .sync-card {
    background: var(--surface); border: 1px solid var(--border); border-radius: 10px;
    padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; margin-bottom: 4px;
  }
  .sync-card.warn { border-color: var(--warning-border); }
  .sync-head { display: flex; align-items: center; gap: 8px; color: var(--text); }
  .sync-state { font-weight: 600; font-size: 13px; }
  .sync-card.warn .sync-state { color: var(--warning-text); }
  .sync-dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .sync-dot.ok { background: var(--success); }
  .sync-dot.warn { background: var(--warning); }
  .sync-spinner { width: 12px; height: 12px; color: var(--accent); }
  .sync-sub { font-size: 12px; color: var(--text-muted); }
  .sync-error { font-size: 12px; color: var(--danger-text); }
  .sync-btn { min-height: 30px; font-size: 12px; margin-top: 2px; }

  .actions { display: flex; gap: 4px; }
  .actions .btn { flex: 1; justify-content: flex-start; padding: 0 10px; min-height: 34px; }
</style>
