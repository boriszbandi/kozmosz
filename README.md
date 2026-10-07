# ![Kozmosz](public/img/kozmosz-mark.svg) Kozmosz

A Kozmosz közösség weboldala. Rust: [Leptos](https://leptos.dev) 0.8 (islands mód) + [axum](https://github.com/tokio-rs/axum), [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) builddel.

Cél: a lehető leggyorsabb betöltés.
- Szerveren renderelt HTML, alapból **nulla JavaScript**. Interaktív részek `#[island]`-ként, csak azokon az oldalakon töltődik be a WASM.
- A statikus oldalak és a 404-es oldal induláskor egyszer renderelődnek, és memóriából mennek ki brotli-11 / gzip-9 tömörítéssel, ETag-gel (304).
- A CSS production módban a HTML-be ágyazva megy ki, így nincs külön render-blokkoló kérés.
- A JS/WASM/CSS fájlnevekben hash van, az `/img` fájlok immutable cache-t kapnak. Előtömörített `.br`/`.gz` fájlok.
- Háttérkép AVIF/WebP/JPEG `image-set()`-tel, három méretben (50–150 KB az eredeti 3.4 MB helyett).
- System font stack, nincs webfont és nincs külső kérés.
- Oldalváltáskor `@view-transition` áttűnés és speculation rules prefetch, JS nélkül.

## Követelmények

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
```

## Fejlesztés

```bash
LEPTOS_HASH_FILES=false cargo leptos watch
```

PowerShellben:

```powershell
$env:LEPTOS_HASH_FILES='false'; cargo leptos watch
```

Ezután: http://127.0.0.1:3000, hot reloaddal. A `watch` csak az első buildnél hashel, ezért fejlesztés közben ki kell kapcsolni a hashelést, különben a CSS-módosítások nem jutnak el a böngészőig.

## Release build

```bash
cargo leptos build --release --precompress
```

Kimenet: `target/release/kozmosz` (szerver), `target/release/hash.txt` és `target/site/` (statikus fájlok).

### Futtatás szerveren

Másold egy könyvtárba:

```text
kozmosz        # target/release/kozmosz
hash.txt       # target/release/hash.txt (a bináris mellé!)
site/          # target/site
```

Környezeti változók:

```sh
LEPTOS_OUTPUT_NAME=kozmosz
LEPTOS_SITE_ROOT=site
LEPTOS_SITE_PKG_DIR=pkg
LEPTOS_SITE_ADDR=127.0.0.1:3000
LEPTOS_ENV=PROD
LEPTOS_HASH_FILES=true
```

Élesben tegyél elé reverse proxyt (Caddy vagy nginx) a TLS, a HTTP/2-3 és a timeoutok miatt. nginx-nél kell a `proxy_buffering off;` beállítás.

## Szerkezet

| Fájl | Mi van benne |
|------|--------------|
| `src/app.rs` | HTML shell, routing, navigáció, `STATIC_PAGES` / `ISLAND_PAGES` listák |
| `src/pages.rs` | oldalak (kezdőlap, közösség, rólunk, 404) |
| `src/server.rs` | memóriás oldal-cache, cache-fejlécek, beágyazott CSS, leállítás |
| `src/main.rs` | axum szerver összerakása |
| `style/main.css` | az összes stílus (sima CSS) |
| `public/` | favicon, képek (a site gyökerébe másolódik) |

Új oldal: komponens a `src/pages.rs`-be, `<Route>` az `App`-ba. Ha a HTML-je minden kérésre ugyanaz, az útvonal kerüljön a `STATIC_PAGES` listába is.

Kép cseréjekor **új fájlnév** kell, mert az `/img` alatti fájlokat a böngésző egy évig cache-eli.

## Teszt

```bash
cargo test --features ssr
```
