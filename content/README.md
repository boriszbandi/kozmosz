# Tartalom a régi oldalról

**Az `oldalak/*.md` fájlok mostantól az új oldal szerkeszthető forrásai.** Formátum: frontmatter (`cim`, `utvonal`, `regi_url`, `leiras`, a projekt-aloldalakon `kivonat` is, mind idézőjeles JSON-szöveg), utána Markdown törzs `## ` szakaszcímekkel. A képekre `kep:<kulcs>` hivatkozik (pl. `![Dr. Kovács Kálmán előadás közben](kep:eloadok/kovacs-kalman)`), a fájlokat a `tools/images.py` generálja a `public/img` mappába. Az érintetlen, szó szerinti eredeti a git-történetben van (`f389e97` commit), és a `D:\Claude\kozmosz-wp-export\raw` mappában.

A https://kozmosz.bme.hu WordPress oldal szövegei, **szó szerint** (2026-10-08-i állapot). Forrás: a nyilvános REST API (`/wp-json/wp/v2/pages`) és az oldalak kész HTML-je. Belépés nélkül szedtem le, így csak a publikált tartalom került ide, a piszkozatok nem.

A szövegen csak a lent felsorolt elírásokat javítottuk, a WordPress-maradványokat (beágyazások, Drive-galériák, régi kép-URL-ek) pedig kivettük. A letöltött eredetiket (78 fájl: használt képek, a médiatár többi fájlja és a nyilvános Drive-képek) a [media.md](media.md) sorolja fel.

## Oldaltérkép

| Régi URL | Menüben | Fájl | Új útvonal |
|---|---|---|---|
| `/` | (logó) | [kezdolap.md](oldalak/kezdolap.md) | `/` |
| `/notlikeus/` | Rólunk | [rolunk.md](oldalak/rolunk.md) | `/rolunk` |
| `/contact/` | Elérhetőség | [elerhetoseg.md](oldalak/elerhetoseg.md) | `/elerhetoseg` |
| `/programjaink/` | Programjaink | [programjaink.md](oldalak/programjaink.md) | `/programjaink` |
| `/eloadok/` | Előadóink | [eloadoink.md](oldalak/eloadoink.md) | `/eloadoink` |
| `/projektek/` | Projektek | [projektek.md](oldalak/projektek.md) | `/projektek` |
| `/idojaras-muhold/` | Projektek › Időjárás műhold | [projektek-idojaras-muhold.md](oldalak/projektek-idojaras-muhold.md) | `/projektek/idojaras-muhold` |
| `/isstv-felvetelek/` | Projektek › ISSTV Felvételek | [projektek-isstv.md](oldalak/projektek-isstv.md) | `/projektek/isstv` |
| `/asztrofotoink/` | Projektek › Asztrófotóink | [projektek-asztrofotok.md](oldalak/projektek-asztrofotok.md) | `/projektek/asztrofotok` |
| `/archiv/` | – | – | elhagyható (lásd lent) |

Ha az új oldal a kozmosz.bme.hu címre kerül, a régi URL-ekről **301-es átirányítás** kell az újakra, hogy a meglévő linkek és a keresőtalálatok ne törjenek el.

Megjegyzések:
- A **Projektek** oldal a három aloldal szövegének és galériájának összefűzése. Az új oldalon elég egy rövid áttekintő, ami az aloldalakra linkel.
- Az **Előadóink** lista (15 előadó, bio és fotó) a régi oldalon sablonból került 7 oldal aljára (Programjaink, Projektek és aloldalai, Archív). Itt csak egyszer, az [eloadoink.md](oldalak/eloadoink.md)-ben szerepel.
- Az **Archív** oldal üres helyőrző: a címén kívül csak a sablonból jövő előadólistát mutatja, menüből és más oldalról nem linkelik.
- A **kezdőlap** tartalma a WordPress-sablonban van, nem az oldal szövegében. Az oldal saját REST-tartalma egy angol demóminta („Home / News / About”), amit a látogatók nem látnak, ezért nem hoztam át.

## Az egész oldalon közös elemek

**Név:** Egyetemi Kozmosz Szakkollégium. Az előadók bióiban helyenként „Egyetemi Kozmosz Kör” szerepel (ez a korábbi név).

**Menü:**
- Rólunk
- Elérhetőség
- Programjaink
- Előadóink
- Projektek
  - Időjárás műhold
  - ISSTV Felvételek
  - Asztrófotóink

