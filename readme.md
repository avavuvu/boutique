# Boutique
Boutique is a framework for integrating server-side rendered Rust apps with Javascript.

## File structure
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

## UI kit
`Button`, `Input` and `Textarea`, with the `password-toggle` setup, are behind boutique's `ui` feature, which is on by default. Depend on `boutique` with `default-features = false` to leave them out.

`bq_setups` writes the kit's TypeScript and CSS to `bindings/boutique/` either way. Registry entries load lazily, so unused setups cost nothing, and kit CSS only reaches the bundle if `site.ts` imports `bindings/boutique/style/components.css`.

### Field errors
`AppError::Fields` and validation errors replace the element with id `<field>-error`, where `<field>` is the form field's name, using an htmx partial (`hx-target="#<field>-error"`, `hx-swap="outerHTML"`). `Input` and `Textarea` render `p.error id="<id>-error"`, and their id defaults to the field name.

This is a core contract, with or without `ui`. A form built without the kit must render an element with id `<field>-error` for every field that can fail, for example:

```rust
html! {
    input #email name="email" aria-describedby="email-error";
    p.error #email-error aria-live="polite" {}
}
```

If you give `Input` an `id` that differs from its name, its error element follows the id and no longer matches.

## CSS
Boutique's CSS uses cascade layers, in this order:

```css
@layer bq.reset, bq.tokens, bq.base, bq.elements, bq.components, app;
```

Import `style/index.css` first, then any of the optional entries:

| Entry | Contents | Layer |
|---|---|---|
| `style/index.css` | Core: reset, tokens, roles and base defaults | `bq.reset`, `bq.tokens`, `bq.base` |
| `style/layout.css` | Layout primitives: `.flow`, `.article-layout`, `.stack`, `.cluster`, `.center`, `.sidebar`, `.switcher`, `.auto-grid` | `bq.base` |
| `style/components.css` | The UI kit's look: `.button`, `.input-component` | `bq.components` |
| `style/elements.css` | Global element styles: `select`, checkbox/radio, `fieldset`, `legend`, `dialog`, `[popover]` | `bq.elements` |
| `style/prose.css` | `.prose` for long-form text | `bq.components` |
| `style/feedback.css` | `.toasts`, `.toast`, `.flash`, `.skeleton` | `bq.components` |

Only `components.css` and `elements.css` style anything without a class you add yourself: `components.css` styles markup the kit renders, and `elements.css` styles bare elements across the whole app. `bq.elements` sits below `bq.components`, so kit classes win over element defaults.

Put app CSS in `@layer app` or leave it unlayered. Either beats every boutique layer, regardless of specificity, so restyle the kit by overriding tokens or rules rather than copying files.

## Setups in client-rendered DOM
Setups attach to any `bq-setup` element inserted into the document, not only server-rendered ones. That includes elements inside `contenteditable` and DOM owned by an editor, such as ProseMirror node views.

Whoever creates the element can `provide` on its root before inserting it, and the root's own setup can `inject` the value. Descendant setups see it too.

```ts
import { context, provide } from "../../bindings/boutique/setups";

export const music = context<MusicHandle>("music");

class MusicView implements NodeView {
    dom: HTMLElement;

    constructor(node: Node, view: EditorView, getPos: () => number | undefined) {
        const template = document.querySelector<HTMLTemplateElement>("#music-editor")!;
        this.dom = template.content.firstElementChild!.cloneNode(true) as HTMLElement;
        provide(this.dom, music, { node, view, getPos });
    }
}
```

The `music-editor` setup then calls `await inject(element, music)`.

Attachment is asynchronous, and lazily loaded setups wait for their module, so the creator must not assume handlers exist yet. Removing the element runs the setup's `disconnect` and aborts its `signal`.

The editor may rebuild its DOM at any time, and the setup with it. Keep long-lived state in the creator, such as the node view, not in the setup.
