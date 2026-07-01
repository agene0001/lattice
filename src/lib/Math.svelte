<script>
  import katex from 'katex';
  // mhchem extension: enables \ce{...} / \pu{...} for chemical equations & units.
  import 'katex/contrib/mhchem';

  let { tex = '', display = false } = $props();

  // Render to an HTML string; never throw on malformed LaTeX — fall back to the
  // raw source so a bad template surfaces visibly rather than crashing the view.
  let html = $derived.by(() => {
    try {
      return katex.renderToString(tex, { displayMode: display, throwOnError: false });
    } catch {
      return tex;
    }
  });
</script>

{#if display}
  <div class="katex-block">{@html html}</div>
{:else}
  <span>{@html html}</span>
{/if}

<style>
  .katex-block {
    overflow-x: auto;
    padding: 0.25rem 0;
  }
</style>
