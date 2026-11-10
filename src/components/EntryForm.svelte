<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";
  import PasswordGenerator from "./PasswordGenerator.svelte";
  import { categoryMeta } from "$lib/categories";

  // Create / edit form shown in the detail pane. Dashboard re-mounts it
  // (via {#key}) for each entry, so the initial values below are read once.
  interface Props {
    entry?: any;
    onSaved: (entryId: string) => void;
    onCancel: () => void;
  }

  let { entry = null, onSaved, onCancel }: Props = $props();

  interface TemplateField {
    name: string;
    type: "text" | "password" | "long_text";
    required: boolean;
  }

  interface CustomField {
    id: number;
    name: string;
    type: string;
    value: string;
  }

  const CATEGORY_TEMPLATES: Record<string, TemplateField[]> = {
    "web-app": [
      { name: "URL", type: "text", required: false },
      { name: "Username", type: "text", required: false },
      { name: "Password", type: "password", required: true },
      { name: "Recovery Email", type: "text", required: false },
    ],
    "ssh-server": [
      { name: "Hostname / IP", type: "text", required: true },
      { name: "Port", type: "text", required: false },
      { name: "Username", type: "text", required: true },
      { name: "Password", type: "password", required: false },
      { name: "Private Key", type: "long_text", required: false },
      { name: "Passphrase", type: "password", required: false },
    ],
    "bank-financial": [
      { name: "Bank Name", type: "text", required: true },
      { name: "Username / Customer ID", type: "text", required: false },
      { name: "Password / PIN", type: "password", required: true },
      { name: "Account Number", type: "text", required: false },
      { name: "Customer Support Number", type: "text", required: false },
    ],
    "email": [
      { name: "Email Address", type: "text", required: true },
      { name: "Password", type: "password", required: true },
      { name: "IMAP Server", type: "text", required: false },
      { name: "SMTP Server", type: "text", required: false },
    ],
    "wi-fi": [
      { name: "SSID", type: "text", required: true },
      { name: "Password", type: "password", required: true },
      { name: "Network Type", type: "text", required: false },
    ],
    "generic": [
      { name: "Username", type: "text", required: false },
      { name: "Password", type: "password", required: true },
      { name: "Notes", type: "long_text", required: false },
    ],
  };

  const CATEGORY_KEYS = Object.keys(CATEGORY_TEMPLATES);

  function templateFor(category: string): TemplateField[] {
    return CATEGORY_TEMPLATES[category] ?? CATEGORY_TEMPLATES["generic"];
  }

  // ── Initial state (read once) ──────────────────────────────────────────────
  const initialCategory: string = entry?.category ?? "web-app";
  const initialTemplateNames = templateFor(initialCategory).map((f) => f.name);
  let nextId = 1;

  let selectedCategory = $state(initialCategory);
  let title = $state<string>(entry?.title ?? "");
  let tagsText = $state<string>((entry?.tags ?? []).filter((t: string) => t).join(", "));
  let notes = $state<string>(entry?.notes ?? "");
  let fieldValues = $state<Record<string, string>>(
    Object.fromEntries((entry?.fields ?? []).map((f: any) => [f.name, f.value]))
  );
  // Fields that are not part of the entry's template are edited as custom fields.
  let customFields = $state<CustomField[]>(
    (entry?.fields ?? [])
      .filter((f: any) => !initialTemplateNames.includes(f.name))
      .map((f: any) => ({ id: nextId++, name: f.name, type: f.fieldType, value: f.value }))
  );

  let loading = $state(false);
  let error = $state("");
  let generatorFor = $state<string | null>(null);
  let revealed = $state<Record<string, boolean>>({});

  const template = $derived(templateFor(selectedCategory));

  // Categories offered as chips: the built-in ones, plus the entry's own
  // category if it is a custom one (so editing never silently changes it).
  const categoryChoices = $derived(
    CATEGORY_KEYS.includes(initialCategory) ? CATEGORY_KEYS : [...CATEGORY_KEYS, initialCategory]
  );

  async function addCustomField() {
    const id = nextId++;
    customFields = [...customFields, { id, name: "", type: "text", value: "" }];
    await tick();
    document.getElementById(`custom-${id}-name`)?.focus();
  }

  function removeCustomField(id: number) {
    customFields = customFields.filter((f) => f.id !== id);
    if (generatorFor === `custom-${id}`) generatorFor = null;
  }

  // The field is shown in plain text while its generator is open, so you can
  // see what it produces, and masked again when the generator closes.
  function openGenerator(key: string) {
    if (generatorFor && generatorFor !== key) revealed[generatorFor] = false;
    generatorFor = key;
    revealed[key] = true;
  }

  function closeGenerator() {
    if (generatorFor) revealed[generatorFor] = false;
    generatorFor = null;
  }

  function toggleGenerator(key: string) {
    if (generatorFor === key) closeGenerator();
    else openGenerator(key);
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    error = "";

    if (!title.trim()) {
      error = "Title is required.";
      return;
    }

    const fields: { name: string; fieldType: string; value: string }[] = [];
    for (const tf of template) {
      const value = fieldValues[tf.name] ?? "";
      if (tf.required && !value.trim()) {
        error = `${tf.name} is required.`;
        return;
      }
      fields.push({ name: tf.name, fieldType: tf.type, value });
    }
    for (const cf of customFields) {
      if (!cf.name.trim()) continue;
      fields.push({ name: cf.name.trim(), fieldType: cf.type, value: cf.value });
    }

    const entryData = {
      category: selectedCategory,
      title: title.trim(),
      fields,
      tags: tagsText.split(",").map((t) => t.trim()).filter(Boolean),
      notes: notes.trim(),
    };

    loading = true;
    try {
      if (entry) {
        await invoke("update_entry", { request: { entryId: entry.id, ...entryData } });
        onSaved(entry.id);
      } else {
        const id = await invoke<string>("create_entry", { request: entryData });
        onSaved(id);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }
</script>

<form class="form" onsubmit={handleSubmit} novalidate>
  <header class="head">
    <h2>{entry ? "Edit entry" : "New entry"}</h2>
    <div class="head-actions">
      <button type="button" class="btn btn-secondary" onclick={onCancel} disabled={loading}>Cancel</button>
      <button type="submit" class="btn btn-primary" disabled={loading}>
        {#if loading}<span class="spinner"></span>{/if}
        {entry ? "Save changes" : "Save"}
      </button>
    </div>
  </header>

  <div class="body">
    {#if error}
      <div class="msg msg-error" role="alert">{error}</div>
    {/if}

    <fieldset class="cats">
      <legend class="label">Category</legend>
      <div class="chips">
        {#each categoryChoices as key (key)}
          {@const meta = categoryMeta(key)}
          <button
            type="button"
            class="cat-chip"
            class:on={selectedCategory === key}
            aria-pressed={selectedCategory === key}
            onclick={() => (selectedCategory = key)}
          >
            <Icon name={meta.icon} size={14} />
            {meta.label}
          </button>
        {/each}
      </div>
    </fieldset>

    <div class="field">
      <label class="label" for="entry-title">Title <span class="required">*</span></label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="entry-title" class="input" type="text" bind:value={title} placeholder="e.g. GitHub" autofocus={!entry} />
    </div>

    <div class="grid">
      {#each template as tf, i (selectedCategory + tf.name)}
        {@const id = `tf-${i}`}
        <div class="field" class:span={tf.type !== "text"}>
          <label class="label" for={id}>
            {tf.name}
            {#if tf.required}<span class="required">*</span>{:else}<span class="optional">· optional</span>{/if}
          </label>
          {#if tf.type === "password"}
            <div class="pw-wrap" class:open={generatorFor === tf.name}>
              <div class="input-group">
                <input
                  {id}
                  class="mono"
                  type={revealed[tf.name] ? "text" : "password"}
                  autocomplete="off"
                  spellcheck="false"
                  bind:value={fieldValues[tf.name]}
                />
                <button
                  type="button"
                  class="icon-btn"
                  onclick={() => (revealed[tf.name] = !revealed[tf.name])}
                  aria-label="{revealed[tf.name] ? 'Hide' : 'Show'} {tf.name}"
                  title={revealed[tf.name] ? "Hide" : "Show"}
                >
                  <Icon name={revealed[tf.name] ? "eye-off" : "eye"} size={16} />
                </button>
                <button
                  type="button"
                  class="icon-btn"
                  class:active={generatorFor === tf.name}
                  onclick={() => toggleGenerator(tf.name)}
                  aria-label="Generate {tf.name}"
                  aria-expanded={generatorFor === tf.name}
                  title="Generate a strong password"
                >
                  <Icon name="refresh" size={16} />
                </button>
              </div>
              {#if generatorFor === tf.name}
                <PasswordGenerator
                  onGenerate={(pw) => (fieldValues[tf.name] = pw)}
                  onClose={closeGenerator}
                />
              {/if}
            </div>
          {:else if tf.type === "long_text"}
            <textarea {id} class="input" class:mono={tf.name === "Private Key"} rows="4" bind:value={fieldValues[tf.name]}></textarea>
          {:else}
            <input {id} class="input" type="text" bind:value={fieldValues[tf.name]} />
          {/if}
        </div>
      {/each}
    </div>

    <section class="custom">
      <span class="label">Custom fields</span>

      {#each customFields as cf (cf.id)}
        {@const key = `custom-${cf.id}`}
        <div class="custom-row">
          <div class="custom-top">
            <label class="sr-only" for="{key}-name">Field name</label>
            <input id="{key}-name" class="input name" type="text" placeholder="Field name" bind:value={cf.name} />
            <label class="sr-only" for="{key}-type">Field type</label>
            <select id="{key}-type" class="input type" bind:value={cf.type}>
              <option value="text">Text</option>
              <option value="password">Secret</option>
              <option value="long_text">Long text</option>
            </select>
            <button type="button" class="icon-btn danger" onclick={() => removeCustomField(cf.id)} aria-label="Remove field {cf.name}" title="Remove field">
              <Icon name="trash" size={16} />
            </button>
          </div>
          <label class="sr-only" for="{key}-value">Value</label>
          {#if cf.type === "password"}
            <div class="pw-wrap" class:open={generatorFor === key}>
              <div class="input-group">
                <input
                  id="{key}-value"
                  class="mono"
                  type={revealed[key] ? "text" : "password"}
                  placeholder="Value"
                  autocomplete="off"
                  spellcheck="false"
                  bind:value={cf.value}
                />
                <button type="button" class="icon-btn" onclick={() => (revealed[key] = !revealed[key])} aria-label="{revealed[key] ? 'Hide' : 'Show'} value">
                  <Icon name={revealed[key] ? "eye-off" : "eye"} size={16} />
                </button>
                <button type="button" class="icon-btn" class:active={generatorFor === key} onclick={() => toggleGenerator(key)} aria-label="Generate value" aria-expanded={generatorFor === key}>
                  <Icon name="refresh" size={16} />
                </button>
              </div>
              {#if generatorFor === key}
                <PasswordGenerator onGenerate={(pw) => (cf.value = pw)} onClose={closeGenerator} />
              {/if}
            </div>
          {:else if cf.type === "long_text"}
            <textarea id="{key}-value" class="input" rows="3" placeholder="Value" bind:value={cf.value}></textarea>
          {:else}
            <input id="{key}-value" class="input" type="text" placeholder="Value" bind:value={cf.value} />
          {/if}
        </div>
      {/each}

      <!-- Below the list so it stays next to the field you just added. -->
      <button type="button" class="btn btn-secondary add" onclick={addCustomField}>
        <Icon name="plus" size={14} /> Add field
      </button>
    </section>

    <div class="grid">
      <div class="field">
        <label class="label" for="entry-tags">Tags <span class="optional">· comma-separated</span></label>
        <input id="entry-tags" class="input" type="text" placeholder="work, personal" bind:value={tagsText} />
      </div>
      <div class="field">
        <label class="label" for="entry-notes">Notes</label>
        <textarea id="entry-notes" class="input" rows="2" bind:value={notes}></textarea>
      </div>
    </div>
  </div>
</form>

<style>
  .form { display: flex; flex-direction: column; min-height: 100%; }

  .head {
    position: sticky; top: 0; z-index: 2;
    display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap;
    padding: 18px 28px; background: var(--bg); border-bottom: 1px solid var(--border-subtle);
  }
  h2 { margin: 0; font-size: 20px; font-weight: 700; letter-spacing: -0.015em; }
  .head-actions { display: flex; gap: 8px; }

  .body { padding: 22px 28px 40px; display: flex; flex-direction: column; gap: 20px; max-width: 760px; }

  .cats { border: 0; margin: 0; padding: 0; }
  .cats legend { padding: 0; margin-bottom: 8px; }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .cat-chip {
    display: inline-flex; align-items: center; gap: 6px;
    min-height: 36px; padding: 0 12px; border-radius: 8px; cursor: pointer;
    border: 1px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 13px;
  }
  .cat-chip:hover { background: var(--surface-hover); }
  .cat-chip.on {
    border: 2px solid var(--accent); padding: 0 11px;
    background: var(--accent-soft); color: var(--text); font-weight: 600;
  }

  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
  .grid .span { grid-column: 1 / -1; }

  .pw-wrap {
    border: 1px solid var(--border-strong); border-radius: 10px; overflow: hidden; background: var(--surface);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .pw-wrap:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px var(--focus-ring); }
  .pw-wrap .input-group { border: 0; border-radius: 0; box-shadow: none; }
  .pw-wrap .input-group input { font-family: var(--font-mono); font-size: 15px; }
  .icon-btn.active { background: var(--accent-soft); color: var(--accent-text); }

  .custom { display: flex; flex-direction: column; gap: 10px; }
  .add { align-self: flex-start; min-height: 34px; border-style: dashed; }
  .custom-row {
    display: flex; flex-direction: column; gap: 8px;
    padding: 12px; border: 1px dashed var(--border-strong); border-radius: 10px;
  }
  .custom-top { display: flex; gap: 8px; align-items: center; }
  .custom-top .name { flex: 1; }
  .custom-top .type { width: 130px; flex: none; }

  @media (max-width: 1100px) {
    .grid { grid-template-columns: minmax(0, 1fr); }
  }
</style>
