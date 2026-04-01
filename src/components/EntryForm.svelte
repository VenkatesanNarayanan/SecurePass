<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import PasswordGenerator from "./PasswordGenerator.svelte";

  interface Props {
    entry?: any;
    categories: string[];
    onClose: () => void;
  }

  let { entry = null, categories, onClose }: Props = $props();

  interface CustomField {
    name: string;
    type: string;
    value: string;
    id: number;
  }

  // Form state
  let selectedCategory = $state(entry?.category || "web-app");
  let title = $state(entry?.title || "");
  let tags = $state(entry?.tags || []);
  let notes = $state(entry?.notes || "");
  let customFields = $state<CustomField[]>([]);
  let loading = $state(false);
  let error = $state("");
  let showPasswordGenerator = $state(false);
  let passwordGeneratorTarget = $state<string | null>(null); // Will store which field is being edited

  // Category templates
  const CATEGORY_TEMPLATES = {
    "web-app": [
      { name: "URL", type: "text", required: false },
      { name: "Username", type: "text", required: false },
      { name: "Password", type: "password", required: true },
      { name: "Recovery Email", type: "text", required: false }
    ],
    "ssh-server": [
      { name: "Hostname / IP", type: "text", required: true },
      { name: "Port", type: "text", required: false },
      { name: "Username", type: "text", required: true },
      { name: "Password", type: "password", required: false },
      { name: "Private Key", type: "long_text", required: false },
      { name: "Passphrase", type: "password", required: false }
    ],
    "bank-financial": [
      { name: "Bank Name", type: "text", required: true },
      { name: "Username / Customer ID", type: "text", required: false },
      { name: "Password / PIN", type: "password", required: true },
      { name: "Account Number", type: "text", required: false },
      { name: "Customer Support Number", type: "text", required: false }
    ],
    "email": [
      { name: "Email Address", type: "text", required: true },
      { name: "Password", type: "password", required: true },
      { name: "IMAP Server", type: "text", required: false },
      { name: "SMTP Server", type: "text", required: false }
    ],
    "wi-fi": [
      { name: "SSID", type: "text", required: true },
      { name: "Password", type: "password", required: true },
      { name: "Network Type", type: "text", required: false }
    ],
    "generic": [
      { name: "Username", type: "text", required: false },
      { name: "Password", type: "password", required: true },
      { name: "Notes", type: "long_text", required: false }
    ]
  };

  // Field values
  let fieldValues = $state<Record<string, string>>({});

  // Initialize form data
  $effect(() => {
    if (entry) {
      // Editing existing entry
      selectedCategory = entry.category;
      title = entry.title;
      tags = [...entry.tags];
      notes = entry.notes;

        // Load existing field values
        if (entry.fields) {
            fieldValues = Object.fromEntries(entry.fields.map((f: any) => [f.name, f.value]));

            // Populate custom fields (fields not in the original entry's template)
            const entryTemplate = CATEGORY_TEMPLATES[entry.category as keyof typeof CATEGORY_TEMPLATES] || CATEGORY_TEMPLATES["generic"];
            const templateFieldNames = entryTemplate.map((f: { name: string }) => f.name);
            customFields = entry.fields
                .filter((f: any) => !templateFieldNames.includes(f.name))
                .map((f: any, index: number) => ({
                    name: f.name,
                    type: f.fieldType,
                    value: f.value,
                    id: Date.now() + index
                }));
        }
    } else {
      // New entry
      resetForm();
    }
  });

  function resetForm() {
    selectedCategory = "web-app";
    title = "";
    tags = [];
    notes = "";
    fieldValues = {};
    customFields = [];
    error = "";
  }

  function getCurrentTemplate() {
    return CATEGORY_TEMPLATES[selectedCategory as keyof typeof CATEGORY_TEMPLATES] || CATEGORY_TEMPLATES["generic"];
  }

  function addCustomField() {
    customFields = [...customFields, {
      name: "",
      type: "text",
      value: "",
      id: Date.now()
    }];
  }

  function removeCustomField(index: number) {
    customFields = customFields.filter((_, i) => i !== index);
  }

  function updateCustomField(index: number, field: string, value: string) {
    customFields[index] = { ...customFields[index], [field]: value };
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    loading = true;
    error = "";

    try {
      // Validate required fields
      if (!title.trim()) {
        throw new Error("Title is required");
      }

      // Prepare fields
      const templateFields = getCurrentTemplate();
      const fields = [];

      // Add template fields
      for (const templateField of templateFields) {
        const value = fieldValues[templateField.name] || "";
        if (templateField.required && !value.trim()) {
          throw new Error(`${templateField.name} is required`);
        }

        fields.push({
          name: templateField.name,
          fieldType: templateField.type,
          value: value
        });
      }

      // Add custom fields
      for (const customField of customFields) {
        if (!customField.name.trim()) continue;

        fields.push({
          name: customField.name,
          fieldType: customField.type,
          value: customField.value
        });
      }

      const entryData = {
        category: selectedCategory,
        title: title.trim(),
        fields,
        tags: tags.filter((tag: string) => tag.trim()),
        notes: notes.trim()
      };

      if (entry) {
        // Update existing entry
        await invoke("update_entry", {
          request: {
            entryId: entry.id,
            ...entryData
          }
        });
      } else {
        // Create new entry
        await invoke("create_entry", { request: entryData });
      }

      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : "An error occurred";
    } finally {
      loading = false;
    }
  }

  function handleCancel() {
    onClose();
  }

  function openPasswordGenerator(fieldName: string) {
    passwordGeneratorTarget = fieldName;
    showPasswordGenerator = true;
  }

  function handlePasswordGenerated(password: string) {
    if (passwordGeneratorTarget) {
      // Update the field value
      if (passwordGeneratorTarget.startsWith('custom-')) {
        // Handle custom fields
        const index = parseInt(passwordGeneratorTarget.replace('custom-', ''));
        if (customFields[index]) {
          customFields[index].value = password;
        }
      } else {
        // Handle template fields
        fieldValues[passwordGeneratorTarget] = password;
      }
    }
    showPasswordGenerator = false;
    passwordGeneratorTarget = null;
  }
