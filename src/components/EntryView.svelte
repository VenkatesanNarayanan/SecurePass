<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";
  import { categoryMeta, initialOf, relativeTime } from "$lib/categories";
  import { copySecret } from "$lib/clipboard";
  import { passwordStrength } from "$lib/strength";

  interface Props {
    entry: any;
    onEdit: () => void;
    onDeleted: () => void;
    onCopied: (label: string, seconds: number) => void;
  }

  let { entry, onEdit, onDeleted, onCopied }: Props = $props();

  let revealed = $state<Record<string, boolean>>({});
  let justCopied = $state<string | null>(null);
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);
  let error = $state("");

  // Hide revealed secrets again whenever a different entry is shown.
  $effect(() => {
    void entry.id;
    revealed = {};
    justCopied = null;
    error = "";
  });

  const cat = $derived(categoryMeta(entry.category));

  const fields = $derived(
    (entry.fields ?? []).filter((f: any) => f.value && f.value.trim() !== "")
  );

  // Same name heuristics as before the redesign: mask anything that looks secret
  // even if it was saved as a plain text field.
  const SECRET_PATTERNS = [
    "password", "passphrase", "pin", "code", "key", "secret", "token",
    "pwd", "pass", "auth", "credential", "access",
  ];

  function isSecret(field: any): boolean {
    if (field.fieldType === "password") return true;
    const name = String(field.name).toLowerCase();
    return SECRET_PATTERNS.some((p) => name.includes(p));
  }

  function isUrl(value: string): boolean {
    return /^https?:\/\/\S+$/i.test(value.trim());
  }

  // Strength of the first real password field, if any.
  const strength = $derived.by(() => {
    const pw = fields.find((f: any) => f.fieldType === "password");
    return pw ? passwordStrength(pw.value) : null;
  });

  function display(field: any): string {
    if (isSecret(field) && !revealed[field.name]) {
      return "•".repeat(Math.min(Math.max(field.value.length, 8), 20));
    }
    return field.value;
  }

  async function copy(field: any) {
    try {
      const seconds = await copySecret(field.value);
      justCopied = field.name;
      setTimeout(() => { if (justCopied === field.name) justCopied = null; }, 1500);
      onCopied(field.name, seconds);
    } catch (e) {
      error = "Couldn't copy to the clipboard: " + (e instanceof Error ? e.message : String(e));
    }
  }

  async function openUrl(url: string) {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (e) {
      error = "Couldn't open the link: " + (e instanceof Error ? e.message : String(e));
    }
  }

  async function handleDelete() {
    deleting = true;
    try {
      await invoke("delete_entry", { request: { entryId: entry.id } });
      showDeleteConfirm = false;
      onDeleted();
    } catch (e) {
      error = "Couldn't delete the entry: " + (e instanceof Error ? e.message : String(e));
      showDeleteConfirm = false;
    } finally {
      deleting = false;
    }
  }
</script>

