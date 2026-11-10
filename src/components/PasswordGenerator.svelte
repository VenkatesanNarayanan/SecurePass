<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "./Icon.svelte";
  import { passwordStrength } from "$lib/strength";
  import { copySecret } from "$lib/clipboard";

  // Inline generator shown under a password field in EntryForm. Every new
  // password is pushed straight into the field through onGenerate.
  interface Props {
    onGenerate: (password: string) => void;
    onClose: () => void;
  }

  let { onGenerate, onClose }: Props = $props();

  const PREFS_KEY = "sp_generator_prefs";

  interface Prefs {
    length: number;
    upper: boolean;
    lower: boolean;
    numbers: boolean;
    symbols: boolean;
    avoidSimilar: boolean;
  }

  function loadPrefs(): Prefs {
    const defaults: Prefs = { length: 20, upper: true, lower: true, numbers: true, symbols: true, avoidSimilar: false };
    try {
      const raw = localStorage.getItem(PREFS_KEY);
      return raw ? { ...defaults, ...JSON.parse(raw) } : defaults;
    } catch {
      return defaults;
    }
  }

  const initial = loadPrefs();
  let length = $state(initial.length);
  let upper = $state(initial.upper);
  let lower = $state(initial.lower);
  let numbers = $state(initial.numbers);
  let symbols = $state(initial.symbols);
  let avoidSimilar = $state(initial.avoidSimilar);
  let current = $state("");
  let error = $state("");

  const UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  const LOWERCASE = "abcdefghijklmnopqrstuvwxyz";
  const NUMBERS = "0123456789";
  const SYMBOLS = "!@#$%^&*()_+-=[]{}|;:,.<>?";
  const SIMILAR = "0O1lI";

  async function generate() {
    let charset = "";
    if (upper) charset += UPPERCASE;
    if (lower) charset += LOWERCASE;
    if (numbers) charset += NUMBERS;
    if (symbols) charset += SYMBOLS;
    if (avoidSimilar) charset = [...charset].filter((c) => !SIMILAR.includes(c)).join("");
    const len = length;

    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify({ length, upper, lower, numbers, symbols, avoidSimilar }));
    } catch { /* ignore */ }

    if (!charset) {
      error = "Pick at least one character type.";
      return;
    }
    error = "";
    try {
      const pw = await invoke<string>("generate_password", { request: { length: len, charset } });
      current = pw;
      onGenerate(pw);
    } catch (e) {
      error = "Couldn't generate a password: " + (e instanceof Error ? e.message : String(e));
    }
  }

  // Regenerate whenever an option changes (and once on open).
  $effect(() => {
    generate();
  });

  const strength = $derived(passwordStrength(current));

  // Copy uses the same auto-clearing clipboard as the entry view.
  let copiedFor = $state(0); // seconds until the clipboard clears; 0 = not copied
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  async function copy() {
    if (!current) return;
    try {
      copiedFor = await copySecret(current);
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copiedFor = 0), 2500);
    } catch (e) {
      error = "Couldn't copy to the clipboard: " + (e instanceof Error ? e.message : String(e));
    }
  }

  $effect(() => () => {
    if (copiedTimer) clearTimeout(copiedTimer);
  });
</script>

<div class="generator" role="group" aria-label="Password generator">
  <div class="row">
    <label for="gen-length" class="row-label">Length</label>
    <input id="gen-length" type="range" min="8" max="64" bind:value={length} />
    <span class="len mono">{length}</span>
  </div>

  <div class="options">
    <label><input type="checkbox" bind:checked={upper} /> A–Z</label>
    <label><input type="checkbox" bind:checked={lower} /> a–z</label>
    <label><input type="checkbox" bind:checked={numbers} /> 0–9</label>
    <label><input type="checkbox" bind:checked={symbols} /> Symbols</label>
    <label><input type="checkbox" bind:checked={avoidSimilar} /> Avoid look-alikes (0/O, 1/l/I)</label>
  </div>

  {#if error}
    <div class="msg msg-error">{error}</div>
  {:else}
    <div class="bars" aria-hidden="true">
      {#each [1, 2, 3, 4] as i}
        <span style="background: {i <= strength.score ? strength.color : 'var(--strength-empty)'}"></span>
      {/each}
    </div>
  {/if}

  <div class="foot">
    <span class="strength" style="color: {copiedFor ? 'var(--success-text)' : strength.color}" role="status">
      {#if copiedFor}
        Copied · clipboard clears in {copiedFor}s
      {:else if strength.score > 0}
        {strength.label} · {length} characters
      {/if}
    </span>
    <div class="foot-actions">
      <button type="button" class="btn btn-ghost" onclick={generate}><Icon name="refresh" size={14} /> Regenerate</button>
      <button type="button" class="btn btn-ghost" onclick={copy} disabled={!current}>
        <Icon name={copiedFor ? "check" : "copy"} size={14} /> {copiedFor ? "Copied" : "Copy"}
      </button>
      <button type="button" class="btn btn-secondary" onclick={onClose}>Done</button>
    </div>
  </div>
</div>

<style>
  .generator {
    background: var(--bg-subtle);
    border-top: 1px solid var(--border-subtle);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row { display: flex; align-items: center; gap: 12px; }
  .row-label { font-size: 13px; color: var(--text-2); width: 56px; }
  .row input[type="range"] { flex: 1; }
  .len { font-size: 13px; width: 24px; text-align: right; }

  .options { display: flex; gap: 8px 18px; flex-wrap: wrap; font-size: 13px; color: var(--text-2); }
  .options label { display: flex; align-items: center; gap: 6px; cursor: pointer; }
  .options input { width: 16px; height: 16px; margin: 0; }

  .bars { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 4px; }
  .bars span { height: 4px; border-radius: 2px; }

  .foot { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
  .strength { font-size: 12px; font-weight: 600; }
  .foot-actions { display: flex; gap: 6px; }
  .foot-actions .btn { min-height: 32px; font-size: 12px; }
</style>
