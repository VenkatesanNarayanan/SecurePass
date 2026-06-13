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
  class="overlay"
  onclick={onCancel}
  onkeydown={(e) => e.key === "Escape" && onCancel()}
  role="presentation"
>
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Resolve Sync Conflicts"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2>Resolve Sync Conflicts</h2>
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
      <div class="error-banner">{error}</div>
    {/if}

    <div class="modal-footer">
      <button class="btn-secondary" onclick={onCancel} disabled={applying}>Cancel</button>
      <button class="btn-primary" onclick={applyResolutions} disabled={applying}>
        {#if applying}
          <span class="spinner"></span> Applying…
        {:else}
          Apply Resolutions
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    padding: 20px;
  }

  .modal {
    background: white;
    border-radius: 12px;
    width: 100%;
    max-width: 820px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.2);
  }

  .modal-header {
    padding: 24px 24px 16px;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h2 {
    margin: 0 0 6px;
    font-size: 1.25rem;
    color: #1a202c;
  }

  .subtitle {
    margin: 0;
    font-size: 0.875rem;
    color: #718096;
  }

  .conflicts-list {
    flex: 1;
    overflow-y: auto;
    padding: 16px 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .conflict-item {
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    overflow: hidden;
  }

  .conflict-header {
    background: #f8fafc;
    padding: 10px 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid #e2e8f0;
  }

  .conflict-number {
    font-size: 0.75rem;
    color: #718096;
    background: #edf2f7;
    padding: 2px 8px;
    border-radius: 10px;
  }

  .conflict-title {
    font-size: 0.95rem;
    color: #2d3748;
  }

  .versions {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .version-card {
    padding: 14px;
    cursor: pointer;
    border-left: 3px solid transparent;
    transition: background 0.15s, border-color 0.15s;
  }

  .version-card:first-child {
    border-right: 1px solid #e2e8f0;
  }

  .version-card.selected {
    background: #ebf4ff;
    border-left-color: #667eea;
  }

  .version-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  .version-label {
    font-weight: 600;
    font-size: 0.875rem;
    color: #2d3748;
    flex: 1;
  }

  .version-date {
    font-size: 0.75rem;
    color: #718096;
  }

  .version-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .version-category {
    font-size: 0.75rem;
    text-transform: capitalize;
    color: #667eea;
    font-weight: 500;
  }

  .version-notes {
    margin: 0;
    font-size: 0.8rem;
    color: #4a5568;
    line-height: 1.4;
    white-space: pre-wrap;
  }

  .version-field {
    display: flex;
    gap: 8px;
    font-size: 0.8rem;
  }

  .field-name {
    color: #718096;
    min-width: 80px;
    font-weight: 500;
  }

  .field-value {
    color: #2d3748;
    word-break: break-all;
  }

  .field-value.password {
    letter-spacing: 2px;
    color: #718096;
  }

  .error-banner {
    margin: 0 24px;
    padding: 10px 14px;
    background: #fed7d7;
    color: #c53030;
    border-radius: 6px;
    font-size: 0.875rem;
    border: 1px solid #feb2b2;
  }

  .modal-footer {
    padding: 16px 24px;
    border-top: 1px solid #e2e8f0;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn-primary {
    padding: 10px 18px;
    background: #667eea;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background 0.2s;
  }

  .btn-primary:hover:not(:disabled) { background: #5a67d8; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-secondary {
    padding: 10px 18px;
    background: #edf2f7;
    color: #4a5568;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn-secondary:hover:not(:disabled) { background: #e2e8f0; }
  .btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 600px) {
    .versions { grid-template-columns: 1fr; }
    .version-card:first-child { border-right: none; border-bottom: 1px solid #e2e8f0; }
  }
</style>