<div class="view">
  <header class="head">
    <span class="tile" style="background: {cat.bg}; color: {cat.fg}">{initialOf(entry.title)}</span>
    <div class="titles">
      <h2>{entry.title}</h2>
      <div class="meta">
        <span class="cat-chip">{cat.label}</span>
        <span>Edited {relativeTime(entry.updated_at)}</span>
      </div>
    </div>
    <div class="head-actions">
      <button class="btn btn-secondary" onclick={onEdit}><Icon name="edit" size={14} /> Edit</button>
      <button
        class="icon-btn bordered danger"
        onclick={() => (showDeleteConfirm = true)}
        disabled={deleting}
        aria-label="Delete entry"
        title="Delete entry"
      >
        <Icon name="trash" size={16} />
      </button>
    </div>
  </header>

  <div class="body">
    {#if error}
      <div class="msg msg-error">{error}</div>
    {/if}

    {#if fields.length > 0}
      <div class="fields">
        {#each fields as field (field.name)}
          {@const secret = isSecret(field)}
          <div class="field-row">
            <div class="field-text">
              <div class="field-label">{field.name}</div>
              {#if field.fieldType === "long_text" && (!secret || revealed[field.name])}
                <pre class="field-value long" class:mono={secret}>{field.value}</pre>
              {:else}
                <div class="field-value" class:mono={secret}>{display(field)}</div>
              {/if}
            </div>
            {#if secret}
              <button
                class="icon-btn"
                onclick={() => (revealed[field.name] = !revealed[field.name])}
                aria-label="{revealed[field.name] ? 'Hide' : 'Reveal'} {field.name}"
                title={revealed[field.name] ? "Hide" : "Reveal"}
              >
                <Icon name={revealed[field.name] ? "eye-off" : "eye"} size={17} />
              </button>
            {:else if isUrl(field.value)}
              <button class="icon-btn" onclick={() => openUrl(field.value.trim())} aria-label="Open {field.name} in browser" title="Open in browser">
                <Icon name="external" size={16} />
              </button>
            {/if}
            <button
              class="icon-btn"
              class:success={justCopied === field.name}
              onclick={() => copy(field)}
              aria-label="Copy {field.name}"
              title="Copy"
            >
              <Icon name={justCopied === field.name ? "check" : "copy"} size={16} />
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <p class="muted">This entry has no saved fields.</p>
    {/if}

    {#if strength && strength.score > 0}
      <div class="strength">
        <div class="strength-top">
          <span class="strength-label" style="color: {strength.color}">
            {strength.score <= 1 ? "Weak password — consider regenerating it" : `${strength.label} password`}
          </span>
        </div>
        <div class="bars" aria-hidden="true">
          {#each [1, 2, 3, 4] as i}
            <span style="background: {i <= strength.score ? strength.color : 'var(--strength-empty)'}"></span>
          {/each}
        </div>
      </div>
    {/if}

    {#if entry.notes}
      <section class="section">
        <div class="section-label">Notes</div>
        <p class="notes">{entry.notes}</p>
      </section>
    {/if}

    {#if entry.tags && entry.tags.filter((t: string) => t).length > 0}
      <div class="tags">
        {#each entry.tags.filter((t: string) => t) as tag}
          <span class="tag">#{tag}</span>
        {/each}
      </div>
    {/if}

    <div class="dates">
      Created {new Date(entry.created_at).toLocaleString()} · Updated {new Date(entry.updated_at).toLocaleString()}
    </div>
  </div>
</div>

{#if showDeleteConfirm}
  <ConfirmDialog
    title="Delete “{entry.title}”?"
    message="This removes the entry from this device and, on the next sync, from your other devices. This can't be undone."
    confirmText={deleting ? "Deleting…" : "Delete"}
    danger={true}
    onConfirm={handleDelete}
    onCancel={() => (showDeleteConfirm = false)}
  />
{/if}

<style>
  .view { display: flex; flex-direction: column; min-height: 100%; }

  .head {
    display: flex; align-items: flex-start; gap: 14px; flex-wrap: wrap;
    padding: 24px 28px 18px; border-bottom: 1px solid var(--border-subtle);
  }
  .tile {
    width: 52px; height: 52px; flex: none; border-radius: 13px;
    display: flex; align-items: center; justify-content: center;
    font-weight: 700; font-size: 22px;
  }
  .titles { flex: 1; min-width: 160px; }
  h2 { margin: 0; font-size: 22px; font-weight: 700; letter-spacing: -0.015em; overflow-wrap: anywhere; }
  .meta { display: flex; gap: 8px; align-items: center; margin-top: 4px; font-size: 12px; color: var(--text-muted); }
  .cat-chip { padding: 2px 8px; border-radius: 4px; background: var(--chip); color: var(--text-2); font-weight: 500; }
  .head-actions { display: flex; gap: 8px; }

  .body { padding: 20px 28px 32px; display: flex; flex-direction: column; gap: 20px; max-width: 680px; }

  .fields { border: 1px solid var(--border); border-radius: 12px; overflow: hidden; background: var(--surface); }
  .field-row { display: flex; align-items: center; gap: 4px; padding: 10px 10px 10px 16px; }
  .field-row + .field-row { border-top: 1px solid var(--border-subtle); }
  .field-text { flex: 1; min-width: 0; }
  .field-label { font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .field-value {
    font-size: 14px; margin-top: 2px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    user-select: text;
  }
  .field-value.long {
    white-space: pre-wrap; overflow-wrap: anywhere; margin: 2px 0 0; font-family: inherit;
    max-height: 220px; overflow-y: auto;
  }
  .field-value.mono, .field-value.long.mono { font-family: var(--font-mono); }

  .strength { display: flex; flex-direction: column; gap: 8px; }
  .strength-top { display: flex; justify-content: space-between; font-size: 12px; }
  .strength-label { font-weight: 600; }
  .bars { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; }
  .bars span { height: 4px; border-radius: 2px; }

  .section { display: flex; flex-direction: column; gap: 6px; }
  .section-label { font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .notes { margin: 0; color: var(--text-2); white-space: pre-wrap; overflow-wrap: anywhere; }

  .tags { display: flex; gap: 6px; flex-wrap: wrap; }
  .tag { font-size: 12px; padding: 3px 10px; border-radius: 999px; border: 1px solid var(--border-strong); color: var(--text-2); }

  .dates { font-size: 12px; color: var(--text-muted); }
  .muted { margin: 0; color: var(--text-muted); }
</style>