**Lábléc:** kerek logó, [Facebook](https://www.facebook.com/bme.kozmosz), [Instagram](https://www.instagram.com/bme.kozmosz/), „© 2025 – Egyetemi Kozmosz Szakkollégium”.

**Elérhetőség:**
- Egyetemi Kozmosz Szakkollégium, 1111 Budapest, Műegyetem rkp. 3. K386.
- E-mail: bme.kozmosz@gmail.com (a régi oldalon sima szöveg, nem kattintható link)
- Telefonszám nincs.

**Régi arculat (referenciának):**
- Sötét háttér (`#252422`, lábléc `#1F1D1B`), sárga kiemelőszín (`#FFDE73`), világos címsorok (`#FAF7EF`), szürke szöveg (`#807E7C`).
- Betűk: Urbanist (címsorok), Inter (szöveg), Bestermind (díszítő).
- Az új Rust oldal most a régi React verzió lila kiemelőszínét (`#BB86FC`) használja. Dönteni kell, melyik maradjon.

## Dinamikus részek és új megoldásuk

| Régi | Hol | Javaslat az új oldalra |
|---|---|---|
| Google Naptár iframe (`bme.kozmosz@gmail.com`) | Programjaink | A szerver percenként letölti a naptár nyilvános ICS-ét, és sima HTML-listaként rendereli a következő eseményeket. Nincs iframe, JS és Google-süti, és gyorsabb. Mellé egy feliratkozó link. |
| `.ics` letöltőgomb („Kozmosz online naptár”) | Programjaink | A naptár nyilvános ICS-címe vagy egy „Hozzáadás a Google Naptárhoz” link. |
| Google Maps iframe | Elérhetőség | Statikus térképkép plusz „Megnyitás térképen” link (Google Maps / OpenStreetMap). Nincs külső betöltés. |
| Google Drive galériák (igd plugin) | Projektek aloldalai | Letöltött, AVIF-re optimalizált képek, statikus rácsban. Nagyítás `<dialog>`-gal vagy egy kis islanddel. |
| Integető kéz és lebegő ikonok (SVG + CSS animáció) | Kezdőlap | Ha kell, tisztán CSS-sel, `prefers-reduced-motion` mellett. |

## Talált hibák a régi oldalon

**Javított elírások** (az `oldalak/*.md` site-tartalomban már javítva, az eredeti a `f389e97` commitban):
- Dr. Szabó József: „valamit az első magyar űrhajós” → *valamint*
- Dr. Bacsárdi László: „Magyar Asztonautikai Társaság” → *Asztronautikai*
- Dr. Kovács Kálmán: „Iráyítástechnikai” → *Irányítástechnikai*; „Villamosmérnöki és Informatika Karának” → *Informatikai Karának*
- Somodi Máté: „A BME-n végzet” → *végzett*
- Detre Örs Hunor: „szervezéséve.” → *szervezésével.*
- Kisebbek: „Szovjet-Magyar” → *szovjet–magyar* (Dr. Szabó József), „RNS világ” → *RNS-világ* (Medvegy Anna), „Kárpát medencét” → *Kárpát-medencét* (Időjárás műhold).
- Az „Asztrófotóink” (menü és oldalcím) és az „Asztrofotóink” (címsor és URL) eltért egymástól: egységesen *Asztrofotóink*.
- Rólunk: „Ha érdekel a világűr és szeretnél…” → *Ha érdekel a világűr, és szeretnél…*
- Az angol idézőjelek magyarra cserélve: “Puli” → *„Puli”*, “vízszimatoló” → *„vízszimatoló”* (Dr. Pacher Tibor).

**Ellenőrizendő, nem javítottuk** (szó szerint maradt):
- Szabó Nimród biójának első mondata hiányos („Szabó Nimród Zombor, az ELTE TTK-n végzett…”).
- A „Nyugat-Magyarországi Egyetem” név elavult lehet (Dr. Bacsárdi László).
- Az Asztrofotóink bevezetője harmadik személyben kezd („A Kozmoszosok … foglalkoznak”), aztán többes szám első személyre vált („kémleljük”). Ugyanígy az Időjárás műhold oldalon („foglalkoznak”, majd „figyeljük”).

**Technikai hibák:**
- `lang="en-US"` és `og:locale=en_US` magyar tartalom mellett. Üres oldalnév, ezért a címek „ -”-re végződnek.
- Több oldalon nincs látható `<h1>` (kezdőlap, Rólunk, Elérhetőség), vagy CSS rejti el.
- A 65 médiafájl közül egynek sincs alt szövege.
- A kezdőlap egyik fotójában (`IMG_1144`) GPS-koordináták vannak, és az élő oldal is így szolgálja ki.
- A Drive-galéria plugin base64-kódolva beleteszi az oldalakba a Google-fiók e-mail-címét és a fájl-ID-kat.
- Telepítve van a MonsterInsights, de nincs beállítva, így nincs analitika. Az Instagram-feed plugin minden oldalon betölt, de feedet nem mutat.

## Nyitott kérdések

1. **Drive képek:** a 14 nyilvános Drive-képet letöltöttem. A maradék 43-at (28 asztrofotó, 15 NOAA-kép) a `bme.kozmosz` fiókból kell exportálni (lásd [media.md](media.md)).
2. **Logó:** a médiatár SVG logója üres. Vektoros „Egyetemi Kozmosz Szakkollégium” logó csak akkor lesz, ha valakinél megvan az eredeti fájl, vagy ha vektorizáljuk a PNG-t.
3. **Elírások:** eldőlt, javítva (lásd fent a „Javított elírások” listát).
4. **Arculat:** eldőlt, a régi oldal sárga kiemelőszíne marad.
5. **URL-ek:** a javasolt magyar útvonalak jók. Nyitott még: hová kerül élesben az oldal?
