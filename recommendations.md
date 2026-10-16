# Boutique recommendations

Implementation plan for boutique, based on a review of boutique and its two consumer apps. Written for agents implementing it. Read **Working on this** before starting any item.

## Progress

As of 2026-10-15. Done items are committed on `main`; each commit message starts with its item number.

| Item | State |
|---|---|
| 1.1, 1.2, 1.3 | Done |
| 2.1, 2.2, 2.4, 2.5 | Done |
| 2.3 | Open |
| 2.6 | Done: crossword migrated (crossword commits `7576b40`…`9cc2a3f`) |
| 3.1–3.11 | Done (3.1 has no `.auth(state)`: with the legacy layer gone, `Server` needs no auth) |
| 4.1, 4.3–4.6 | Done (`.grid` is named `.auto-grid`) |
| 4.2, 4.7 | Later |
| 5 Bun | Done in boutique and showroom |
| 5 file structure, 6, 7, 8 | Later |
| 9.2 | Done except `publish = false` (licenses: `MIT OR Apache-2.0`) |
| 3.12 Compression and static-file caching | Done (`Cache` policies; build assets immutable; `.vite/` hidden) |
| Rough edges round 1 | Done: decisions 1–5 below and the bugs listed under "Known rough edges" as of 2026-10-15 |
| 9.4 | Done |
| Showroom migration | Done (showroom commits `bf91f7f`…`284b213`); view CSS colocated and ascii background as a component (`f7288bf`…`4005490`) |

### Open decisions from implementation

