<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  interface Props {
    entry: any;
    onClose: () => void;
    onEdit: () => void;
    onDelete: () => void;
  }

  let { entry, onClose, onEdit, onDelete }: Props = $props();

  let copyStatus = $state<Record<string, boolean>>({});
  let copyTimeouts = $state<Record<string, ReturnType<typeof setTimeout>>>({});
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);

  function getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      'web-app': '🌐',
      'ssh-server': '🖥️',
      'bank-financial': '🏦',
      'email': '📧',
      'wi-fi': '📶',
      'generic': '🔑'
    };
    return icons[category] || '🔑';
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString();
  }

  async function copyToClipboard(fieldName: string, value: string) {
    try {
      await invoke("copy_to_clipboard", { text: value });

      // Show success status
      copyStatus[fieldName] = true;

      // Clear status after 2 seconds
      setTimeout(() => {
        copyStatus[fieldName] = false;
      }, 2000);

      // Auto-clear clipboard after 30 seconds for security
      if (copyTimeouts[fieldName]) {
        clearTimeout(copyTimeouts[fieldName]);
      }

      copyTimeouts[fieldName] = setTimeout(async () => {
        try {
          await invoke("clear_clipboard");
        } catch (error) {
          console.error("Failed to clear clipboard:", error);
        }
      }, (parseInt(localStorage.getItem("sp_clipboard_timeout") || "30", 10)) * 1000);

    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
      // Could show error toast here
    }
  }

  function isPasswordField(field: any): boolean {
    if (field.fieldType === 'password') return true;
    const passwordPatterns = [
      'password', 'passphrase', 'pin', 'code', 'key', 'secret', 'token',
      'pwd', 'pass', 'auth', 'credential', 'access'
    ];
    return passwordPatterns.some(pattern => field.name.toLowerCase().includes(pattern));
  }

  function maskValue(value: string, field: any): string {
    if (isPasswordField(field) && value.length > 0) {
      return '•'.repeat(Math.min(value.length, 20));
    }
    return value;
  }

  async function handleDelete() {
    deleting = true;
    try {
      await invoke("delete_entry", { request: { entryId: entry.id } });
      onDelete();
    } catch (error) {
      console.error("Failed to delete entry:", error);
      // Could show error toast here
    } finally {
      deleting = false;
      showDeleteConfirm = false;
    }
  }
</script>

<!-- Modal Overlay -->
<div
  class="modal-overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="presentation"
