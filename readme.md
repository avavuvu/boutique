# Boutique

Libraries for server-rendered Rust web sites built on axum, maud and htmx.

## /boutique
Server, auth, session, htmx helpers, `AppError`, and page head. Pure Rust. Ships `htmx.js` and `islands.js` from the binary.

No Alpine. Small client behaviours are custom elements: they survive htmx swaps and morphs with no compat layer. Use `hx-morph-skip` on elements that own their inner DOM.

## /macros (`bq_macros`)
`#[component]` attribute macro, re-exported from `bq_components`.

## /style
Plain CSS, no crate. Import these first, before component CSS:

- `reset.css`: Tailwind preflight, body font from `--font-body`
- `tokens.css`: width tokens `--xs` to `--xl`
- `roles.css`: the palette contract as `:root` defaults. A site overrides the material tokens (`--ink`, `--paper`, `--brand`, `--font-body`, `--font-title`) or any role in its own `:root`, loaded after this file.

## /components (`bq_components`)
Shared UI components. Each component is one folder with `mod.rs` and its colocated `.css` and `.ts`. Files with a `_` prefix are private helpers, not entry points. Scripts self-register on load (event delegation on `document`, or `customElements.define`); nothing needs to be called.

| Folder | Contents |
|---|---|
| `base/` | Behaviour CSS the Rust components rely on: `.error:empty`, `[aria-busy]`, `.htmx-request`, `.sr-only` |
| `button/` | `Button` and `.button` look: `.primary`, `.secondary`, `.danger`, `.ghost`, `.small` |
| `input/` | `Input` and `.input-component` look, password toggle |
| `behaviours/` | Delegated handlers: `[data-copy]`, `[data-share]`, `[data-copied]`, `input[data-select-all]`, `[data-open]`, `textarea[data-submit-on-enter]` |
| `autosave/` | `form[data-autosave]` dirty tracking, `a[data-leave]` flush-then-navigate, `beforeunload` guard |

### Palette contract

Component CSS reads only these roles. A site defines them in `:root`:

| Role | Purpose |
|---|---|
| `--color-surface` | page and control background |
| `--color-text` | body text |
| `--color-border` | control borders |
| `--color-primary` | brand accent, focus rings |
| `--color-error` | validation and danger |
| `--color-muted` | secondary text |
| `--color-surface-muted` | hover fills, subtle backgrounds |
| `--font-body` | body font stack |

Recommended shape for a site stylesheet, so user themes can override a few material tokens:

```css
:root {
    --ink: #000;
    --paper: #fff;
    --brand: #92ca3a;
    --font-body: "Times", serif;
    --font-title: "Playfair Display", serif;

    --color-surface: var(--paper);
    --color-text: var(--ink);
    --color-border: var(--ink);
    --color-primary: var(--brand);
    --color-error: #af1d27;
    --color-muted: color-mix(in srgb, var(--ink) 33%, var(--paper));
    --color-surface-muted: color-mix(in srgb, var(--ink) 7%, var(--paper));
}
```

Named themes are `[data-theme="name"]` blocks that override only what differs. `Head::theme(name)` renders the attribute on `<html>`.

### Bundling

In `vite.config.ts` alias `@bq` to the boutique repository root (a sibling checkout, or `node_modules/boutique`), then from the site entry:

```ts
import "@bq/style/reset.css";
import "@bq/style/tokens.css";
import "@bq/style/roles.css";
import.meta.glob("@bq/components/src/**/*.css", { eager: true });
import.meta.glob(["@bq/components/src/**/*.ts", "!@bq/components/src/**/_*.ts"], { eager: true });
```

## Consuming from a site

Rust: depend on `boutique` and `bq_components` with `git = "https://github.com/avavuvu/boutique.git", rev = "..."`.
For local work, add a `.cargo/config.toml` (gitignored) in the site workspace:

```toml
[patch."https://github.com/avavuvu/boutique.git"]
boutique = { path = "../boutique/boutique" }
bq_components = { path = "../boutique/components" }
```

JavaScript: add `"boutique": "github:avavuvu/boutique#<same rev>"` to `devDependencies`. The Vite alias can prefer a local sibling checkout when present and fall back to `node_modules/boutique`.

Keep the Cargo `rev` and the package.json ref equal.
