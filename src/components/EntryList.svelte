<script lang="ts">
  import Icon from "./Icon.svelte";
  import { categoryMeta, initialOf, relativeTime } from "$lib/categories";

  interface Props {
    entries: any[];
    heading: string;
    selectedId: string | null;
    loading?: boolean;
    /** True when a search or category filter is active (changes the empty state). */
    filtered?: boolean;
    showCategory?: boolean;
    /** entry id → epoch ms of last use on this device (see $lib/recent). */
    recentUse?: Record<string, number>;
    onSelect: (entry: any) => void;
    onCreate: () => void;
  }

  let {
    entries,
    heading,
    selectedId,
    loading = false,
    filtered = false,
    showCategory = true,
    recentUse = {},
    onSelect,
    onCreate,
  }: Props = $props();

  type SortMode = "used" | "updated" | "az";
  const SORTS: { mode: SortMode; label: string }[] = [
    { mode: "used", label: "Recently used" },
    { mode: "updated", label: "Recently updated" },
    { mode: "az", label: "A–Z" },
  ];
  let sortMode = $state<SortMode>(readSort());

  function readSort(): SortMode {
    try {
      const v = localStorage.getItem("sp_list_sort");
      if (v === "az" || v === "updated" || v === "used") return v;
      if (v === "recent") return "updated"; // value saved before "Recently used" existed
    } catch { /* ignore */ }
    return "used";
  }

  function setSort(mode: SortMode) {
    sortMode = mode;
    try { localStorage.setItem("sp_list_sort", mode); } catch { /* ignore */ }
  }

  const updatedMs = (e: any) => new Date(e.updated_at).getTime();
  const byTitle = (a: any, b: any) => a.title.localeCompare(b.title, undefined, { sensitivity: "base" });

  const sorted = $derived(
    [...entries].sort((a, b) => {
      if (sortMode === "az") return byTitle(a, b);
      if (sortMode === "used") {
        // Used entries first (most recent use on top); never-used ones after,
        // by last update.
        const ua = recentUse[a.id] ?? 0;
        const ub = recentUse[b.id] ?? 0;
        if (ua !== ub) return ub - ua;
      }
      return updatedMs(b) - updatedMs(a) || byTitle(a, b);
    })
  );

  function metaTime(entry: any): string {
    if (sortMode === "used") {
      const used = recentUse[entry.id];
      return used ? `Used ${relativeTime(new Date(used).toISOString())}` : "Not used yet";
    }
    return relativeTime(entry.updated_at);
  }

  // Arrow-key navigation between rows.
  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    if (sorted.length === 0) return;
    event.preventDefault();
    const idx = sorted.findIndex((e) => e.id === selectedId);
    const next = event.key === "ArrowDown"
      ? Math.min(sorted.length - 1, idx + 1)
      : Math.max(0, idx === -1 ? 0 : idx - 1);
    onSelect(sorted[next]);
    const el = document.getElementById(`entry-row-${sorted[next].id}`);
    el?.focus();
    el?.scrollIntoView({ block: "nearest" });
  }
</script>

<section class="list-pane" aria-label="Entries">
  <header class="list-header">
    <div class="titles">
      <h1>{heading}</h1>
      <div class="sub">{entries.length} {entries.length === 1 ? "item" : "items"}</div>
    </div>
    <button class="btn btn-primary" onclick={onCreate} title="New entry (Ctrl+N)">
      <Icon name="plus" size={14} strokeWidth={2.4} /> New
    </button>
  </header>

  <div class="sort" role="group" aria-label="Sort entries">
    {#each SORTS as s (s.mode)}
      <button class="chip" class:on={sortMode === s.mode} aria-pressed={sortMode === s.mode} onclick={() => setSort(s.mode)}>{s.label}</button>
    {/each}
  </div>

  <div class="rows" role="listbox" aria-label={heading} tabindex="-1" onkeydown={handleKeydown}>
    {#if loading && entries.length === 0}
      <div class="empty"><span class="spinner"></span></div>
    {:else if entries.length === 0}
      <div class="empty">
        {#if filtered}
          <Icon name="search" size={22} />
          <p class="empty-title">No matching entries</p>
          <p>Try a different search or category.</p>
        {:else}
          <Icon name="key" size={22} />
          <p class="empty-title">Your vault is empty</p>
          <p>Add your first password to get started.</p>
          <button class="btn btn-secondary" onclick={onCreate}><Icon name="plus" size={14} /> New entry</button>
        {/if}
      </div>
    {:else}
      {#each sorted as entry (entry.id)}
        {@const cat = categoryMeta(entry.category)}
        <button
          id="entry-row-{entry.id}"
          class="row"
          class:selected={entry.id === selectedId}
          role="option"
          aria-selected={entry.id === selectedId}
          onclick={() => onSelect(entry)}
        >
          <span class="tile" style="background: {cat.bg}; color: {cat.fg}">{initialOf(entry.title)}</span>
          <span class="text">
            <span class="title">{entry.title}</span>
            <span class="meta">
              {showCategory ? `${cat.label} · ${metaTime(entry)}` : metaTime(entry)}
            </span>
          </span>
        </button>
      {/each}
    {/if}
  </div>
</section>

<style>
  .list-pane {
    width: 300px;
    flex: none;
    height: 100%;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    background: var(--bg);
    min-height: 0;
  }

  .list-header {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    padding: 18px 16px 10px;
  }
  .titles { min-width: 0; }
  h1 {
    margin: 0; font-size: 17px; font-weight: 700; letter-spacing: -0.01em;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .sub { font-size: 12px; color: var(--text-muted); }

  .sort { display: flex; flex-wrap: wrap; gap: 6px; padding: 0 16px 10px; }
  .chip {
    font-size: 12px; padding: 4px 10px; border-radius: 999px; border: 0; cursor: pointer;
    background: var(--chip); color: var(--text-2);
  }
  .chip.on { background: var(--text); color: var(--bg); }

  .rows {
    flex: 1; min-height: 0; overflow-y: auto;
    display: flex; flex-direction: column; gap: 2px; padding: 0 8px 12px;
    outline: none;
  }

  .row {
    display: flex; align-items: center; gap: 12px; width: 100%;
    min-height: 56px; padding: 8px 10px;
    border: 0; border-radius: 10px; background: transparent;
    color: var(--text); text-align: left; cursor: pointer;
  }
  .row:hover { background: var(--surface-hover); }
  .row.selected { background: var(--accent-soft); }

  .tile {
    width: 36px; height: 36px; flex: none; border-radius: 9px;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 14px;
  }
  .text { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .title, .meta { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .title { font-weight: 600; font-size: 14px; }
  .meta { font-size: 12px; color: var(--text-muted); }

  .empty {
    display: flex; flex-direction: column; align-items: center; gap: 6px;
    padding: 48px 16px; text-align: center; color: var(--text-muted);
  }
  .empty p { margin: 0; font-size: 13px; }
  .empty .empty-title { color: var(--text); font-weight: 600; font-size: 14px; margin-top: 6px; }
  .empty .btn { margin-top: 10px; }

  @media (max-width: 1000px) {
    .list-pane { width: 260px; }
  }
</style>