>
  <div
    class="modal-content"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Entry Details"
    tabindex="-1"
  >
    <div class="modal-header">
      <div class="entry-info">
        <div class="entry-icon">
          {getCategoryIcon(entry.category)}
        </div>
        <div class="entry-details">
          <h2>{entry.title}</h2>
          <p class="category">{entry.category.replace('-', ' ')}</p>
        </div>
      </div>
      <div class="header-actions">
        <button class="delete-btn" onclick={() => showDeleteConfirm = true} disabled={deleting}>
          🗑️ Delete
        </button>
        <button class="edit-btn" onclick={onEdit}>
          ✏️ Edit
        </button>
        <button class="close-btn" onclick={onClose}>✕</button>
      </div>
    </div>

    <div class="entry-content">
      <!-- Basic Info -->
      <div class="info-section">
        {#if entry.notes}
          <div class="info-item">
            <span class="label">Notes</span>
            <div class="notes-content">{entry.notes}</div>
          </div>
        {/if}

        {#if entry.tags && entry.tags.length > 0}
          <div class="info-item">
            <span class="label">Tags</span>
            <div class="tags">
              {#each entry.tags as tag}
                <span class="tag">{tag}</span>
              {/each}
            </div>
          </div>
        {/if}

        <div class="info-item">
          <span class="label">Created</span>
          <span>{formatDate(entry.created_at)}</span>
        </div>

        <div class="info-item">
          <span class="label">Last Updated</span>
          <span>{formatDate(entry.updated_at)}</span>
        </div>
      </div>

      <!-- Fields -->
      {#if entry.fields.filter((field: any) => field.value && field.value.trim() !== '').length > 0}
        <div class="fields-section">
          <h3>Fields</h3>
          <div class="fields-grid">
            {#each entry.fields.filter((field: any) => field.value && field.value.trim() !== '') as field}
            <div class="field-item">
              <span class="label">{field.name}</span>
              <div class="field-value-container">
                <div class="field-value" class:password={isPasswordField(field)}>
                  {maskValue(field.value, field)}
                </div>
                <button
                  class="copy-btn"
                  onclick={() => copyToClipboard(field.name, field.value)}
                  title="Copy to clipboard (auto-clears in 30s)"
                >
                  {#if copyStatus[field.name]}
                    ✅
                  {:else}
                    📋
                  {/if}
                </button>
              </div>
              {#if copyStatus[field.name]}
                <div class="copy-feedback">
                  Copied! Will auto-clear in 30 seconds.
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
      {/if}
    </div>
  </div>
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <ConfirmDialog
    title="Delete Entry"
    message="Are you sure you want to delete this entry? This action cannot be undone."
    confirmText="Delete"
    danger={true}
    onConfirm={handleDelete}
    onCancel={() => showDeleteConfirm = false}
  />
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 20px;
  }

  .modal-content {
    background: white;
    border-radius: 12px;
    width: 100%;
    max-width: 700px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 24px;
    border-bottom: 1px solid #e2e8f0;
  }

  .entry-info {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .entry-icon {
    font-size: 2rem;
  }

  .entry-details h2 {
    margin: 0 0 4px 0;
    color: #1a202c;
    font-size: 1.5rem;
  }

  .category {
    color: #718096;
    font-size: 0.875rem;
    text-transform: capitalize;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .edit-btn {
    background: #edf2f7;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s;
  }

  .edit-btn:hover {
    background: #e2e8f0;
  }

  .delete-btn {
    background: #fed7d7;
    color: #c53030;
    border: 1px solid #feb2b2;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s;
  }

  .delete-btn:hover:not(:disabled) {
    background: #feb2b2;
  }

  .delete-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #a0aec0;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: #f7fafc;
    color: #4a5568;
  }

  .entry-content {
    padding: 24px;
  }

  .info-section {
    margin-bottom: 32px;
  }

  .info-item {
    display: flex;
    margin-bottom: 16px;
    align-items: flex-start;
  }

  .info-item .label {
    font-weight: 600;
    color: #4a5568;
    min-width: 100px;
    margin-right: 16px;
  }

  .notes-content {
    color: #2d3748;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag {
    background: #e6fffa;
    color: #234e52;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .fields-section h3 {
    margin: 0 0 16px 0;
    color: #2d3748;
    font-size: 1.125rem;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 16px;
  }

  .field-item {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 16px;
  }

  .field-item .label {
    display: block;
    font-weight: 600;
    color: #4a5568;
    margin-bottom: 8px;
  }

  .field-value-container {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .field-value {
    flex: 1;
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 8px 12px;
    font-family: monospace;
    font-size: 0.875rem;
    word-break: break-all;
    min-height: 36px;
    display: flex;
    align-items: center;
  }

  .field-value.password {
    font-family: 'Courier New', monospace;
    letter-spacing: 2px;
  }

  .copy-btn {
    background: #edf2f7;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 8px;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
    min-width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .copy-btn:hover {
    background: #e2e8f0;
  }

  .copy-feedback {
    margin-top: 8px;
    font-size: 0.75rem;
    color: #38a169;
    font-weight: 500;
  }

  @media (max-width: 768px) {
    .modal-content {
      margin: 10px;
      max-height: calc(100vh - 20px);
    }

    .modal-header {
      padding: 16px;
      flex-direction: column;
      gap: 16px;
      align-items: flex-start;
    }

    .header-actions {
      align-self: flex-end;
    }

    .entry-info {
      width: 100%;
    }

    .entry-content {
      padding: 16px;
    }

    .fields-grid {
      grid-template-columns: 1fr;
    }

    .info-item {
      flex-direction: column;
      gap: 4px;
    }

    .info-item .label {
      min-width: auto;
    }
  }
</style>