</script>

<!-- Modal Overlay -->
  <div
    class="modal-overlay"
    onclick={handleCancel}
    onkeydown={(e) => e.key === 'Escape' && handleCancel()}
    role="presentation"
  >
    <div
      class="modal-content"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="Entry Form"
      tabindex="-1"
    >
    <div class="modal-header">
      <h2>{entry ? 'Edit Entry' : 'New Entry'}</h2>
      <button class="close-btn" onclick={handleCancel}>✕</button>
    </div>

    <form onsubmit={handleSubmit} class="entry-form">
      <!-- Basic Info -->
      <div class="form-section">
        <div class="form-row">
          <div class="form-group">
            <label for="category">Category</label>
            <select id="category" bind:value={selectedCategory} required>
              {#each Object.keys(CATEGORY_TEMPLATES) as category}
                <option value={category}>
                  {category.replace('-', ' ').replace(/\b\w/g, l => l.toUpperCase())}
                </option>
              {/each}
            </select>
          </div>

          <div class="form-group">
            <label for="title">Title</label>
            <input
              id="title"
              type="text"
              bind:value={title}
              placeholder="Entry title"
              required
            />
          </div>
        </div>

        <div class="form-group">
          <label for="tags">Tags (comma-separated)</label>
          <input
            id="tags"
            type="text"
            bind:value={tags}
            placeholder="work, personal, important"
              oninput={(e) => tags = (e.target as HTMLInputElement).value.split(',').map(t => t.trim())}
          />
        </div>

        <div class="form-group">
          <label for="notes">Notes</label>
          <textarea
            id="notes"
            bind:value={notes}
            placeholder="Additional notes..."
            rows="3"
          ></textarea>
        </div>
      </div>

      <!-- Category Fields -->
      <div class="form-section">
        <h3>Entry Details</h3>
        <div class="fields-grid">
          {#each getCurrentTemplate() as field}
            <div class="form-group">
              <label for={field.name}>
                {field.name}
                {#if field.required}<span class="required">*</span>{/if}
              </label>
              {#if field.type === 'password'}
                <div class="password-field-group">
                  <input
                    id={field.name}
                    type="password"
                    bind:value={fieldValues[field.name]}
                    placeholder={`Enter ${field.name.toLowerCase()}`}
                    required={field.required}
                  />
                  <button
                    type="button"
                    class="generate-password-btn"
                    onclick={() => openPasswordGenerator(field.name)}
                    title="Generate secure password"
                  >
                    🔐
                  </button>
                </div>
              {:else if field.type === 'long_text'}
                <textarea
                  id={field.name}
                  bind:value={fieldValues[field.name]}
                  placeholder={`Enter ${field.name.toLowerCase()}`}
                  rows="4"
                  required={field.required}
                ></textarea>
              {:else}
                <input
                  id={field.name}
                  type="text"
                  bind:value={fieldValues[field.name]}
                  placeholder={`Enter ${field.name.toLowerCase()}`}
                  required={field.required}
                />
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- Custom Fields -->
      <div class="form-section">
        <div class="section-header">
          <h3>Custom Fields</h3>
          <button type="button" class="add-field-btn" onclick={addCustomField}>
            ➕ Add Field
          </button>
        </div>

        {#each customFields as customField, index (customField.id)}
          <div class="custom-field-row">
            <input
              type="text"
              placeholder="Field name"
              bind:value={customField.name}
              oninput={(e) => updateCustomField(index, 'name', (e.target as HTMLInputElement).value)}
            />
            <select
              bind:value={customField.type}
              onchange={(e) => updateCustomField(index, 'type', (e.target as HTMLSelectElement).value)}
            >
              <option value="text">Text</option>
              <option value="password">Password</option>
              <option value="long_text">Long Text</option>
            </select>
            {#if customField.type === 'password'}
              <div class="password-field-group">
                <input
                  type="password"
                  placeholder="Value"
                  bind:value={customField.value}
                  oninput={(e) => updateCustomField(index, 'value', (e.target as HTMLInputElement | HTMLTextAreaElement).value)}
                />
                <button
                  type="button"
                  class="generate-password-btn"
                  onclick={() => openPasswordGenerator(`custom-${index}`)}
                  title="Generate secure password"
                >
                  🔐
                </button>
              </div>
            {:else if customField.type === 'long_text'}
              <textarea
                placeholder="Value"
                bind:value={customField.value}
                oninput={(e) => updateCustomField(index, 'value', (e.target as HTMLTextAreaElement).value)}
                rows="2"
              ></textarea>
            {:else}
              <input
                type="text"
                placeholder="Value"
                bind:value={customField.value}
                oninput={(e) => updateCustomField(index, 'value', (e.target as HTMLInputElement).value)}
              />
            {/if}
            <button type="button" class="remove-field-btn" onclick={() => removeCustomField(index)}>
              🗑️
            </button>
          </div>
        {/each}
      </div>

      {#if error}
        <div class="error-message">{error}</div>
      {/if}

      <!-- Form Actions -->
      <div class="form-actions">
        <button type="button" class="btn-secondary" onclick={handleCancel} disabled={loading}>
          Cancel
        </button>
        <button type="submit" class="btn-primary" disabled={loading}>
          {#if loading}
            <span class="spinner"></span>
            {entry ? 'Updating...' : 'Creating...'}
          {:else}
            {entry ? 'Update Entry' : 'Create Entry'}
          {/if}
        </button>
      </div>
    </form>
  </div>

  {#if showPasswordGenerator}
    <PasswordGenerator
      onClose={() => showPasswordGenerator = false}
      onSelect={handlePasswordGenerated}
    />
  {/if}
</div>

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
    max-width: 800px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h2 {
    margin: 0;
    color: #1a202c;
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

  .entry-form {
    padding: 24px;
  }

  .form-section {
    margin-bottom: 32px;
  }

  .form-section h3 {
    margin: 0 0 16px 0;
    color: #2d3748;
    font-size: 1.125rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .add-field-btn {
    background: #edf2f7;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s;
  }

  .add-field-btn:hover {
    background: #e2e8f0;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 16px;
  }

  .form-group {
    margin-bottom: 16px;
  }

  label {
    display: block;
    margin-bottom: 6px;
    color: #4a5568;
    font-weight: 500;
  }

  .required {
    color: #e53e3e;
  }

  input, select, textarea {
    width: 100%;
    padding: 12px;
    border: 2px solid #e1e5e9;
    border-radius: 6px;
    font-size: 16px;
    transition: border-color 0.2s;
    box-sizing: border-box;
  }

  input, select {
    height: 46px;
    padding: 0 12px;
  }

  input:focus, select:focus, textarea:focus {
    outline: none;
    border-color: #667eea;
  }

  textarea {
    resize: vertical;
    min-height: 80px;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 16px;
  }

  .custom-field-row {
    display: grid;
    grid-template-columns: 2fr 1fr 2fr auto;
    gap: 8px;
    align-items: start;
    margin-bottom: 12px;
    padding: 12px;
    background: #f8fafc;
    border-radius: 6px;
    border: 1px solid #e2e8f0;
  }

  .custom-field-row input,
  .custom-field-row select,
  .custom-field-row textarea {
    border: 2px solid #e1e5e9;
    font-size: 16px;
    box-sizing: border-box;
    height: 46px;
    padding: 0 12px;
  }

  .custom-field-row select {
    background: white;
    cursor: pointer;
  }

  .custom-field-row textarea {
    height: auto;
    min-height: 60px;
    padding: 12px;
  }

  .remove-field-btn {
    background: #fed7d7;
    color: #c53030;
    border: 1px solid #feb2b2;
    padding: 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .remove-field-btn:hover {
    background: #feb2b2;
  }

  .error-message {
    background: #fed7d7;
    color: #c53030;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 20px;
    border: 1px solid #feb2b2;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding-top: 20px;
    border-top: 1px solid #e2e8f0;
  }

  .btn-primary {
    background: #667eea;
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-primary:hover:not(:disabled) {
    background: #5a67d8;
  }

  .btn-primary:disabled {
    background: #a0aec0;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: white;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 12px 24px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #f7fafc;
    border-color: #cbd5e0;
  }

  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid transparent;
    border-top: 2px solid currentColor;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  @media (max-width: 768px) {
    .modal-content {
      margin: 10px;
      max-height: calc(100vh - 20px);
    }

    .modal-header,
    .entry-form {
      padding: 16px;
    }

    .form-row {
      grid-template-columns: 1fr;
    }

    .fields-grid {
      grid-template-columns: 1fr;
    }

    .custom-field-row {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .form-actions {
      flex-direction: column-reverse;
    }

    .btn-primary,
    .btn-secondary {
      width: 100%;
    }
  }

  .password-field-group {
    display: flex;
    gap: 8px;
    align-items: stretch;
  }

  .password-field-group input {
    flex: 1;
  }

  .generate-password-btn {
    background: #e2e8f0;
    border: 1px solid #cbd5e0;
    border-radius: 6px;
    padding: 12px 16px;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 48px;
  }

  .generate-password-btn:hover {
    background: #cbd5e0;
    border-color: #a0aec0;
  }

  .generate-password-btn:active {
    background: #a0aec0;
  }
</style>