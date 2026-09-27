# Boutique

Libraries for server-rendered Rust web sites built on axum, maud and htmx.

## /boutique
Server, auth, session, htmx helpers, `AppError`, and page head. Pure Rust. Ships `htmx.js` and `islands.js` from the binary.

No Alpine. Small client behaviours are custom elements: they survive htmx swaps and morphs with no compat layer. Use `hx-morph-skip` on elements that own their inner DOM.

## /bq_macros
`#[component]` attribute macro, re-exported from `bq_components`.

## /bq_components
Shared UI components. Each component is one folder with `mod.rs` and its colocated `.css` (and `.ts` when needed).

The CSS is the crossword.blue button and input, moved as-is. It reads these variables, which a site must define in `:root`:

- `--color-surface`, `--color-text`, `--color-border`
- `--color-1`, `--color-on-brand`
- `--color-contrast`, `--color-on-contrast`
- `--color-focus`, `--color-secondary-active`, `--color-wrong`

Target role set (not yet applied): `--color-surface`, `--color-text`, `--color-border`, `--color-primary`, `--color-error`, `--color-muted`, `--color-surface-muted`, `--font-body`.

To bundle the CSS, add an alias in `vite.config.ts` that points at `bq_components/src`, then glob `**/*.css` from the site entry.

## Consuming from a site

Rust: depend on `boutique` and `bq_components` with `git = "https://github.com/avavuvu/boutique.git", rev = "..."`.
For local work, add a `.cargo/config.toml` (gitignored) in the site workspace:

```toml
[patch."https://github.com/avavuvu/boutique.git"]
boutique = { path = "../boutique/boutique" }
bq_components = { path = "../boutique/bq_components" }
```

JavaScript: add `"boutique": "github:avavuvu/boutique#<same rev>"` to `devDependencies`. The Vite alias can prefer a local sibling checkout when present and fall back to `node_modules/boutique`.

Keep the Cargo `rev` and the package.json ref equal.
