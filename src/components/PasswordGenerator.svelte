<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    onClose: () => void;
    onSelect: (password: string) => void;
  }

  let { onClose, onSelect }: Props = $props();

  // Generator options
  let length = $state(16);
  let includeUppercase = $state(true);
  let includeLowercase = $state(true);
  let includeNumbers = $state(true);
  let includeSymbols = $state(true);
  let excludeSimilar = $state(false);
  let generatedPassword = $state("");
  let loading = $state(false);

  // Character sets
  const UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  const LOWERCASE = "abcdefghijklmnopqrstuvwxyz";
  const NUMBERS = "0123456789";
  const SYMBOLS = "!@#$%^&*()_+-=[]{}|;:,.<>?";

  // Similar characters to exclude
  const SIMILAR_CHARS = "0O1lI";

  async function generatePassword() {
    loading = true;
    try {
      // Build character set based on options
      let charset = "";
      if (includeUppercase) charset += UPPERCASE;
      if (includeLowercase) charset += LOWERCASE;
      if (includeNumbers) charset += NUMBERS;
      if (includeSymbols) charset += SYMBOLS;

      // Remove similar characters if option is selected
      if (excludeSimilar) {
        charset = charset.split('').filter(char => !SIMILAR_CHARS.includes(char)).join('');
      }

      if (charset.length === 0) {
        generatedPassword = "Please select at least one character type";
        return;
      }

      // Generate password using secure random
      const password = await invoke("generate_password", {
        request: {
          length,
          charset
        }
      });

      generatedPassword = password as string;
    } catch (error) {
      console.error("Failed to generate password:", error);
      generatedPassword = "Error generating password";
    } finally {
      loading = false;
    }
  }

  function handleSelect() {
    if (generatedPassword && !generatedPassword.startsWith("Please") && !generatedPassword.startsWith("Error")) {
      onSelect(generatedPassword);
      onClose();
    }
  }

  function handleCancel() {
    onClose();
  }

  // Auto-generate on mount and when options change
  $effect(() => {
    if (length || includeUppercase || includeLowercase || includeNumbers || includeSymbols || excludeSimilar) {
      generatePassword();
    }
  });

  function calculateStrength() {
    if (!generatedPassword || generatedPassword.startsWith("Please") || generatedPassword.startsWith("Error")) {
      return { level: 'weak', percentage: 0, text: 'No password' };
    }

    let score = 0;
    const password = generatedPassword;

    // Length scoring
    if (password.length >= 8) score += 1;
    if (password.length >= 12) score += 1;
    if (password.length >= 16) score += 1;
    if (password.length >= 20) score += 1;

    // Character variety scoring
    if (/[a-z]/.test(password)) score += 1;
    if (/[A-Z]/.test(password)) score += 1;
    if (/[0-9]/.test(password)) score += 1;
    if (/[^a-zA-Z0-9]/.test(password)) score += 1;

    // Determine strength level
    let level: string;
    let percentage: number;
    let text: string;

    if (score <= 3) {
      level = 'weak';
      percentage = 25;
      text = 'Weak';
    } else if (score <= 5) {
      level = 'fair';
      percentage = 50;
      text = 'Fair';
    } else if (score <= 7) {
      level = 'good';
      percentage = 75;
      text = 'Good';
    } else {
      level = 'strong';
      percentage = 100;
      text = 'Strong';
    }

    return { level, percentage, text };
  }
