# Boutique
Boutique is a framework for integrating server-side rendered Rust apps with Javascript.

## File structure
├─ public/
│  └─ robots.txt
└─ src/
   ├─ components/
   │  └─ button/
   │     ├─ index.css
   │     ├─ index.ts
   │     └─ mod.rs
   ├─ handlers/
   ├─ models/
   └─ views/
      └─ lander/
         ├─ lander.css
         └─ lander.rs

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
@layer bq.reset, bq.tokens, bq.base, bq.components, app;
```

- `style/index.css` is the core: reset, tokens, roles, base defaults and layout primitives (`.flow`, `.article-layout`, `.stack`, `.cluster`, `.center`, `.sidebar`, `.switcher`, `.auto-grid`).
- `style/components.css` is the UI kit's look, in `bq.components`. Import it only if you want it.

Put app CSS in `@layer app` or leave it unlayered. Either beats every boutique layer, regardless of specificity, so restyle the kit by overriding tokens or rules rather than copying files.
