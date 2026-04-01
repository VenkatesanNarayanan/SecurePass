<script lang="ts">
  interface Props {
    entries: any[];
    onViewEntry: (entry: any) => void;
    onEditEntry: (entry: any) => void;
  }

  let { entries, onViewEntry, onEditEntry }: Props = $props();

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
    return date.toLocaleDateString();
  }
</script>

<div class="entry-list">
  {#if entries.length === 0}
    <div class="empty-state">
      <div class="empty-icon">📝</div>
      <h3>No entries found</h3>
      <p>Create your first password entry to get started.</p>
    </div>
  {:else}
    <div class="entries-grid">
      {#each entries as entry (entry.id)}
         <div
           class="entry-card"
           onclick={() => onViewEntry(entry)}
           onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onViewEntry(entry)}
           role="button"
           tabindex="0"
         >
           <div class="entry-header">
             <div class="entry-icon">
               {getCategoryIcon(entry.category)}
             </div>
             <div class="entry-meta">
               <span class="category">{entry.category.replace('-', ' ')}</span>
               <span class="date">{formatDate(entry.updated_at)}</span>
             </div>
           </div>

           <div class="entry-content">
             <h4 class="entry-title">{entry.title}</h4>
             {#if entry.tags && entry.tags.length > 0}
               <div class="entry-tags">
                 {#each entry.tags.slice(0, 3) as tag}
                   <span class="tag">{tag}</span>
                 {/each}
                 {#if entry.tags.length > 3}
                   <span class="tag more">+{entry.tags.length - 3}</span>
                 {/if}
               </div>
             {/if}
             {#if entry.notes}
               <p class="entry-notes">{entry.notes}</p>
             {/if}
           </div>

           <div class="entry-footer">
             <span class="field-count">
               {entry.field_count} fields
             </span>
             <div class="entry-actions">
               <button class="view-btn" onclick={(event) => { event.stopPropagation(); onViewEntry(entry); }}>
                 👁️ View
               </button>
               <button class="edit-btn" onclick={(event) => { event.stopPropagation(); onEditEntry(entry); }}>
                 ✏️ Edit
               </button>
             </div>
           </div>
         </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .entry-list {
    width: 100%;
  }

  .empty-state {
    text-align: center;
    padding: 60px 20px;
    color: #718096;
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .empty-state h3 {
    margin: 0 0 8px 0;
    color: #4a5568;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
  }

  .entries-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }

  .entry-card {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .entry-card:hover {
    border-color: #667eea;
    box-shadow: 0 4px 12px rgba(102, 126, 234, 0.15);
    transform: translateY(-1px);
  }

  .entry-header {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }

  .entry-icon {
    font-size: 1.5rem;
    margin-right: 12px;
  }

  .entry-meta {
    flex: 1;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .category {
    background: #edf2f7;
    color: #4a5568;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 500;
    text-transform: capitalize;
  }

  .date {
    color: #a0aec0;
    font-size: 0.75rem;
  }

  .entry-content {
    margin-bottom: 16px;
  }

  .entry-title {
    margin: 0 0 8px 0;
    color: #1a202c;
    font-size: 1.125rem;
    font-weight: 600;
  }

  .entry-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }

  .tag {
    background: #e6fffa;
    color: #234e52;
    padding: 2px 6px;
    border-radius: 8px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .tag.more {
    background: #fed7d7;
    color: #9b2c2c;
  }

  .entry-notes {
    margin: 0;
    color: #718096;
    font-size: 0.875rem;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .entry-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .field-count {
    color: #a0aec0;
    font-size: 0.75rem;
  }

  .entry-actions {
    display: flex;
    gap: 8px;
  }

  .view-btn, .edit-btn {
    background: none;
    border: none;
    color: #667eea;
    cursor: pointer;
    font-size: 0.875rem;
    padding: 4px 8px;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .view-btn:hover, .edit-btn:hover {
    background: #edf2f7;
  }

  @media (max-width: 640px) {
    .entries-grid {
      grid-template-columns: 1fr;
    }

    .entry-card {
      padding: 16px;
    }

    .entry-meta {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }
  }
</style>