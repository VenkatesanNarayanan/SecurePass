<script lang="ts">
  import "../app.css";
  import { applyTheme, getThemePreference } from "$lib/theme";

  let { children } = $props();

  applyTheme(getThemePreference());
</script>

{@render children()}
