<script lang="ts">
  interface Props {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    title,
    message,
    confirmText = "Confirm",
    cancelText = "Cancel",
    danger = false,
    onConfirm,
    onCancel,
  }: Props = $props();
</script>

<div class="modal-overlay confirm-overlay" onclick={onCancel} onkeydown={(e) => e.key === "Escape" && onCancel()} role="presentation">
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => { e.stopPropagation(); if (e.key === "Escape") onCancel(); }}
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-message"
    tabindex="-1"
  >
    <div class="modal-header">
      <h2 id="confirm-title">{title}</h2>
    </div>
    <div class="modal-body">
      <p id="confirm-message">{message}</p>
    </div>
    <div class="modal-footer">
      <!-- Focus starts on Cancel so Enter never confirms a destructive action by accident. -->
      <!-- svelte-ignore a11y_autofocus -->
      <button class="btn btn-secondary" onclick={onCancel} autofocus>{cancelText}</button>
      <button class="btn {danger ? 'btn-danger' : 'btn-primary'}" onclick={onConfirm}>{confirmText}</button>
    </div>
  </div>
</div>

<style>
  .confirm-overlay { z-index: 2000; }
  .modal-header, .modal-footer { border: 0; }
  .modal-body { padding-top: 0; padding-bottom: 4px; }
</style>
