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

A `drive-cache/` könyvtár (a Drive-galériák kódolt képei, lásd lent) a munkakönyvtárban jön létre. Maradjon meg újraindítások között, különben minden képet újra kell kódolni.

Élesben tegyél elé reverse proxyt (Caddy vagy nginx) a TLS, a HTTP/2-3 és a timeoutok miatt. nginx-nél kell a `proxy_buffering off;` beállítás.

## Drive-galériák

A projektoldalak galériái a klub Google Drive-mappáiból jönnek, mint a régi oldalon az *Integrate Google Drive* pluginnal: amit feltöltötök a mappába, az pár percen belül megjelenik az oldalon.

- A mappák a `src/drive.rs` `ALBUMS` listájában vannak (asztrofotók, ISSTV, időjárásműhold).
- A szerver induláskor, majd 10 percenként listázza a mappákat. Az új vagy megváltozott képeket letölti és átkódolja AVIF-re és JPEG-re, több méretben. A metaadatok, köztük a GPS-koordináták, nem kerülnek át. A kész fájlok a `drive-cache/files/` könyvtárba kerülnek, és a `/drive/` alól mennek ki. A leltár (`drive-cache/manifest.json`, Drive-azonosítókkal) nem publikus. Ha egy fájlt nem sikerül feldolgozni, a szerver addig nem próbálja újra, amíg a fájl nem változik. A Drive-ból törölt képek egy napig még elérhetők, aztán törlődnek. A látogató böngészője sosem kér semmit a Google-től.
- Minden képnek saját oldala van (`/projektek/isstv/kep/20241113-202308`). Az oldalon a kép nagyban jelenik meg, lapozni lehet, és megosztáskor ez a kép lesz az előnézet.
- Felirat:
  - Ha van a Drive-ban leírás, az lesz a felirat (csak a 2. és 3. elérési módnál).
  - Ha nincs, a Markdownban azonos nevű kép alt szövege: a `20241113_202308.png` fájlhoz a `kep:sstv/20241113-202308`.
  - Ha az sincs, a fájlnév: a `Vénusz.jpg` felirata „Vénusz”. Ezért érdemes beszédes fájlneveket adni.
  - SSTV- és műholdképeknél a fájlnévben lévő időpont a vétel ideje.
- Formátumok: JPG, PNG, WebP. A HEIC (iPhone) képeket kihagyja, ezeket exportáld JPG-be.
- Amíg egy mappa nem olvasható, vagy még nem futott le a szinkron, a galéria a Markdown-fájlban felsorolt képeket mutatja.

A szerver háromféleképpen érheti el a mappákat. Ebből az elsőhöz nem kell semmi beállítás:

1. **Link alapú megosztás** (alapértelmezés): a mappa megosztása „Bárki, aki rendelkezik a linkkel: Megtekintő”. Nincs kulcs, de a Drive-leírások nem látszanak. Most az ISSTV-mappa ilyen, az asztrofotós és az időjárásműholdas mappa még privát.
2. **API-kulcs**: link alapú megosztás mellett `KOZMOSZ_DRIVE_API_KEY=<kulcs>`. A kulcsot a Google Cloud Console-ban kell létrehozni, a Drive API engedélyezésével. Ezzel a leírások is átjönnek.
3. **Service account**, privát mappákhoz: a Google Cloud Console-ban hozz létre egy service accountot, és tölts le hozzá egy JSON kulcsot. A mappákat oszd meg a service account e-mail-címével (Megtekintő), majd add meg a kulcs útvonalát: `KOZMOSZ_DRIVE_CREDENTIALS=/etc/kozmosz/drive-key.json`. A kulcsfájl titkos, nem kerülhet a repóba.

A képek első kódolása lassú, AVIF-nél képenként akár 10 másodperc is lehet. Ha a szerveren van `nasm`, a `--features ssr,fast-avif` build (cargo-leptos-szal: `bin-features = ["ssr", "fast-avif"]`) ennek a töredéke alatt kódol.

További változók: `KOZMOSZ_DRIVE_DIR` (cache-könyvtár, alapból `drive-cache`), `KOZMOSZ_DRIVE=off` (nincs szinkron, csak a már letöltött képek látszanak).

## Szerkezet

| Fájl | Mi van benne |
|------|--------------|
| `content/oldalak/*.md` | az oldalak szövegei (ezeket kell szerkeszteni), képek `kep:<kulcs>` formában |
| `src/content.rs` | a Markdown-tartalom betöltése (frontmatter, `##` szekciók, galériák) |
| `src/pages/` | az oldalak elrendezése (kezdőlap, rólunk, elérhetőség, programok, előadók, projektek, 404) |
| `src/layout.rs` | fejléc, menü (mobilon JS nélküli `<details>`), lábléc, meta tagek |
| `src/site.rs` | név, cím, e-mail, közösségi linkek, menü, régi URL-ek átirányítása |
| `src/media.rs` | reszponzív `<picture>` (AVIF + JPEG), galéria, ikonok |
| `src/drive/` | Drive-galériák: mappák listázása, letöltés, AVIF/JPEG átkódolás, cache, Google-bejelentkezés |
| `src/gallery.rs` | projektgalériák összeállítása (Drive vagy Markdown), feliratok, vételi idők |
| `src/calendar/` | a klub Google Naptárának letöltése (10 percenként), iCal-feldolgozás, ismétlődő események, magyar dátumok |
| `src/server.rs` | memóriás oldal-cache (naptár- és galériafrissítéskor újrarenderel), átirányítások, cache-fejlécek, beágyazott CSS |
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
