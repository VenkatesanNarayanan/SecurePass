<script lang="ts">
  interface Props {
    query: string;
    categories: string[];
    selectedCategory: string;
  }

  let { query = $bindable(""), categories, selectedCategory = $bindable("") }: Props = $props();

  function clearFilters() {
    query = "";
    selectedCategory = "";
  }

  function hasActiveFilters(): boolean {
    return query.length > 0 || selectedCategory.length > 0;
  }
</script>

<div class="search-bar">
  <div class="search-input-group">
    <div class="search-icon">🔍</div>
    <input
      type="text"
      bind:value={query}
      placeholder="Search entries by title, username, or tags..."
      class="search-input"
    />
    {#if hasActiveFilters()}
      <button class="clear-btn" onclick={clearFilters} title="Clear filters">
        ✕
      </button>
    {/if}
  </div>

  <div class="filter-group">
    <select bind:value={selectedCategory} class="category-filter">
      <option value="">All Categories</option>
      {#each categories as category}
        <option value={category}>{category.replace('-', ' ').replace(/\b\w/g, l => l.toUpperCase())}</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .search-bar {
    display: flex;
    gap: 16px;
    align-items: center;
    flex: 1;
  }

  .search-input-group {
    position: relative;
    flex: 1;
    max-width: 400px;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #a0aec0;
    font-size: 1rem;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0 40px;
    border: 2px solid #e1e5e9;
    border-radius: 6px;
    font-size: 16px;
    transition: border-color 0.2s;
    box-sizing: border-box;
    height: 46px;
  }

  .search-input:focus {
    outline: none;
    border-color: #667eea;
  }

  .clear-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: #a0aec0;
    cursor: pointer;
    font-size: 1rem;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .clear-btn:hover {
    background: #fed7d7;
    color: #e53e3e;
  }

  .filter-group {
    display: flex;
    gap: 8px;
  }

  .category-filter {
    padding: 0 12px;
    border: 2px solid #e1e5e9;
    border-radius: 6px;
    background: white;
    font-size: 16px;
    color: #4a5568;
    cursor: pointer;
    transition: border-color 0.2s;
    min-width: 160px;
    box-sizing: border-box;
    height: 46px;
  }

  .category-filter:focus {
    outline: none;
    border-color: #667eea;
  }

  .category-filter option {
    padding: 8px;
  }

  @media (max-width: 768px) {
    .search-bar {
      flex-direction: column;
      align-items: stretch;
      gap: 12px;
    }

    .search-input-group {
      max-width: none;
    }

    .filter-group {
      justify-content: center;
    }

    .category-filter {
      flex: 1;
      min-width: 0;
    }
  }

  @media (max-width: 480px) {
    .search-input {
      font-size: 16px; /* Prevents zoom on iOS */
    }

    .category-filter {
      font-size: 16px; /* Prevents zoom on iOS */
    }
  }
</style>