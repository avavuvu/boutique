# Boutique

Libraries for server-rendered Rust web sites built on axum, maud and htmx.

## /boutique
Server, auth, session, htmx helpers and page head. Pure Rust. Ships `htmx.js`, `islands.js` and `hx-alpine-compat.js` from the binary.

## /bq_components
Shared UI components. Each component is one folder with `mod.rs` and its colocated `.css` (and `.ts` when needed).

The CSS is the crossword.blue button and input, moved as-is. It reads these variables, which a site must define in `:root`:

- `--color-surface`, `--color-text`, `--color-border`
- `--color-1`, `--color-on-brand`
- `--color-contrast`, `--color-on-contrast`
- `--color-focus`, `--color-secondary-active`, `--color-wrong`

Target role set (not yet applied): `--color-surface`, `--color-text`, `--color-border`, `--color-primary`, `--color-error`, `--color-muted`, `--color-surface-muted`, `--font-body`.

To bundle the CSS, add an alias in `vite.config.ts` that points at `bq_components/src`, then glob `**/*.css` from the site entry.
