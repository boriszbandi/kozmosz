# ![Kozmosz](public/img/kozmosz-mark.svg) Kozmosz

A Kozmosz közösség weboldala. Rust: [Leptos](https://leptos.dev) 0.8 (islands mód) + [axum](https://github.com/tokio-rs/axum), [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) builddel.

Cél: a lehető leggyorsabb betöltés.
- Szerveren renderelt HTML, alapból **nulla JavaScript**. Interaktív részek `#[island]`-ként, csak azokon az oldalakon töltődik be a WASM.
- A statikus oldalak és a 404-es oldal induláskor egyszer renderelődnek, és memóriából mennek ki brotli-11 / gzip-9 tömörítéssel, ETag-gel (304).
- A CSS production módban a HTML-be ágyazva megy ki, így nincs külön render-blokkoló kérés.
- A JS/WASM/CSS fájlnevekben hash van, az `/img` fájlok immutable cache-t kapnak. Előtömörített `.br`/`.gz` fájlok.
- Képek AVIF + JPEG változatban, több szélességben, metaadat (GPS) nélkül; az első képernyőn lévő kép prioritással, a többi lustán töltődik.
- Címsorok: Urbanist (17 KB, magyar karakterekre szűkítve), szöveg: rendszerbetű. Nincs külső kérés.
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
| `content/oldalak/*.md` | az oldalak szövegei (ezeket kell szerkeszteni), képek `kep:<kulcs>` formában |
| `src/content.rs` | a Markdown-tartalom betöltése (frontmatter, `##` szekciók, galériák) |
| `src/pages/` | az oldalak elrendezése (kezdőlap, rólunk, elérhetőség, programok, előadók, projektek, 404) |
| `src/layout.rs` | fejléc, menü (mobilon JS nélküli `<details>`), lábléc, meta tagek |
| `src/site.rs` | név, cím, e-mail, közösségi linkek, menü, régi URL-ek átirányítása |
| `src/media.rs` | reszponzív `<picture>` (AVIF + JPEG), galéria, ikonok |
| `src/calendar/` | a klub Google Naptárának letöltése (10 percenként), iCal-feldolgozás, ismétlődő események, magyar dátumok |
| `src/server.rs` | memóriás oldal-cache (naptárfrissítéskor újrarenderel), átirányítások, cache-fejlécek, beágyazott CSS |
| `src/main.rs` | axum szerver összerakása |
| `style/main.css` | az összes stílus (sima CSS) |
| `tools/images.py` | képgenerálás: forrásfotókból `public/img/*` és `src/images.rs` |
| `public/` | favicon, fontok, generált képek (a site gyökerébe másolódik) |

Új oldal: Markdown a `content/oldalak/`-ba, komponens a `src/pages/`-be, `<Route>` az `App`-ba, menüpont a `src/site.rs`-be. Ha a HTML-je minden kérésre ugyanaz, az útvonal kerüljön a `STATIC_PAGES` listába is.

Új kép: a forrásfájlt vedd fel a `tools/images.py` CONFIG listájába, futtasd (`python tools/images.py`), majd hivatkozz rá `kep:<kulcs>` formában. Kép cseréjekor **új kulcs (fájlnév)** kell, mert az `/img` alatti fájlokat a böngésző egy évig cache-eli.

## Teszt

```bash
cargo test --features ssr
```