</script>

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
    aria-label="Password Generator"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2>🔐 Password Generator</h2>
      <button class="close-btn" onclick={handleCancel}>✕</button>
    </div>

    <div class="generator-content">
      <!-- Generated Password Display -->
      <div class="password-display">
        <div class="password-field">
          <input
            type="text"
            value={generatedPassword}
            readonly
            placeholder="Generated password will appear here"
          />
          <button
            type="button"
            class="copy-btn"
            onclick={() => navigator.clipboard.writeText(generatedPassword)}
            disabled={!generatedPassword || generatedPassword.startsWith("Please") || generatedPassword.startsWith("Error")}
          >
            📋
          </button>
        </div>
        {#if loading}
          <div class="loading-indicator">Generating...</div>
        {/if}
      </div>

      <!-- Generator Options -->
      <div class="options-section">
        <h3>Options</h3>

        <div class="option-group">
          <label for="length">Password Length: {length}</label>
          <input
            id="length"
            type="range"
            min="8"
            max="64"
            bind:value={length}
          />
          <div class="range-labels">
            <span>8</span>
            <span>64</span>
          </div>
        </div>

        <div class="checkbox-options">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={includeUppercase} />
            <span>Include uppercase letters (A-Z)</span>
          </label>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={includeLowercase} />
            <span>Include lowercase letters (a-z)</span>
          </label>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={includeNumbers} />
            <span>Include numbers (0-9)</span>
          </label>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={includeSymbols} />
            <span>Include symbols (!@#$%^&*)</span>
          </label>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={excludeSimilar} />
            <span>Exclude similar characters (0/O, 1/l/I)</span>
          </label>
        </div>
      </div>

      <!-- Strength Indicator -->
      <div class="strength-section">
        <h3>Password Strength</h3>
        <div class="strength-indicator">
          {#if generatedPassword && !generatedPassword.startsWith("Please") && !generatedPassword.startsWith("Error")}
            {@const strength = calculateStrength()}
            <div class="strength-bar">
              <div class="strength-fill strength-{strength.level}" style="width: {strength.percentage}%"></div>
            </div>
            <span class="strength-text">{strength.text}</span>
          {:else}
            <div class="strength-bar">
              <div class="strength-fill strength-weak" style="width: 0%"></div>
            </div>
            <span class="strength-text">No password generated</span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Modal Actions -->
    <div class="modal-footer">
      <button type="button" class="btn-secondary" onclick={handleCancel}>
        Cancel
      </button>
      <button type="button" class="btn-primary" onclick={generatePassword} disabled={loading}>
        🔄 Regenerate
      </button>
      <button
        type="button"
        class="btn-primary"
        onclick={handleSelect}
        disabled={!generatedPassword || generatedPassword.startsWith("Please") || generatedPassword.startsWith("Error")}
      >
        ✅ Use Password
      </button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    max-width: 500px;
    width: 90%;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h2 {
    margin: 0;
    color: #2d3748;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #718096;
    padding: 4px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background-color: #f7fafc;
    color: #2d3748;
  }

  .generator-content {
    padding: 20px;
  }

  .password-display {
    margin-bottom: 24px;
  }

  .password-field {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .password-field input {
    flex: 1;
    padding: 12px;
    border: 2px solid #e2e8f0;
    border-radius: 6px;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 14px;
    background-color: #f8f9fa;
  }

  .password-field input:focus {
    outline: none;
    border-color: #4299e1;
  }

  .copy-btn {
    padding: 12px 16px;
    background-color: #e2e8f0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
  }

  .copy-btn:hover:not(:disabled) {
    background-color: #cbd5e0;
  }

  .copy-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-indicator {
    color: #718096;
    font-size: 14px;
    text-align: center;
  }

  .options-section h3,
  .strength-section h3 {
    margin: 0 0 16px 0;
    color: #2d3748;
    font-size: 1rem;
    font-weight: 600;
  }

  .option-group {
    margin-bottom: 20px;
  }

  .option-group label {
    display: block;
    margin-bottom: 8px;
    color: #4a5568;
    font-weight: 500;
  }

  .option-group input[type="range"] {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: #e2e8f0;
    outline: none;
    -webkit-appearance: none;
  }

  .option-group input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #4299e1;
    cursor: pointer;
  }

  .option-group input[type="range"]::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #4299e1;
    cursor: pointer;
    border: none;
  }

  .range-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 12px;
    color: #718096;
  }

  .checkbox-options {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    color: #4a5568;
  }

  .checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: #4299e1;
  }

  .strength-section {
    margin-top: 24px;
    padding-top: 20px;
    border-top: 1px solid #e2e8f0;
  }

  .strength-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .strength-bar {
    flex: 1;
    height: 8px;
    background-color: #e2e8f0;
    border-radius: 4px;
    overflow: hidden;
  }

  .strength-fill {
    height: 100%;
    transition: width 0.3s ease, background-color 0.3s ease;
  }

  .strength-weak { background-color: #e53e3e; }
  .strength-fair { background-color: #dd6b20; }
  .strength-good { background-color: #38a169; }
  .strength-strong { background-color: #3182ce; }

  .strength-text {
    font-size: 14px;
    font-weight: 500;
    color: #4a5568;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 20px;
    border-top: 1px solid #e2e8f0;
  }

  .btn-secondary {
    padding: 10px 16px;
    background-color: #e2e8f0;
    color: #4a5568;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .btn-secondary:hover {
    background-color: #cbd5e0;
  }

  .btn-primary {
    padding: 10px 16px;
    background-color: #4299e1;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: #3182ce;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
 </style>