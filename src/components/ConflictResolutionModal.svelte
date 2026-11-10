<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    conflicts: ConflictInfo[];
    onResolved: () => void;
    onCancel: () => void;
  }

  interface ConflictInfo {
    localEntry: EntrySnapshot;
    remoteEntry: EntrySnapshot;
  }

  interface EntrySnapshot {
    id: string;
    title: string;
    category: string;
    notes: string;
    tags: string[];
    fields: { name: string; fieldType: string; value: string }[];
    updatedAt: string;
  }

  let { conflicts, onResolved, onCancel }: Props = $props();

  // Default every conflict to "local".
  let choices = $state<Record<string, "local" | "remote">>(
    Object.fromEntries(conflicts.map((c) => [c.localEntry.id, "local"]))
  );

  let applying = $state(false);
  let error = $state("");

  function formatDate(iso: string) {
    return new Date(iso).toLocaleString();
  }

  function categoryLabel(cat: string) {
    return cat.replace(/-/g, " ");
  }

  function maskIfPassword(field: { name: string; fieldType: string; value: string }) {
    if (field.fieldType === "password") return "•".repeat(Math.min(field.value.length, 16));
    return field.value;
  }

  async function applyResolutions() {
    applying = true;
    error = "";
    try {
      const resolutions = Object.entries(choices).map(([entryId, keep]) => ({
        entryId,
        keep,
      }));
      await invoke("resolve_conflicts", { resolutions });
      onResolved();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      applying = false;
    }
  }
</script>

<div
  class="modal-overlay"
  onclick={onCancel}
  onkeydown={(e) => e.key === "Escape" && onCancel()}
  role="presentation"
>
  <div
    class="modal xwide"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="conflicts-title"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2 id="conflicts-title">Resolve sync conflicts</h2>
      <p class="subtitle">
        {conflicts.length} {conflicts.length === 1 ? "entry was" : "entries were"} edited on
        multiple devices. Choose which version to keep for each.
      </p>
    </div>

    <div class="conflicts-list">
      {#each conflicts as conflict, i}
        {@const entryId = conflict.localEntry.id}
        <div class="conflict-item">
          <div class="conflict-header">
            <span class="conflict-number">Conflict {i + 1} of {conflicts.length}</span>
            <strong class="conflict-title">{conflict.localEntry.title}</strong>
          </div>

          <div class="versions">
            <!-- Local version -->
            <label
              class="version-card"
              class:selected={choices[entryId] === "local"}
              for="local-{entryId}"
            >
              <div class="version-header">
                <input
                  id="local-{entryId}"
                  type="radio"
                  name="conflict-{entryId}"
                  value="local"
                  bind:group={choices[entryId]}
                />
                <span class="version-label">This device</span>
                <span class="version-date">{formatDate(conflict.localEntry.updatedAt)}</span>
              </div>
              <div class="version-body">
                <span class="version-category">{categoryLabel(conflict.localEntry.category)}</span>
                {#if conflict.localEntry.notes}
                  <p class="version-notes">{conflict.localEntry.notes}</p>
                {/if}
                {#each conflict.localEntry.fields as field}
                  <div class="version-field">
                    <span class="field-name">{field.name}</span>
                    <span class="field-value" class:password={field.fieldType === "password"}>
                      {maskIfPassword(field)}
                    </span>
                  </div>
                {/each}
              </div>
            </label>

            <!-- Remote version -->
            <label
              class="version-card"
              class:selected={choices[entryId] === "remote"}
              for="remote-{entryId}"
            >
              <div class="version-header">
                <input
                  id="remote-{entryId}"
                  type="radio"
                  name="conflict-{entryId}"
                  value="remote"
                  bind:group={choices[entryId]}
                />
                <span class="version-label">Google Drive</span>
                <span class="version-date">{formatDate(conflict.remoteEntry.updatedAt)}</span>
              </div>
              <div class="version-body">
                <span class="version-category">{categoryLabel(conflict.remoteEntry.category)}</span>
                {#if conflict.remoteEntry.notes}
                  <p class="version-notes">{conflict.remoteEntry.notes}</p>
                {/if}
                {#each conflict.remoteEntry.fields as field}
                  <div class="version-field">
                    <span class="field-name">{field.name}</span>
                    <span class="field-value" class:password={field.fieldType === "password"}>
                      {maskIfPassword(field)}
                    </span>
                  </div>
                {/each}
              </div>
            </label>
          </div>
        </div>
      {/each}
    </div>

    {#if error}
      <div class="msg msg-error error-banner">{error}</div>
    {/if}

    <div class="modal-footer">
      <button class="btn btn-secondary" onclick={onCancel} disabled={applying}>Cancel</button>
      <button class="btn btn-primary" onclick={applyResolutions} disabled={applying}>
        {#if applying}
          <span class="spinner"></span> Applying…
        {:else}
          Keep selected versions
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-header { flex-direction: column; gap: 0; }

  .conflicts-list {
    flex: 1; min-height: 0; overflow-y: auto;
    padding: 16px 24px; display: flex; flex-direction: column; gap: 20px;
  }

  .conflict-item { display: flex; flex-direction: column; gap: 10px; }
  .conflict-header { display: flex; align-items: baseline; gap: 10px; }
  .conflict-number { font-size: 12px; color: var(--text-muted); }
  .conflict-title { font-size: 15px; }

  .versions { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }

  .version-card {
    display: flex; flex-direction: column; gap: 10px;
    padding: 12px 14px; border: 1px solid var(--border-strong); border-radius: 10px;
    background: var(--surface); cursor: pointer; transition: border-color 0.15s, background 0.15s;
  }
  .version-card:hover { background: var(--surface-hover); }
  .version-card.selected { border: 2px solid var(--accent); padding: 11px 13px; background: var(--accent-soft); }

  .version-header { display: flex; align-items: center; gap: 8px; }
  .version-header input { margin: 0; width: 16px; height: 16px; }
  .version-label { font-weight: 600; }
  .version-date { margin-left: auto; font-size: 12px; color: var(--text-muted); }

  .version-body { display: flex; flex-direction: column; gap: 6px; font-size: 13px; }
  .version-category {
    align-self: flex-start; padding: 2px 8px; border-radius: 4px;
    background: var(--chip); color: var(--text-2); font-size: 12px; text-transform: capitalize;
  }
  .version-notes { margin: 0; color: var(--text-2); white-space: pre-wrap; }
  .version-field { display: flex; justify-content: space-between; gap: 12px; }
  .field-name { color: var(--text-muted); flex: none; }
  .field-value { text-align: right; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .field-value.password { font-family: var(--font-mono); }

  .error-banner { margin: 0 24px 12px; }

  @media (max-width: 700px) {
    .versions { grid-template-columns: minmax(0, 1fr); }
  }
</style>
