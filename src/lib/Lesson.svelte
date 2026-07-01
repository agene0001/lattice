<script>
  import { marked } from 'marked';
  import katex from 'katex';
  // mhchem extension: enables \ce{...} chemical notation inside lessons.
  import 'katex/contrib/mhchem';

  let { source = '' } = $props();

  // Render a Markdown lesson with embedded KaTeX. Math spans ($…$ inline,
  // $$…$$ display) are pulled out into placeholders *before* the Markdown pass
  // so Markdown never mangles the LaTeX, then swapped back as rendered HTML
  // afterwards. Bad LaTeX falls back to its source rather than throwing.
  let html = $derived.by(() => render(source));

  // Drop optional `---`-fenced frontmatter (source/license) so it never renders
  // as a stray rule + key lines — mirrors the backend's split_frontmatter, and
  // lets the editor preview raw file content directly.
  function stripFrontmatter(src) {
    if (!src.startsWith('---\n') && !src.startsWith('---\r\n')) return src;
    const lines = src.split('\n');
    for (let i = 1; i < lines.length; i++) {
      if (lines[i].trim() === '---') {
        return lines.slice(i + 1).join('\n').replace(/^[\r\n]+/, '');
      }
    }
    return src; // no closing fence — treat as ordinary body
  }

  function render(raw) {
    if (!raw) return '';
    const src = stripFrontmatter(raw);
    const math = [];
    const stash = (tex, display) => {
      math.push({ tex: tex.trim(), display });
      return `@@LATTICEMATH${math.length - 1}@@`;
    };

    // Display math first so the greedier inline rule can't claim a `$$`.
    let protectedSrc = src
      .replace(/\$\$([\s\S]+?)\$\$/g, (_, tex) => stash(tex, true))
      .replace(/\$([^\n$]+?)\$/g, (_, tex) => stash(tex, false));

    let out = marked.parse(protectedSrc);
    out = out.replace(/@@LATTICEMATH(\d+)@@/g, (_, i) => {
      const { tex, display } = math[Number(i)];
      try {
        return katex.renderToString(tex, { displayMode: display, throwOnError: false });
      } catch {
        return tex;
      }
    });
    return out;
  }
</script>

<div class="lesson-body">{@html html}</div>

<style>
  .lesson-body :global(h1) {
    font-size: 1.5rem;
    margin: 0 0 0.75rem;
  }
  .lesson-body :global(h2) {
    font-size: 1.15rem;
    margin: 1.4rem 0 0.5rem;
  }
  .lesson-body :global(h3) {
    font-size: 1rem;
    margin: 1.1rem 0 0.4rem;
  }
  .lesson-body :global(p) {
    line-height: 1.65;
    margin: 0.6rem 0;
  }
  .lesson-body :global(ul),
  .lesson-body :global(ol) {
    line-height: 1.65;
    padding-left: 1.4rem;
  }
  .lesson-body :global(li) {
    margin: 0.25rem 0;
  }
  .lesson-body :global(code) {
    background: var(--panel-2);
    border-radius: 5px;
    padding: 0.08rem 0.35rem;
    font-size: 0.88em;
  }
  .lesson-body :global(pre) {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 0.8rem 0.9rem;
    overflow-x: auto;
  }
  .lesson-body :global(pre code) {
    background: none;
    padding: 0;
  }
  .lesson-body :global(blockquote) {
    margin: 1rem 0;
    padding: 0.6rem 0.9rem;
    border-left: 3px solid var(--accent);
    background: rgba(76, 141, 255, 0.08);
    border-radius: 0 8px 8px 0;
  }
  .lesson-body :global(blockquote p) {
    margin: 0.25rem 0;
  }
  .lesson-body :global(.katex-display) {
    overflow-x: auto;
    overflow-y: hidden;
    padding: 0.3rem 0;
  }
  .lesson-body :global(a) {
    color: var(--accent);
  }
  /* Diagrams: images (from static/) and inline SVG (free-body, circuits, …). */
  .lesson-body :global(img),
  .lesson-body :global(svg) {
    display: block;
    max-width: 100%;
    height: auto;
    margin: 1rem auto;
  }
  .lesson-body :global(figure) {
    margin: 1rem 0;
    text-align: center;
  }
  .lesson-body :global(figcaption) {
    color: var(--muted);
    font-size: 0.85rem;
    margin-top: 0.35rem;
  }
  .lesson-body :global(table) {
    border-collapse: collapse;
    margin: 0.8rem 0;
  }
  .lesson-body :global(th),
  .lesson-body :global(td) {
    border: 1px solid var(--border);
    padding: 0.4rem 0.7rem;
  }
  /* Check-yourself reveals (<details>/<summary>) */
  .lesson-body :global(details) {
    margin: 0.5rem 0;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    padding: 0.1rem 0.9rem;
  }
  .lesson-body :global(details[open]) {
    background: rgba(76, 141, 255, 0.06);
    border-color: rgba(76, 141, 255, 0.35);
  }
  .lesson-body :global(summary) {
    cursor: pointer;
    padding: 0.55rem 0;
    font-weight: 600;
    list-style: none;
  }
  .lesson-body :global(summary::before) {
    content: '▸ ';
    color: var(--accent);
  }
  .lesson-body :global(details[open] summary::before) {
    content: '▾ ';
  }
  .lesson-body :global(summary::-webkit-details-marker) {
    display: none;
  }
  .lesson-body :global(details > p) {
    margin: 0.2rem 0 0.7rem;
  }
</style>
