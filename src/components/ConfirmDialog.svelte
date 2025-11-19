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
    onCancel
  }: Props = $props();
</script>

<!-- Modal Overlay -->
<div class="modal-overlay" onclick={onCancel}>
  <div class="modal-content" onclick={(event) => event.stopPropagation()}>
    <div class="modal-header">
      <h2>{title}</h2>
    </div>

    <div class="modal-body">
      <p>{message}</p>
    </div>

    <div class="modal-footer">
      <button class="btn-secondary" onclick={onCancel}>
        {cancelText}
      </button>
      <button class="btn-primary {danger ? 'danger' : ''}" onclick={onConfirm}>
        {confirmText}
      </button>
    </div>
  </div>
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
    z-index: 2000;
    padding: 20px;
  }

  .modal-content {
    background: white;
    border-radius: 12px;
    width: 100%;
    max-width: 400px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    padding: 24px 24px 16px 24px;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h2 {
    margin: 0;
    color: #1a202c;
    font-size: 1.25rem;
  }

  .modal-body {
    padding: 16px 24px;
  }

  .modal-body p {
    margin: 0;
    color: #4a5568;
    line-height: 1.5;
  }

  .modal-footer {
    padding: 16px 24px 24px 24px;
    display: flex;
    justify-content: flex-end;
    gap: 12px;
  }

  .btn-primary {
    background: #667eea;
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: #5a67d8;
  }

  .btn-primary.danger {
    background: #e53e3e;
  }

  .btn-primary.danger:hover:not(:disabled) {
    background: #c53030;
  }

  .btn-secondary {
    background: white;
    color: #4a5568;
    border: 1px solid #e2e8f0;
    padding: 10px 20px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #f7fafc;
    border-color: #cbd5e0;
  }

  @media (max-width: 480px) {
    .modal-content {
      margin: 10px;
    }

    .modal-header,
    .modal-body,
    .modal-footer {
      padding-left: 16px;
      padding-right: 16px;
    }

    .modal-footer {
      flex-direction: column;
    }

    .btn-primary,
    .btn-secondary {
      width: 100%;
    }
  }
</style>