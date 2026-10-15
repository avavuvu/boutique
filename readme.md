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

## CSS
Boutique's CSS uses cascade layers, in this order:

```css
@layer bq.reset, bq.tokens, bq.base, bq.components, app;
```

- `style/index.css` is the core: reset, tokens, roles, base defaults and layout primitives (`.flow`, `.article-layout`, `.stack`, `.cluster`, `.center`, `.sidebar`, `.switcher`, `.auto-grid`).
- `style/components.css` is the UI kit's look, in `bq.components`. Import it only if you want it.

Put app CSS in `@layer app` or leave it unlayered. Either beats every boutique layer, regardless of specificity, so restyle the kit by overriding tokens or rules rather than copying files.