1. **Licenses** for `bq_macros`, `bq_components`, `bq_setups` and `boutique` (`bq_html` is `MIT OR Apache-2.0` with maud's license files). Also: when to remove `publish = false`, and whether internal `version = "0.2.0"` requirements move into `[workspace.dependencies]`.
2. ~~**`components.css` is broader than the kit's look.**~~ Done: split into `components.css` (kit only), `elements.css` (global element styles, new `bq.elements` layer below `bq.components`), `prose.css` and `feedback.css`, all opt-in. See the readme's CSS section.
3. **Class-name collisions.** `.flow`, `.article-layout` and `.prose` merge with app rules of the same name (showroom had to override `grid-column`). Decide whether boutique's primitives get a prefix.
4. **Error page fallbacks.** If an extractor in the `error_page` render handler fails, its rejection body becomes the page; boutique should fall back to `views::error_page`. Plain-text error bodies (e.g. axum's form deserialisation messages) are no longer shown. `.error-page` from `error_content` has no boutique CSS.
5. **App CSS order under the glob.** Vite hoists eager `import.meta.glob` imports above every explicit import in `site.ts`, so globbed `src/**/*.css` loads before boutique's CSS and before `app.css`, whatever the source order. Consequences:
   - A globbed file using `@layer app { }` declares `app` before boutique's order statement, making `app` the *lowest* layer, contrary to the readme.
   - Unlayered app files load in alphabetical path order, so `views/layouts/` comes after `views/dashboard/`; showroom needed a second glob to load layouts first.
   - The glob also picks up CSS that is not for the site (showroom's `components/email/email.css`, which is `include_str!`ed into emails).

   Options: import boutique's CSS from a separate entry or a `<link>` before `site` so its order statement always comes first; recommend app sub-layers (`@layer app.layouts, app.components, app.views`) declared in a file imported first; or document a fixed glob order and an exclusion convention (e.g. `*.email.css`). Update the `site.ts` example in 9.4 and section 5 once decided.

### Known rough edges

- `Generator` without `.boutique()` produces imports of `./boutique/setups` that nothing writes. Write the runtime unconditionally.
- `components/src/setups.ts` fails `noUncheckedIndexedAccess` (around line 225). It now lives in the app as plain `.ts`, so `skipLibCheck` does not hide it.
- Bare `bq-setup` detection recognises `html!` by name only; a renamed import (`use … html as h`) gives a false "outside a component" error.
- The generator message `… is not inside an element with bq-setup=(DirtyForm)` still shows the old syntax.
- `UserEntity::PASSWORD_HASH` duplicates `AuthUser::password_hash`.
- `bq_setups` now compiles `bq_components`, `bq_macros`, `bon` and `maud` for the host as a build dependency.
- `yoke-derive 0.8.3` in `Cargo.lock` is yanked.
- `bq_setups` does not walk a let-bound `html!` that is never spliced into another `html!` (e.g. `let row = html! {…}; partial(.., row)`). Its refs and handlers are silently missing from the generated types; crossword split `clue_row` into a function to work around it. Walk such bindings, or warn.
- The reset's `:where(dialog, [popover]) { margin: auto }` re-centres anchor-positioned popovers; crossword had to restore its menus' margins. Limit it to `dialog` and modal popovers, or document it.
- `textarea { min-block-size: 4lh }` in `base.css` also hits one-line, content-sized textareas (crossword's clue fields).
- A `Button` with `href` has the TS tag `"a" | "button"`, so a setup on a link needs narrowing to use `.href`.
- A component called only through its builder produces `unused import` on the function name, not the alias.
- `registry.ts` emits one import line per eager setup, even from the same module.
- `readme.md`'s file tree (`views/lander/lander.rs` + `lander.css`) disagrees with section 5 (`home/mod.rs` + `home.css`).

### Before showroom or crossword deploys

- Push boutique and bump the apps' boutique revs (showroom `7369760`, crossword `8868412`); local builds only work through `.cargo/config.toml`. Crossword's `bun run typecheck` has a pre-existing TS2366 error in `game/reduce/index.ts`.
- Add a bun buildpack on Heroku; the root `build` script now runs `bun install && bun run build`.
- Review the visual differences in a browser. The migration report lists them: prose line-height 1.6, focus rings, legend styling, popover transitions, auto-growing textareas, and others. Each is a one-line override.
- Outstanding password-reset links stop working on deploy (3.11).

## Working on this

### Repositories

| Repo | Path | Notes |
|---|---|---|
| boutique | `/Users/adem/Desktop/dev/boutique` | This repo. All items are implemented here unless stated otherwise. |
| showroom | `/Users/adem/Desktop/showroom/showroom-rs` | Email/newsletter app. Mostly up to date (pinned `3b6d705`). Web crate: `showroom-web`. |
| crossword | `/Users/adem/Desktop/dev/crossword-rs` | Crossword site. Pinned to an old version (`8868412`). Web crate: `crossword-dot-blue`. |

Both apps have a gitignored `.cargo/config.toml` that patches the boutique git dependency to this checkout, and a `vite.config.ts` that prefers this checkout when it exists. **Changes in boutique immediately affect both apps' local builds.** Crossword may already fail to build against boutique HEAD (see 2.6).

### Crates in this workspace

| Crate | Dir | Role |
|---|---|---|
| `boutique` | `boutique/` | Server, auth, sessions, errors, htmx helpers, `Head`/`base` views, assets manifest, cloudinary |
| `bq_components` | `components/` | `Attrs`, concrete components (`Button`, `Input`, `Textarea`, feature `ui`), setups runtime (`components/src/setups.ts`), CSS, re-exports macros |
| `bq_macros` | `macros/` | `#[component]`, `attrs!`, `html!` entry |
| `bq_html` | `html/` | Fork of `maud_macros` 0.27 adding components, `(..attrs)`, `@element`, `bq-*` attribute handling |
| `bq_setups` | `setups/` | Build-time generator: walks Rust sources and writes `bindings/setups.ts` and `bindings/registry.ts` in the app |

CSS lives in `components/src/style/` and next to each kit component.

Generated code resolves the crate path at expansion time (`crate_path()` in `bq_macros`): `::bq_components` or `::boutique::components`.

### Commands

| Purpose | Command | Where |
|---|---|---|
| Rust build and tests | `cargo test --workspace` | boutique |
| Check showroom | `cargo check -p showroom-web` | showroom-rs |
| Check crossword | `cargo check -p crossword-dot-blue` | crossword-rs |
| Bun | `~/.bun/bin/bun` (1.4.x). `bun` on `PATH` is an old 1.1.27; do not use it. | any |

### Rules

- Do not write code comments.
- Commit after each item on `main`: one commit per item, message starting with the item number (e.g. `2.2: Replace bon attributes on component parameters`). Stage files explicitly by path; never `git add -A` or `git commit -a`, since other work may be in progress in the same tree. Do not create branches. Do not push.
- Do not edit showroom or crossword unless the task explicitly includes migrating them. Migration notes are included per item.
- If an item is marked **Open**, do not implement it without a decision from Ava.

### Tests
Ava prefers minimal agent-written tests, because agent-written tests tend to be shaped to pass rather than to check behaviour.

- A small regression set from before the test removal (`88b6a72`) is restored: `components/tests/components.rs`, `html_absolute_paths.rs`, `html_component.rs`, `html_on.rs`, `html_setup.rs`, `html_spread.rs`, and `setups/src/tests.rs`.
- These tests are a fixed reference. When an item changes syntax (`setup!` → bare `bq-setup`, `#[builder(...)]` → `#[default]`/`#[positional]`), convert the test **inputs** mechanically. Do not change **expected outputs** unless the item explicitly changes rendered output, and say so in the commit message.
- Never weaken, skip or delete a restored test to make it pass. If one fails and the item does not explain why, stop and report.
- Write new tests only where an item says "Test:". Everywhere else, "Done when" is verified by building and by checking showroom, not by new tests.
- No trybuild error snapshots. Required compile errors are checked by hand and described in the commit message.

### Status legend

- **Decided**: implement as written.
- **Open**: needs a decision first. The open question is stated.
- **Later**: decided in direction, but scheduled after other phases.

### Order of work

| Phase | Items | Why this order |
|---|---|---|
| 1. Packaging | 1.1, 9.4, 9.2 (dry run only) | Removes version drift and the `@bq` alias before anything else changes |
| 2. Component model | 2.2, then 2.1, then 2.4 | 2.1's examples assume 2.2's attributes. All three are breaking. |
| 3. Rust API | 3.1–3.11 | Mostly independent of each other; 3.6 needs 3.5 |
| 4. CSS | 4.1, 4.3, 4.4, then the rest | 4.2 depends on 7.2 |
| 5. Structure | 5, 6 | Needs phases 1–3 settled |
| 6. Tooling | 7, 8 | Needs the structure settled |
| 7. Publish | 9.3 | After phases 1–3 |

## 1. Packaging and versioning

### 1.1 Pin the version once
**Status:** Decided

**Problem:** each app repeats the git rev for `boutique`, `bq_components` and `bq_setups` in its web crate's `Cargo.toml`, and once more in `package.json` (`github:avavuvu/boutique#rev`). Bumping means editing four places.

**Change:**
- In the template and apps, declare boutique crates once in the root `[workspace.dependencies]` and use `boutique.workspace = true` in member crates.
- Make `boutique` sufficient as the only runtime dependency. Since generated code hardcodes `::bq_components`, either:
  - resolve the crate path in `bq_macros`/`bq_html` with `proc-macro-crate` (emit `::boutique::components::…` when the app depends on `boutique`, `::bq_components::…` otherwise), or
  - keep `bq_components` as a required direct dependency and only document it.

  Prefer `proc-macro-crate`.
- The `package.json` pin goes away with 9.4.

**Done when:** an app depending only on `boutique` (runtime) and `bq_setups` (build) compiles components, `html!`, `attrs!` and `#[component]`. Covered by a test crate or trybuild pass case.

### 1.2 Local linking
**Status:** Decided (follows from 9.4)

**Problem:** the local override lives in three places with machine-specific relative paths: `.cargo/config.toml` (gitignored), `existsSync("../../../dev/boutique")` in `vite.config.ts` (committed), and `paths`/`include` in `tsconfig.json` (committed).

**Change:** after 9.4, all boutique JS and CSS come through Cargo, so the Cargo patch is the only switch. The Vite and tsconfig paths are deleted from apps during migration. `cargo bq link` (8) can toggle the patch.

**Done when:** an app's `vite.config.ts` and `tsconfig.json` contain no reference to boutique's location.

### 1.3 One copy of the setups TypeScript
**Status:** Decided (solved by 9.4)

`Generator::boutique()` reads `components/src` from the Cargo checkout while the bundle imports it from `node_modules/boutique`. They only agree when both revs match. 9.4 removes the npm copy.

## 2. Components and setups

### 2.1 Remove `setup!`; attach a component's setup with bare `bq-setup`
**Status:** Decided. Implement after 2.2.

**Problem:** `setup!(Counter)` next to `fn counter` states the name twice, and `Counter` means two things: the component (value namespace, from the alias `#[component]` generates) and the setup (type namespace, from `enum Counter {}`).

**Rule:** a setup is a component's JS.

**Target syntax:** a bare `bq-setup` attribute inside a `#[component]` function means "this component's own setup goes on this element".

```rust
#[component]
pub fn counter(#[default] start: i32) -> Markup {
    html! {
        div.counter bq-setup {
            output bq-ref="value" { (start) }
            button.button type="button" bq-on:click="increment" { "+1" }
        }
    }
}
```

```ts
import type { Counter } from "@setups";

export const counter: Counter = (_root, { value }) => ({
    increment() {
        value.textContent = String(Number(value.textContent) + 1);
    },
});
```

**Properties to preserve:**
- Placement is explicit, so setups work on `<details>`, `<form>`, inner elements, and inside `@if`/`@for`.
- Components without JS are unaffected.
- Scoping is lexical: `children` are rendered in the caller's function and cannot pick up the wrong setup.
- Call-site use works: inside `theme_form`, `DirtyForm(action) bq-setup { … }` attaches `theme_form`'s setup to `DirtyForm`'s root and merges with `DirtyForm`'s own setup, as `bq-setup` merging works today.

**Implementation:**
1. `#[component]` (`macros/src/lib.rs`) declares a hidden setup type inside the function body, implementing `::bq_components::Setup`. Names derive from the function: attribute value kebab (`counter`), TypeScript type PascalCase (`Counter`), TypeScript export camelCase (`counter`).
2. `bq_html` rewrites bare `bq-setup` to `bq-setup=(<hidden type>)`. Existing `bq-setup` handling (`setup_value`, `setup_attrs` in `html/src/generate.rs`) and the `Setup` trait check then apply unchanged.
3. Rendered output stays `bq-setup="counter"`. The runtime (`components/src/setups.ts`), merging, and `bq-ref`/`bq-on` ownership do not change.
4. `bq_setups` discovers setups from `#[component]` functions containing a bare `bq-setup`, replacing `setup!` discovery. It must apply exactly the same naming rule as step 1. Generator output format (`setups.ts`, `registry.ts`) stays the same.
5. Options move to the attribute macro: `#[component(eager)]` and `#[component(setup = "editor-toolbar")]`. Both must be understood by the macro and by `bq_setups`.
6. Remove `setup!` from `bq_macros` and its re-export from `bq_components`.

**Required errors:**
- Bare `bq-setup` outside a `#[component]` function: a clear message, not "cannot find type `__…`".
- Bare `bq-setup` in a helper function called from a component: same error. Do not bind it to anything implicitly.
- Two setups with the same derived name: the generator error must name both locations and suggest `#[component(setup = "…")]`.

**Restored tests to convert mechanically:** `setups/src/tests.rs` and `components/tests/html_setup.rs` use `setup!` throughout. Also convert `components/src/input/mod.rs` (`PasswordToggle`).

**App migration (separate task):**

| Setup | Where it is attached | Change |
|---|---|---|
| `DirtyForm`, `Menu`, `ColorInput` (showroom) | root of the component with the same name | direct |
| `EditorLinkBar`, `EditorToolbar`, `NewsletterEditor`, `ColorOverride` (showroom) | inside `link_bar`, `toolbar`, `editor_page`, `style_overrides` | `setup = "…"` to keep the name, or rename |
| `ThemePreview` (showroom) | call site on `DirtyForm`, inside `theme_form` | `DirtyForm(action) bq-setup { … }`; becomes `theme-form` unless overridden |
| `PasswordToggle` (boutique) | conditional inner `div` in `Input` | `Input`'s setup with `setup = "password-toggle"` |
| `Dropdown` (showroom) | plain `<details>` in `views/layouts/dashboard_shell.rs` | new `Dropdown` component (candidate for 2.3) |
| `CardProgress` (crossword) | inside `card` | `#[component(eager, setup = "card-progress")]` |

Renames change registry keys, TypeScript export names and prefixed strings such as `bq-on:input="theme-preview.schedule"`.

**Rationale for removing rather than keeping `setup!`:** nearly every setup converts; adding `setup!` back later for mixin-style behaviours is non-breaking, while removing it later would be breaking.

**Done when:** `setup!` no longer exists, the restored tests pass after mechanical conversion, and each required error has been triggered by hand and its message quoted in the commit message.

### 2.2 Replace bon's attributes on component parameters
**Status:** Decided. Implement before 2.1.

**Problem:** `#[component]` passes bon's builder attributes through, so props are written in bon's terms. Current uses across the three repos:

| Attribute | Uses | Meaning |
|---|---|---|
| `#[builder(start_fn)]` | ~15 | positional argument |
| `#[builder(default)] attrs: Attrs` | 10 | always optional |
| `#[builder(default)]` on a `bool` | 6 | flag, off unless given |
| `#[builder(default)]` on an enum or slice | 6 | the type's default |
| `#[builder(default = html! {})] children` | 2 | body is optional |
| `#[builder(default = "0")]` | 2 | a specific default |
| `#[builder(required)]` on an `Option` | 4 | the caller passes an `Option` |

**Change:**
1. **Implicit defaults**, recognised from the source by the macro:
   - a parameter named `attrs` of type `Attrs` is optional, default empty;
   - a parameter of type `bool` is optional, default `false`;
   - a parameter named `children` is optional, default empty markup. A forgotten `{ … }` body is no longer a compile error; this is accepted.

   Do not extend implicit defaults to any other type.
2. **`#[default]`** (uses `Default::default()`) and **`#[default = expr]`** for everything else. `#[component]` strips these before expansion.
3. **`#[positional]`** replaces `#[builder(start_fn)]`.
4. **`name = [expr]`** in component calls passes an `Option` to an optional prop by calling `maybe_name(expr)`, matching maud's `href=[opt]` for attributes. This makes `#[builder(required)]` on `Option` parameters unnecessary.
5. Raw `#[builder(...)]` attributes keep working; they are not documented.

**Implementation:**
1. `#[component]` translates `#[default]`, `#[default = …]` and `#[positional]` into bon's attributes and inserts implicit defaults before emitting `#[bon::builder]`.
2. `builder_options` and `props` in `macros/src/lib.rs` read the new names so the generated props table still shows `optional, default …`.
3. `bq_html` maps `name = [expr]` in component arguments to `.maybe_name(expr)`.
4. Update boutique's own components (`components/src/button/mod.rs`, `components/src/input/mod.rs`) and tests.

**Result:**

```rust
#[component]
pub fn button(
    href: Option<&str>,
    post: Option<&str>,
    submit: bool,
    disabled: bool,
    attrs: Attrs,
    children: Markup,
) -> Markup
```

**Rejected alternatives:** `#[prop(default)]` (longer); `Option<T>` with `unwrap_or` in the body (hides the default from docs); `start: i32 = 0` (not valid Rust inside an attribute macro; a function-like `component! {}` would lose rustfmt and rust-analyzer).

**Done when:** `Button`, `Input` and `Textarea` use no `#[builder(...)]` attributes, the props tables in rustdoc are correct, and the restored tests pass.

### 2.3 Upstream duplicated components
**Status:** Open: which components, and their APIs.

Both apps built their own `icon` and a `<details>` dropdown or menu. Showroom has `dirty_form`; crossword has a copy button (`components/copy.rs`). Candidates: Icon, Dropdown, Menu, Dialog, DirtyForm, CopyButton. Source implementations to start from are in `showroom-web/src/components/` and `crossword-dot-blue/src/components/`.

### 2.4 Colocate component CSS
**Status:** Decided

**Problem:** boutique keeps component CSS in `style/components/`, while apps colocate `index.css` next to `mod.rs`. Showroom copied `button.css` and `input.css` into `showroom-web/src/components/{button,input}/index.css` to fix them.

**Change:**
- Move `style/components/button.css` and `input.css` to `components/src/button/index.css` and `components/src/input/index.css`. `style/components/base.css` (`.error`, `.sr-only`, busy states) is core and moves to `components/src/style/base.css` per 9.4.
- Upstream showroom's fixes: `color: var(--color-on-primary)` for `.primary` and `.secondary` hover; styles for `.hint` and `.suffix` (and `.prefix`), which `Input` renders but boutique does not style. Add `--color-on-primary` to `roles.css`.
- Remove the duplicated `padding` in the password button rule in `input.css`.

**Done when:** every class rendered by `Button` and `Input` has a style, and no CSS remains in `style/components/`.

### 2.5 UI kit behind a feature, with its CSS opt-in separately
**Status:** Decided

**Problem:** `bq_components` mixes the core (`Attrs`, `Setup`, macro re-exports, setups runtime) with concrete components (`Button`, `Input`, `Textarea`). The apps use the kit differently: crossword uses the components and boutique's CSS (`site.ts` globs `@bq/style/components/*.css`); showroom uses the components but copied and modified the CSS.

**Decision:** the kit is a Cargo feature, not a separate crate. Its markup and its look are opted into separately:

| Part | Contents | Opt-in |
|---|---|---|
| Core | `Attrs`, `Setup`, macros, setups runtime, reset/tokens/roles/base CSS | always |
| UI components | `Button`, `Input`, `Textarea` and their setups (`PasswordToggle`); components upstreamed under 2.3 | Cargo feature `ui` |
| UI look | component CSS (`button`, `input`, …) | `site.ts` imports `bindings/boutique/style/components.css` |
| Element, prose and feedback styles | global element defaults; `.prose`; toasts and skeletons | `site.ts` imports `elements.css`, `prose.css` or `feedback.css` |

**Why not a crate:** a sixth crate to publish, and it would need 1.1's `proc-macro-crate` work just to use its own macros.

**Why the CSS is not behind the feature:** `bq_setups` is a build dependency, and Cargo resolves build-dependency features separately, so the generator cannot reliably see whether `ui` is on in the app. It does not need to: the generator always writes the kit's TS and CSS. Registry entries are lazy `load: () => import(…)`, so unused setups cost nothing at runtime, and CSS that is not imported never reaches the bundle.

**Change:**
1. In `bq_components`, gate the concrete components behind `ui`, off by default.
2. `boutique` depends on `bq_components` with `default-features = false`, exposes `ui = ["bq_components/ui"]`, and enables it by default. Apps that want no kit set `default-features = false`.
3. Split boutique CSS into two entries written by 9.4: `bindings/boutique/style/index.css` (core layers) and `bindings/boutique/style/components.css` (kit, in the `bq.components` layer).
4. Write kit CSS as structure plus tokens, so apps restyle it through tokens instead of copying files. Showroom's copies differ only in `font-weight`, `--color-on-primary`, and `.hint`/`.suffix` styles; with 2.4, 4.1 and kit tokens such as `--button-weight`, `--radius` and `--border-width`, showroom can drop its copies.

**Core contract:** `AppError` field errors (`boutique/src/htmx/fragments.rs`) target `#<field>-error`, the id `Input` renders. This stays a core contract regardless of `ui`: forms built without the kit must render that id. Document it in the readme.

**Done when:** boutique builds and tests pass with and without `ui`; an app with `default-features = false` cannot name `Button`; `components.css` contains every kit style and `index.css` none.

### 2.6 Crossword upgrade blockers
**Status:** Information for the crossword migration task.

- `crossword-dot-blue/resources/js/site.ts` imports `@bq/components/src/autosave/autosave` and `@bq/components/src/behaviours/behaviours`, which no longer exist.
- Setups use the old `(card, { progress, signal })` signature; the current runtime calls `(element, refs, context)`.
- `src/views/account/mod.rs` calls `.entry("editor")`, but crossword's `vite.config.ts` defines no `editor` entry, so the call only logs a warning. It shows the cost of string-named entries (see section 5).

## 3. Rust API

### 3.1 Make auth optional on `Server`
**Status:** Decided

**Problem:** `Server<U: AuthUser>` (`boutique/src/server.rs`) requires `AuthState`, only for `clear_legacy_cookies` (`boutique/src/middleware.rs`), which removes showroom's old `jwt`/`refresh` cookies.

**Change:** `Server::new()` takes no arguments; `.auth(state)` is optional. Remove `clear_legacy_cookies` and `cookies::LEGACY` from boutique. Showroom adds its own layer if it still needs one.

**Done when:** a `Server` without auth serves routes; `cross_origin_protection` and `https_redirect` still apply.

### 3.2 Let `Server` own the asset manifest
**Status:** Decided

**Problem:** `assets::manifest::init` sets global state. If it is not called, `Head::entry("site")` logs with `eprintln` and silently drops the script. Both apps define identical `BUILD_ROUTE = "/build"` and `BUILD_DIR = "public/build"`.

**Change:** `Server::assets(route, dir)` initialises the manifest and serves the directory, defaulting to `/build` and `public/build`. Calling `Head::entry` before initialisation panics in debug builds.

**Later:** generate typed entry handles at build time so a missing entry is a compile error (depends on 7.1).

### 3.3 Fix `AuthenticatedUser` for htmx requests
**Status:** Decided

**Problem:** on a missing session, the extractor (`boutique/src/extractors.rs`) returns a 303 to `login_url`. htmx follows it inside fetch and swaps the login page into the target element.

**Change:** when the request has `HX-Request`, return `htmx::redirect(login_url)` (an `HX-Redirect` header) instead.

**Done when:** the rejection returns `HX-Redirect` for htmx requests and a 303 otherwise.

### 3.4 Ship the sea-orm session store
**Status:** Decided

**Problem:** `src/auth.rs` is byte-identical in crossword and showroom (a sea-orm `AuthStore` implementation), and the users/sessions migrations were copied too.

**Change:** behind the existing `sea-orm` feature, provide the store generically over the app's user and session entities, plus a migration for the sessions table. Use showroom's `showroom-web/src/auth.rs` as the reference implementation.

**Done when:** showroom's `auth.rs` can be replaced by the boutique store (migration is a separate task).

### 3.5 Upstream error pages
**Status:** Decided

**Problem:** showroom's `showroom-web/src/services/error_page.rs` turns error responses into full pages, but it buffers the body and sniffs for `<!doctype`.

**Change:**
1. `AppError::into_response` attaches a response extension, `ErrorReport`, carrying the status and the messages: a list of `(Option<field>, message)`, so field errors and general messages share one shape. The body stays as today (the htmx fragment), because `into_response` cannot see the request.
2. `Server::error_page(render)` adds a middleware that sees the request headers. For requests without `HX-Request` that accept `text/html`, it replaces the body with `render(&ErrorReport)`, keeping the status and other headers.
3. Error responses that did not come from `AppError` (plain status codes, extractor rejections) get an `ErrorReport` built from the status alone.
4. The middleware never inspects the body.

**Done when:** showroom's `error_page.rs` can be replaced by `Server::error_page`; htmx requests, non-htmx HTML requests and non-HTML `Accept` requests each get the right body.

### 3.6 Non-htmx form errors
**Status:** Decided. Implement after 3.5.

**Problem:** `AppError::Validation`, `AppError::Fields` and `AppError::Message` (`boutique/src/error.rs`) always return `hx-partial` fragments, so a form posted without htmx gets a bare fragment back.

**Change:** non-htmx requests get a full page through 3.5 instead of the fragment.
- Status `422 Unprocessable Entity`, as today.
- The `ErrorReport` carries each field error with its field name and message, and `Message` as a general message.
- The page passed to `Server::error_page` lists the messages and links back to the form (`Referer`, falling back to `/`).

Out of scope: re-rendering the original form with submitted values (Post/Redirect/Get with flash state). Revisit if the error page proves too blunt.

**Done when:** a failing form posted without `HX-Request` receives a full page containing each validation message; the same request with `HX-Request` still receives the `hx-partial` fragments.

### 3.7 Add a prelude
**Status:** Decided

**Problem:** `boutique/src/views/base.rs` and `boutique/src/htmx/fragments.rs` use upstream `maud::html!`, while apps use the fork via `boutique::html`. Apps also depend on axum, maud and validator directly despite the re-exports.

**Change:** add `boutique::prelude` exporting `html`, `Markup`, `PreEscaped`, `component`, `Attrs`, `AppError`, `AppResult` and `AuthenticatedUser`. Switch boutique internals to the fork.

### 3.8 Use `tracing`
**Status:** Decided

Replace `eprintln!` in `boutique/src/error.rs`, `boutique/src/form.rs` and `boutique/src/assets/manifest.rs` with `tracing`. Add `tower_http::trace::TraceLayer` to `Server`.

### 3.9 Version the htmx path
**Status:** Decided

`HTMX_PATH` (`boutique/src/assets/mod.rs`) is a fixed `/boutique/htmx.js` served with `max-age=86400`, so an upgrade can serve a stale file. Include a content hash (computed at build time) in the path and serve with `Cache-Control: public, max-age=31536000, immutable`.

### 3.10 Smaller consistency fixes
**Status:** Decided

- `Head::class(String)` and `Metadata::website(&str)` / `Metadata::article(&str, …)` take `impl Into<String>`.
- `base` hardcodes `lang="en"`; add `Head::lang`.
- `Head::preload_font` hardcodes `font/woff2`; infer the type from the extension.
- `AuthState` has `new`, `with_config` and `.config()`; keep `new` plus `.config()`.
- `session::issue` deletes expired sessions on every login; move to a periodic task the app can spawn.

### 3.11 Replace `jsonwebtoken` in password reset
**Status:** Decided

**Problem:** `boutique/src/reset.rs` is the only user of `jsonwebtoken`. It already computes an HMAC fingerprint of the password hash with `hmac` and `sha2`, so the JWT layer only adds a dependency and a fallible API (`token_for` returns `Result<String, jsonwebtoken::errors::Error>`).

**Token format:** `{hex(user_id)}.{expires_unix}.{hex(mac)}`, where `mac = HMAC-SHA256(secret, "password_reset" \0 user_id \0 expires_unix \0 password_hash)`.
- Hex encoding keeps the token URL-safe for any user id, matching how `session.rs` encodes tokens.
- Including the current password hash in the MAC keeps today's property: a token stops working once the password changes, so each link is single-use.
- The `"password_reset"` prefix separates these MACs from any other use of the secret.

**Change:**
1. `token_for` returns `String` (it can no longer fail).
2. `verify_token` parses the three parts, rejects expired tokens, loads the user by id, recomputes the MAC with the user's current password hash, and compares with `Mac::verify_slice` (constant time). Any failure is `ResetError::InvalidToken`.
3. `request` and `complete` keep their signatures.
4. Remove `jsonwebtoken` from `boutique/Cargo.toml`, and the `Claims` struct and `fingerprint` function from `reset.rs`.

Outstanding JWT reset links stop working on deploy; they expire within `reset_ttl_hours` (1 hour by default) anyway.

**App migration (separate task):** `showroom-web/src/handlers/dashboard/settings.rs` matches on the `Result` from `token_for`; it becomes a plain value.

**Test:** this is security code, so it gets unit tests: a valid token, an expired token, a tampered user id, a tampered MAC, and a token issued before a password change. Each test must fail if the corresponding check in `verify_token` is removed.

**Done when:** those tests pass and `jsonwebtoken` is gone from `Cargo.lock`'s dependency tree for `boutique`.

## 4. CSS

Core CSS lives in `components/src/style/`; component CSS in `components/src/<name>/index.css`.

### 4.1 Cascade layers
**Status:** Decided

- Every entry file (`index.css`, `components.css`, `elements.css`, `prose.css`, `feedback.css`) starts with the order statement `@layer bq.reset, bq.tokens, bq.base, bq.elements, bq.components, app;`, so the order holds whichever is imported first.
- Each file wraps its own rules in its layer (`@layer bq.reset { … }`, `@layer bq.components { … }`), rather than relying on `@import … layer(…)`, so layering survives any bundler.
- Document in the readme that app CSS goes in `@layer app` or stays unlayered; either beats every boutique layer.

### 4.2 Breakpoints
**Status:** Later (depends on 7.2)

`--xs`…`--xl` in `tokens.css` cannot be used in `@media` and mix `em` and `px`. Replace with `@custom-media` once lightningcss processes boutique CSS. Until then, remove them or leave them unchanged; do not add new uses.

### 4.3 Tokens
**Status:** Decided

- Spacing scale `--space-1`…`--space-8` and a fluid type scale using `clamp()`.
- Radius, shadow, duration/easing, z-index and focus-ring tokens.
- `--color-on-primary`, `--color-on-error`, `--color-success`.
- `color-scheme: light dark` and `light-dark()` in `roles.css`; `data-theme` overrides keep working.

### 4.4 Base defaults
**Status:** Decided

`:focus-visible` ring; `accent-color: var(--color-primary)`; `text-wrap: balance` on headings and `pretty` on paragraphs; `scrollbar-gutter: stable`; `prefers-reduced-motion`; `interpolate-size: allow-keywords`; `field-sizing: content` on textareas.

### 4.5 Layout primitives
**Status:** Decided

`.flow` and `.article-layout` (from `showroom-web/resources/css/app.css`), plus `.stack`, `.cluster`, `.center`, `.sidebar`, `.switcher` and an auto-fit `.auto-grid`.

### 4.6 Component styles
**Status:** Decided

Select, checkbox/radio, fieldset/legend, `dialog` with `::backdrop`, `[popover]`, prose (reference: `showroom-web/resources/css/prose.css`), toast/flash, `.skeleton`.

### 4.7 Fonts
**Status:** Later (with 8)

A convention for `@font-face` that `cargo bq fonts` generates. Reference: `crossword-dot-blue/scripts/fonts.sh`.

## 5. Default file structure

**Status:** Decided

| Topic | crossword | showroom | Default |
|---|---|---|---|
| Module style | `handlers.rs` + `handlers/` | `mod.rs` | `mod.rs`, since folders also hold `.css`/`.ts` |
| Static files | `public/assets` | `resources/static` | `public/` |
| View CSS | colocated, one entry per view | `resources/css/views/` | colocated |
| Package manager | bun | npm and bun lockfiles | bun |
| Routing | `router.rs` | `router.rs` + `routers/` | `router.rs`, split into `routers/` when needed |

```
my-app/
├─ Cargo.toml
├─ package.json
├─ .bun-version
├─ .env.example
├─ .cargo/config.toml.example
├─ Procfile
├─ migration/
│  └─ src/
│     ├─ lib.rs
│     ├─ main.rs
│     └─ m00000000_000001_users_and_sessions.rs
└─ web/
   ├─ Cargo.toml
   ├─ build.rs
   ├─ package.json
   ├─ tsconfig.json
   ├─ vite.config.ts
   ├─ bindings/
   │  ├─ setups.ts
   │  ├─ registry.ts
   │  └─ boutique/
   ├─ public/
   │  ├─ favicon.ico
   │  ├─ robots.txt
   │  └─ build/
   ├─ resources/
   │  ├─ js/site.ts
   │  ├─ css/app.css
   │  ├─ css/theme.css
   │  └─ fonts/
   └─ src/
      ├─ main.rs
      ├─ config.rs
      ├─ state.rs
      ├─ router.rs
      ├─ handlers/
      ├─ models/
      ├─ services/
      ├─ views/
      │  ├─ mod.rs
      │  ├─ layouts/mod.rs
      │  └─ home/
      │     ├─ mod.rs
      │     └─ home.css
      └─ components/
         ├─ mod.rs
         └─ counter/
            ├─ mod.rs
            ├─ index.ts
            └─ index.css
```

- `bindings/` is generated by `build.rs` and committed (see 9.4).
- `public/build/` is gitignored.
- All `src/**/*.css` is globbed into the `site` entry.
- Per-page CSS entries (crossword's approach: each view's CSS is its own Vite entry, requested with `head(..).css("edit")`) were considered and rejected for now. String names fail silently (crossword's `.entry("editor")` refers to an entry that does not exist), pages must list the CSS of every partial they render (`lander` asks for `"listing"`), and the saving is small (about 1,100 lines of view CSS per app). Revisit with typed entry handles (3.2) if per-page CSS or JS becomes worthwhile, for example to keep dashboard CSS off showroom's public pages. A `#[view]` macro was considered for the same reason and deferred: with one bundle it adds nothing a glob does not.
- Update the file structure section of boutique's `readme.md` to match.

### Bun
- Bun is the only JS package manager and script runner, in apps, the template and boutique. Commands: `bun install`, `bun add`, `bun run <script>`.
- Use the text lockfile `bun.lock`, not the binary `bun.lockb`.
- Pin the version in `.bun-version` at the repo root, as crossword already does.
- Boutique: delete `package-lock.json`. Boutique's `package.json` has no dependencies or scripts, so no lockfile is needed until it does.
- App migration (separate task): showroom removes `package-lock.json` (root and `showroom-web/`) and `bun.lockb`, and generates `bun.lock`. Its scripts call `npm run`; change them to `bun run`. Crossword converts `bun.lockb` to `bun.lock`.

## 6. Template

**Status:** Later (after phases 1–3)

Keep the template in this repo and test it in CI (generate, then `bun install`, `cargo check` and `bun run typecheck`). Contents:

- root `Cargo.toml` with `[workspace.dependencies]` (1.1)
- `main.rs`, `config.rs`, `state.rs` (with `FromRef` for `AuthState` and a `Viewer` alias), `router.rs`
- `views/layouts` with `head()` and `shell()`
- one component (`mod.rs`, `index.ts`, `index.css`) using bare `bq-setup` (2.1) and `#[default]` (2.2)
- `site.ts` importing from `bindings/boutique/` (9.4)
- users/sessions migration and the boutique sea-orm store (3.4)
- login, logout and an error page (3.5)

`main.rs` shape, using today's API (simplify as 3.1 and 3.2 land):

```rust
async fn serve(state: AppState) {
    Server::new(state.auth.clone())
        .debug(cfg!(debug_assertions))
        .file("/favicon.ico", "public/favicon.ico")
        .file("/robots.txt", "public/robots.txt")
        .static_dir(BUILD_ROUTE, BUILD_DIR)
        .serve(router::create(state.clone()), &state.config.port)
        .await;
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    manifest::init(BUILD_ROUTE, BUILD_DIR);

    let db = Database::connect(&config.database_url).await.expect("connect to database");
    Migrator::up(&db, None).await.expect("run migrations");

    let auth = AuthState::new(auth::Store::new(db.clone()), config.secret.clone());
    boutique::run(AppState { db, auth, config }, serve).await;
}
```

## 7. Build tooling

**Goal:** Vite becomes an implementation detail. "Rust only" is not a goal; bun stays for third-party packages, scripts and `tsc`.

**Context:** neither app uses Vite's dev server or HMR. Both run `vite build --watch` next to `dx serve`, so Vite only bundles, hashes and writes `.vite/manifest.json`. Vite 8 already runs on Rolldown.

### 7.1 Own the config and the manifest
**Status:** Later (with 8)

Apps do not have a `vite.config.ts`; boutique generates or supplies it, and defines its own manifest format consumed by `boutique/src/assets/manifest.rs`.

### 7.2 CSS through lightningcss
**Status:** Later

Process boutique and app CSS with lightningcss (Rust): `@custom-media` (4.2), nesting, `@import` bundling, and CSS discovery mirroring how `bq_setups` discovers setups.

### 7.3 JS bundling in Rust
**Status:** Not planned. Revisit only if 7.1 and 7.2 leave a clear gap. The `rolldown` crate's API is unstable and expensive to compile; never put a bundler in an app's `build.rs`.

## 8. CLI (`cargo bq`)

**Status:** Later. Build `link` and `upgrade` first, then `dev` and `build`, then `new` and `gen`.

A `cargo-bq` binary in this workspace, installed with `cargo install`.

| Command | Replaces |
|---|---|
| `cargo bq link ../boutique` / `unlink` | hand-editing `.cargo/config.toml` |
| `cargo bq upgrade [version]` | editing `[workspace.dependencies]` |
| `cargo bq dev` | `concurrently` + `vite build --watch` + `dx serve` |
| `cargo bq build` | `npm run build` then `cargo build --release`, and the manifest panic in `build.rs` |
| `cargo bq check` | `setups:check` and `bindings:check` npm scripts |
| `cargo bq new <name>` | copying an existing project (uses the template from 6) |
| `cargo bq gen component/view/migration` | hand-writing the three component files; wraps `sea-orm-cli` |
| `cargo bq fonts` | `crossword-dot-blue/scripts/fonts.sh`, plus `@font-face` generation (4.7) |

Constraints:
- It orchestrates `cargo`, `dx`, the bundler and `sea-orm-cli`; it does not replace them.
- Everything it generates is plain committed files. Nothing only works through the CLI.
- Codegen that must stay in sync with compilation (setups, `bindings/boutique/`) stays in `build.rs`.
- `cargo bq dev` checks that the installed `dx` matches the `dioxus-devtools` version in `boutique/Cargo.toml` (currently `0.7.3`).
- No `cargo bq add`; `bun add` already handles third-party packages.

## 9. Publishing

**Goal:** publishable now (dry runs pass); publish after phases 1–3.

### 9.1 What publishing changes
Only section 1 is affected; sections 2–8 are not.

| Item | Effect |
|---|---|
| 1.1 | Git revs become semver versions; `cargo update` works. |
| 1.2 | `[patch.crates-io]` replaces the git patch. |
| 1.3 | Already solved by 9.4. |
| Crossword's old version | Not solved; on 0.x every minor version can break. |

### 9.2 Prerequisites
**Status:** Decided. Do these in phase 1 and verify with `cargo publish --dry-run --workspace` (or per crate in order).

- `Generator::boutique()` must not read `CARGO_MANIFEST_DIR/../components/src`; that path does not exist for a published crate. Solved by 9.4.
- Crates publish in order: `bq_html` → `bq_macros` → `bq_components` → `bq_setups` → `boutique`. Path dependencies need `version =`.
- Remove `publish = false` from `[workspace.package]`. Add `license`, `description` and `repository` to every crate. `bq_html` keeps maud's license files.
- `boutique`, `bq_html`, `bq_macros`, `bq_components` and `bq_setups` are free on crates.io (checked 2026-10-15). `boutique` is taken on npm, which 9.4 makes irrelevant.
- Versions can be yanked but not deleted.

### 9.3 Releasing
**Status:** Later

- Until publishing, apps use git tags (`tag = "v0.3.0"`) instead of revs.
- Publish after 2.1, 2.2, 3.1 and 9.4 have landed.

### 9.4 Boutique's JS and CSS ship inside the crates
**Status:** Decided

**Decision:** boutique's TypeScript and CSS are distributed through Cargo, not the npm registry. `bq_setups` writes them into the app's `bindings/boutique/` from `build.rs`. Boutique's `package.json` stays private and only runs boutique's own JS tests.

**Why:** the JS version always matches the crate version; no `@bq` alias, `node_modules/boutique` or npm name; works with any bundler and with `tsc`. It extends what the generator already does for `bindings/setups.ts` and `registry.ts`.

**Considered:**
- Publishing to npm as well (Tauri, Turbo, Phoenix): two releases, versions kept in sync by hand, scoped name needed.
- Aliasing the bundler to Cargo's source folder via `cargo metadata` (like Phoenix's `NODE_PATH` to `deps/`): no copies, but the path must be generated into every bundler and tsconfig. Revisit with 7.1.
- Serving prebuilt JS from Rust (like `htmx.js` today, Livewire, Dioxus): app setups import the runtime's types and CSS needs bundling, so this does not fit.

**Implementation:**
1. Move all boutique CSS into the crate under `components/src/`: `style/reset.css`, `tokens.css`, `roles.css` and `components/base.css` go to `components/src/style/`; component CSS per 2.4. Keeping everything under `components/src/` means relative `@import`s (e.g. `style/components.css` importing `../button/index.css`) resolve the same way in the source tree and in the written `bindings/boutique/`.
2. Expose boutique's files from `bq_components` with `include_str!`, as a static list of `(relative path, contents)`. This covers `components/src/setups.ts`, every component `index.ts` and `index.css`, the `style/` files, and the component Rust sources that `bq_setups` analyses.
3. `bq_setups` depends on `bq_components` for that list. `Generator::boutique()` analyses the embedded Rust sources instead of reading `../components/src`, and writes the TS and CSS to `<out>/boutique/`, mirroring `components/src` (so `components/src/style/index.css` becomes `<out>/boutique/style/index.css` and `input/index.ts` still imports `../setups`). Use `write_if_changed`.
4. Generated imports change from `@bq/components/src/…` to relative paths: `registry.ts` and `setups.ts` import `./boutique/setups`, and boutique component setups `./boutique/input/index`. Remove the `runtime` field default of `"@bq/components/src/setups"`.
5. Provide CSS entries: `bindings/boutique/style/index.css` (core layers, in order per 4.1), `components.css` (UI kit, per 2.5), and the opt-in `elements.css`, `prose.css` and `feedback.css`.

Resulting `site.ts`:

```ts
import "../../bindings/boutique/style/index.css";
import "../../bindings/boutique/style/components.css";
import "../css/theme.css";
import "../css/app.css";
import { defineSetups } from "../../bindings/boutique/setups";
import { registry } from "../../bindings/registry";

import.meta.glob("../../src/**/*.css", { eager: true });

defineSetups(registry);
```

**Committing generated files:** `bindings/` (including `bindings/boutique/`) is committed. The production build runs the JS build (`bun run build`) before `cargo build --release`, so the files must exist before Cargo runs. This matches how `bindings/` is handled today (`setups:check` diffs it).

**Third-party JS:** unchanged. Apps install packages like prosemirror with `bun add` into their own `package.json` and import them normally from component `index.ts` files.

**Constraint on boutique's JS:** boutique's TS and CSS must not import npm packages, since nothing installs them. Today `components/src/setups.ts` has no imports and `input/index.ts` only imports `../setups`. If a future boutique component needs an npm package, add a peer-requirement check to the generator: read the app's `package.json` and fail with the exact `bun add <package>@<range>` command. Do not vendor third-party packages into the crate.

**App migration (separate task):** delete the `@bq` alias from `vite.config.ts` and `tsconfig.json`, remove `boutique` from `package.json`, update `site.ts` as above, delete copied component CSS that is now upstream.

**Done when:**
- `cargo publish --dry-run` passes for `bq_components` and `bq_setups`.
- The restored `setups/src/tests.rs` passes; only expected import paths in generated `registry.ts`/`setups.ts` may change, from `@bq/components/src/…` to relative paths.
- Showroom builds with `bun run build` and `bun run typecheck` after migration, without any `@bq` alias.

## 10. Code health

From an audit on 2026-10-16. Goal: same behaviour (except the listed bug fixes), clearer code. Each item says which crate it touches. Bugs get exactly one regression test each (**Test:**); everything else is verified by the restored tests, `cargo clippy --workspace --all-features`, and building showroom and crossword.

### 10.0 Formatting
`rustfmt.toml` with `max_width = 140` at the root; format the whole workspace in one commit that changes nothing else.

### 10.1 Bugs

| # | Crate | Bug | Fix |
|---|---|---|---|
| a | `bq_macros` | `#[component]` breaks on generic components: elided lifetimes are placed after named params (bon puts them first), lifetimes inside `fn(&str)`/`Fn(..)` are counted though bon skips them, `where` clauses are dropped from the `Render` impl. | Before emitting `#[bon::builder]`, name every elided input lifetime explicitly in `func.sig`; build the `Render` impl from `split_for_impl()` + the where clause; delete `count_elided_lifetimes`. **Test:** one generic component with a `where` clause and a `&str` param. |
| b | `bq_html`, `bq_components` | Three attribute-merging paths disagree: `attrs_merged` (compile-time strings), `component_attrs` (builds `Attrs`), and `Attrs` at runtime. `b bq-ref=[Some("own")] (..spread)` loses the spread value; `b class (..spread)` drops the bare `class`; `class=""` differs between paths. | Plain elements keep maud's path. Any element with a spread, a repeated name, or `class=` next to `.class` builds an `Attrs` (id, statics in source order, `extend_except(spread, skip)`) and writes it with one `Attrs::write_to`. Delete `attrs_merged`, `Rest`, `class_part`, the generator's `merges`, `__value`, `write_attrs`. **Test:** the three divergences. |
| c | `bq_setups` | Orphan `bq-ref`/`bq-on` errors found while walking a function *in context* (followed helper or component) are discarded (`walk.rs` drops deferred orphans whose function is in `followed`); a helper used inside and outside a setup is only checked inside. Wrong refs pass the build. | Orphans found with a non-empty scope or frame are errors immediately; only context-free walks defer. Superseded by 10.3's call-graph walk. **Test:** wrong-prefix ref in a helper called inside a setup. |
| d | `bq_setups` | An `html!` that fails to parse is dropped silently, followed by a misleading "not reached" warning. | Record a diagnostic ("html! at … could not be parsed; its bq-* attributes are ignored"); never emit "not reached" for a function with unparsed blocks. |
| e | `bq_setups` | False errors for `let x: Markup = html!{}` (only `Pat::Ident` matched), `impl` methods used as helpers (never indexed), and `#[cfg(test)]` code (analysed). Functions are keyed by bare name, so `a::row`/`b::row` and `other_crate::row` resolve wrongly. | Match `Pat::Type`; index impl methods; skip `#[cfg(test)]` items; key functions by (file, module path, name) and use path qualifiers when present. **Test:** one case covering typed `let` and an impl-method helper. |
| f | `bq_setups` | Generated TS is not escaped (`@element("x\"y")` emits `Setup<"x"y">`). | `ts_str()` for every string written into TS. |
| g | `boutique` | `htmx::redirect` panics on an invalid header value (reachable from the `AuthenticatedUser` extractor through `login_url`); `Server::serve` `expect`s on bind/serve; a missing manifest panics inside `router()` in release and behaves differently in debug. | `redirect` logs and falls back; `serve(app, addr: impl ToSocketAddrs) -> io::Result<()>`; `router()` returns `Result<Router, ManifestError>` (or `serve` reports it); one manifest behaviour chosen by `Server::debug`. |

### 10.2 Shared conventions and dead code (`bq_html`, `bq_macros`, `bq_components`)
- **Dead typed-setup pipeline.** Since 2.1, nothing implements `Setup` by hand or writes `bq-setup=(…)`. Put the setup *name* in the marker and have the bare-setup pass emit a literal; delete the `Setup` trait (or make it `doc(hidden)` if still needed), `setup_attrs`, `setup_value` and the `enum __BqSetup` injection. `html!` rejects `bq-setup` with a value, with a spanned error.
- **Marker protocol.** Replace the token-level `has_bare` heuristic with AST-based detection shared by `#[component]` and `bq_setups` (`bq_html::setup::has_bare_setup`). Use a marker no user would write. Mark every `html!` in a component body.
- **`bq_html::conventions`**: one home for `HTML_MACROS` (the accepted macro names; `is_html_macro(&syn::Path)`), prop names `attrs`/`children`, finish fn `build`, `maybe_` prefix, `merges()`, `kebab()`, setup/TS naming (`setup_name`, `ts_type_name`, `ts_export_name`, `component_alias`), the `bq-setup`/`bq-ref`/`bq-on:` constants, and `ComponentOptions`. Every crate uses it; delete the copies (`on::kebab`, `"__BqSetup"` literals, last-segment `html` checks).
- **`Attrs` internals.** Move `__text`, `__flag`, `__class`, `__extend*` into `__private` as free functions (or a sealed trait); take `PreEscaped<String>` where the value must already be escaped.
- **Public surface.** `bq_html::ast` and `bq_components::FILES` become `#[doc(hidden)]`; `pub use bon` is `#[doc(hidden)]`; `macros/Cargo.toml` description no longer mentions `setup!`. `#[component]` emits `#[allow(clippy::needless_pass_by_value)]`. Box large `ElementName` variants.
- **Fork isolation.** Move boutique's additions out of the maud files into `ast/ext.rs` and `generate/ext.rs` (or similar) so upstream files only carry small hooks; record the upstream version (`maud_macros 0.27.0`) in the crate.

### 10.3 `bq_setups` structure
- **Pre-parsed index.** One `html.rs` with `parse` (keeps diagnostics), a single `HtmlFinder` visitor (tokens, flags, binding), and `Flags`, shared by index and walk. Every `html!` is parsed once, nested ones included.
- **Call-graph walk.** Build callers/callees in the index; walk once from roots (functions with no callers, plus one per cycle) with one `visiting` guard. Removes the probe pass, `reached`/`followed`, most orphan triage and the exponential re-walking.
- **Diagnostics.** `Loc { file, line, column }`, `Diagnostic { level, loc, message, via }`, a sorted, deduplicating sink. `Generator::try_write() -> Result<Report, Failure>`; `write()` prints every diagnostic and exits non-zero instead of panicking, keeping warnings. Parse errors carry locations.
- **Model.** One `TagSet` (replaces three tag representations); one declaration type; `enum Root { Dir, Embedded }`; key setups by name and derive TS names at emission; named structs instead of tuples.
- **Emission.** `write!`-based, with `ts_str` escaping (10.1f).
- **Embedded files crate.** Move the `build.rs`-generated `FILES` into a tiny dependency-free crate (`bq_assets`), used by `bq_components` and `bq_setups`, so the generator no longer compiles `bq_components`, `bq_macros`, `bon` and `maud` for the host.
- **Layout:** `lib.rs` (Generator), `sources.rs`, `diagnostics.rs`, `html.rs`, `index/{mod,collect,graph}.rs`, `attrs.rs`, `walk/{mod,scope,frame}.rs`, `model.rs`, `emit/{types,registry,ts}.rs`, `fs.rs`.

### 10.4 `boutique` crate
- **Errors.** `std::error::Error` + `Debug` + `Display` on every error type. `AuthStore::Error: Error + Send + Sync + 'static`; `StoreError(Box<dyn Error + Send + Sync>)` with `source()`. `From<LoginError | ResetError | UploadError | cloudinary::Error> for AppError` (infrastructure → `Internal`, domain → default 422 message). Wrap argon2 errors in `password::HashError`. One `ErrorMessage { field: Option<Cow<'static, str>>, text }` shape for `AppError::Fields`, validation, `ErrorReport` and the fragments. `From<DbErr>` moves into the sea-orm module.
- **Manifest.** `static MANIFEST: OnceLock<Manifest>` with `enum Manifest { Live { route, path }, Loaded { route, entries } }`, chosen by `Server::debug`, not `cfg!`. One lookup per `Head::entry`. `AuthConfig::secure_cookies` default no longer uses `cfg!`.
- **Layout and surface.** Modules: `error/{mod,report,fragments}`, `server/{mod,files,layers,error_page,assets,dev}`, `auth/{mod,store,session,reset,password,extract,sea_orm/{store,migration}}`, `htmx`, `views/`, `forms`, `ids`, `cloudinary`. Root re-exports are canonical and modules private where covered; keep `pub use auth::{session, reset, password}` and the sea-orm store path working. Delete `htmx::oob_only`, `fragments::from_errors`, `AuthUser::email`. Internal helpers (`cookies::*`, `cross_origin_protection`, manifest functions) become `pub(crate)`. `AuthState` fields private.
- **Config.** Private fields and setters on `AuthConfig`, `Head`, `Metadata`; durations as `std::time::Duration`, converted at the edges. `#[must_use]` on builder types.
- **Naming.** `Head::css` → `entry_css`; `Metadata::with_image`/`with_modified_time` → `image`/`modified_time`; `session::hash` → `token_hash`; `base(head, …)`; `Server::router(app)`; consistent `_with` naming for cache variants.
- **Internals.** `Mount { route, path, cache, kind }` instead of parallel vectors; `header_str` helper; split `error_page` middleware into small functions; header-name constants; one private `hex` module; `run` → `server/dev.rs`; structured `tracing` fields; log the swallowed error in `session::revoke`; same rejection type for both `AuthenticatedUser` extractors.
- **sea-orm store.** Table/column names in `create_sessions_table` come from the entities; require `DateTimeWithTimeZone` (drop the silent fallback).
- **Forms and cloudinary.** Rename cloudinary's private `UploadError` to `ApiError`; `form::UploadError::Unreadable(MultipartError)`; `upload(bytes: impl Into<reqwest::Body>)` with a client stored in `Cloudinary`; `blank_to_none(&str)`.
- **Dependencies.** Drop `rand_core` (use `argon2`'s `std` feature), `validator`'s `derive` feature, `uuid` (use the token generator). Re-export only crates whose types appear in boutique's public API (`axum`, `maud`, `validator`, `chrono`); put `CookieJar` in the root/prelude.
- **Tests.** Small table tests for the pure security functions `is_cross_origin` and `has_dotfile`; move the reset tests' mock store into `#[cfg(test)] mod testing`.

### 10.5 `#[component]` structure (`bq_macros`)
Split `macros/src/lib.rs` into `crate_path.rs`, `attrs.rs`, `component/{mod,params,setup,render,docs}.rs`. Parse each parameter once into `Param { name, ty, role, default: None | Default | Expr | Implicit }` and generate both the bon attributes and the docs from it (no round trip through `#[builder]` attributes). Spanned errors for non-identifier patterns, receivers, `#[positional]` with `#[default]`, and a non-`Markup` return type. Docs only on the function; drop the Type column and the `tidy` string hack (print defaults as written). Stop accepting raw `#[builder(...)]` on component parameters.

### 10.6 Auth traits (breaking)
`trait AuthStore { type User: AuthUser; type Error; … }`, `AuthState<S: AuthStore>`, `AuthUser { id, password_hash }`. `UserEntity` no longer duplicates `password_hash` if avoidable. Migrate showroom and crossword (`models/user.rs`, `state.rs`).
