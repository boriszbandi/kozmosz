# Kozmosz – project rules

Hungarian space-community website. Rust: Leptos 0.8 (islands mode) + axum, built with cargo-leptos.
Top priority: fastest possible load. These rules override any design skill or DESIGN.md
(design-taste-frontend, design-md, etc.) that suggests otherwise.

## Stack constraints
- Server-rendered HTML. Zero JavaScript by default. Interactivity only via `#[island]`, and every page
  that contains an island must be listed in `ISLAND_PAGES` (src/app.rs), otherwise it gets no WASM.
- Plain CSS in `style/main.css` (Lightning CSS minifies it, inlined into the HTML in production).
  No Tailwind, no Sass, no CSS-in-JS, no npm packages.
- System font stack only (`--font`). No webfonts unless the user explicitly approves the cost.
- Icons: inline SVG or files under `public/img/`. No icon libraries, no CDNs.
- No third-party requests at runtime (no picsum, no CDN logos, no analytics) unless the user asks.
- Motion: CSS only (transform/opacity, `@view-transition`), always behind `prefers-reduced-motion`.
  Content must never start hidden (no opacity:0 waiting for JS).
- Images: AVIF/WebP/JPEG via `image-set()` or `<picture>`, explicit width/height, no metadata (strip EXIF/GPS).
  `/img/*` is cached immutable for a year: a changed image gets a new file name.

## Pages
- New page: component in `src/pages.rs`, `<Route>` in `App` (src/app.rs).
- If its HTML is identical for every request, add the path to `STATIC_PAGES`: it is rendered once at
  startup and served from memory, precompressed (src/server.rs `PageCache`).
- The 404 page is also rendered once and cached, so `NotFound` must not depend on the request path.
- UI text is Hungarian. Code, comments and identifiers are English.

## Commands
- Dev (hot reload): `LEPTOS_HASH_FILES=false cargo leptos watch` → http://127.0.0.1:3000
  (watch only hashes its first build; with hashing on, CSS edits never reach the browser)
- Release: `cargo leptos build --release --precompress` (or `cargo leptos serve --release --precompress`)
- Tests: `cargo test --features ssr`
