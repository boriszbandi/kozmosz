# Médialeltár

A régi WordPress oldal (kozmosz.bme.hu) és a galériák Google Drive mappáinak képei, 2026-10-08-i állapot.
Az eredeti fájlok **nincsenek a repóban**, hanem a `D:\Claude\kozmosz-wp-export\` alatt. Az új oldalra optimalizálva (AVIF/WebP, metaadat nélkül) kerülnek a `public/img/` alá, amikor felhasználjuk őket.

| Mappa | Mi van benne | Fájl |
|---|---|---:|
| `media/` | a régi oldalakon használt képek | 22 |
| `media-extra/` | a WordPress médiatárban lévő, de oldalon nem használt fájlok | 42 |
| `drive/sstv/`, `drive/asztro/` | a Drive galériák nyilvánosan letölthető képei | 14 |
| `raw/` | REST API JSON-ok, oldalak HTML-je, leltárak | – |

**Fontos:**
- A régi médiatárban **egyik képnek sincs alt szövege**: az új oldalon mindegyikhez írni kell.
- **Két fotóban GPS-koordináták vannak** (lent jelölve), ezeket csak metaadat nélkül szabad kitenni.
- A `logo_transzparens_feher.svg` üres: vektoros szakkollégiumi logó nincs, csak a 7500×4800-as PNG. Vektorizálni kell (vagy elkérni az eredetit attól, aki tervezte).

## Oldalakon használt képek

| Fájl | Méret | Felbontás | Hol szerepelt | Megjegyzés |
|---|---:|---:|---|---|
| `media/641571979_1514896063973762_135689781821570869_n.jpg` | 576 KB | 2048×1152 | rólunk |  |
| `media/cropped-logo_kor-1.png` | 28 KB | 512×512 | archív, asztrofotók, elérhetőség, előadóink, kezdőlap, időjárás műhold, ISSTV, rólunk, programjaink, projektek | kerek embléma (lábléc / favicon) |
| `media/cropped-logo_kor.png` | 28 KB | 512×512 | archív, asztrofotók, elérhetőség, előadóink, kezdőlap, időjárás műhold, ISSTV, rólunk, programjaink, projektek | kerek embléma (lábléc / favicon) |
| `media/ede2127c-f25b-40ec-a5bc-de8790124068-scaled.jpg` | 467 KB | 2560×1440 | kezdőlap |  |
| `media/image-1.png` | 854 KB | 945×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-10.png` | 1000 KB | 991×660 | előadóink (sablonból 7 oldalon) |  |
| `media/image-11.png` | 1112 KB | 944×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-12.png` | 896 KB | 944×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-13.png` | 1082 KB | 944×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-14.png` | 1081 KB | 944×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-16.png` | 1002 KB | 944×629 | előadóink (sablonból 7 oldalon) | duplikátum: `media-extra/image-15.png` |
| `media/image-3.png` | 1242 KB | 945×630 | előadóink (sablonból 7 oldalon) | duplikátum: `media-extra/image-2.png` |
| `media/image-4.png` | 958 KB | 945×630 | előadóink (sablonból 7 oldalon) |  |
| `media/image-5.png` | 889 KB | 945×630 | előadóink (sablonból 7 oldalon) |  |
| `media/image-6.png` | 1235 KB | 944×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-7.png` | 1106 KB | 945×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-8.png` | 1066 KB | 945×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image-9.png` | 1091 KB | 945×629 | előadóink (sablonból 7 oldalon) |  |
| `media/image.png` | 1384 KB | 945×630 | előadóink (sablonból 7 oldalon) |  |
| `media/IMG_1144-scaled.jpg` | 184 KB | 1440×2560 | kezdőlap | **GPS az EXIF-ben!** |
| `media/IMG_1185-scaled.jpg` | 684 KB | 2560×1920 | kezdőlap |  |
| `media/logo_transzparens_feher.png` | 183 KB | 7500×4800 | archív, asztrofotók, elérhetőség, előadóink, kezdőlap, időjárás műhold, ISSTV, rólunk, programjaink, projektek | fejléc logó (Egyetemi Kozmosz Szakkollégium), csak PNG |

## Drive: ISS SSTV galéria (13 / 13 nyilvános)

| Fájl | Méret | Felbontás | Hol szerepelt | Megjegyzés |
|---|---:|---:|---|---|
| `drive/sstv/20241012_125306.png` | 720 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/20241113_002156.png` | 572 KB | 640×450 | ISSTV galéria |  |
| `drive/sstv/20241113_202308.png` | 792 KB | 640×496 | ISSTV galéria | duplikátum: `media-extra/20241113_202308.png` |
| `drive/sstv/20241114_193138.png` | 805 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/20241114_224220.png` | 808 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/20241115_235901.png` | 795 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/20250103_230258.png` | 667 KB | 640×496 | ISSTV galéria | duplikátum: `media-extra/20250103_230258.png` |
| `drive/sstv/20250103_230712.png` | 709 KB | 640×499 | ISSTV galéria |  |
| `drive/sstv/20250412_092231.png` | 825 KB | 640×493 | ISSTV galéria |  |
| `drive/sstv/20250415_114915.png` | 675 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/IMG_3217.png` | 898 KB | 640×496 | ISSTV galéria | valójában JPEG |
| `drive/sstv/PD120_20241116_173959.png` | 569 KB | 640×496 | ISSTV galéria |  |
| `drive/sstv/PD120_20241116_174408.png` | 538 KB | 640×496 | ISSTV galéria |  |

## Drive: asztrofotó galéria (1 / 29 nyilvános)

| Fájl | Méret | Felbontás | Hol szerepelt | Megjegyzés |
|---|---:|---:|---|---|
| `drive/asztro/Hold1 (1).jpg` | 3734 KB | 2509×2822 | asztrofotó galéria |  |

## Médiatárban, de oldalon nem használt

Köztük: közösségi fotók (csoportképek, félévzáró, irodaavató, éjszakai észlelés), Orion-köd, asztrokurzus, SSTV-montázsok, logóváltozatok, egy 404-es illusztráció. A `.jpeg` előadófotók a `.png`-k kisebb, 605 px-es változatai.

| Fájl | Méret | Felbontás | Hol szerepelt | Megjegyzés |
|---|---:|---:|---|---|
| `media-extra/189-IMG_5052-scaled.jpg` | 361 KB | 2560×1440 |  | duplikátum: `media-extra/IMG_5052-scaled.jpg` |
| `media-extra/20241113_202308.png` | 792 KB | 640×496 |  | duplikátum: `drive/sstv/20241113_202308.png` |
| `media-extra/20250103_230258.png` | 667 KB | 640×496 |  | duplikátum: `drive/sstv/20250103_230258.png` |
| `media-extra/404-e1737577196979.png` | 892 KB | 1602×892 |  |  |
| `media-extra/asztrokurzus1.png` | 794 KB | 909×961 |  |  |
| `media-extra/asztrokurzus_result.png` | 985 KB | 889×546 |  |  |
| `media-extra/DSC00915-scaled.jpg` | 445 KB | 2560×1704 |  |  |
| `media-extra/DSC01261-scaled.jpg` | 553 KB | 2560×1704 |  |  |
| `media-extra/ep_naturalblack.png` | 255 KB | 400×400 |  |  |
| `media-extra/felevzaro_csoportkep.png` | 475 KB | 630×642 |  |  |
| `media-extra/felevzaro_kepek.png` | 778 KB | 813×725 |  |  |
| `media-extra/image-1.jpeg` | 22 KB | 605×403 |  |  |
| `media-extra/image-10.jpeg` | 21 KB | 604×402 |  |  |
| `media-extra/image-11.jpeg` | 21 KB | 604×402 |  |  |
| `media-extra/image-12.jpeg` | 29 KB | 604×402 |  |  |
| `media-extra/image-13.jpeg` | 25 KB | 604×402 |  |  |
| `media-extra/image-15.png` | 1002 KB | 944×629 |  | duplikátum: `media/image-16.png` |
| `media-extra/image-2.jpeg` | 26 KB | 605×403 |  |  |
| `media-extra/image-2.png` | 1242 KB | 945×630 |  | duplikátum: `media/image-3.png` |
| `media-extra/image-3.jpeg` | 19 KB | 605×403 |  |  |
| `media-extra/image-4.jpeg` | 25 KB | 605×402 |  |  |
| `media-extra/image-5.jpeg` | 18 KB | 605×403 |  |  |
| `media-extra/image-6.jpeg` | 23 KB | 605×402 |  |  |
| `media-extra/image-7.jpeg` | 18 KB | 605×402 |  |  |
| `media-extra/image-8.jpeg` | 19 KB | 634×422 |  |  |
| `media-extra/image-9.jpeg` | 29 KB | 604×402 |  |  |
| `media-extra/image.jpeg` | 32 KB | 605×403 |  |  |
| `media-extra/IMG_1192-scaled.jpg` | 438 KB | 2560×1440 |  |  |
| `media-extra/IMG_5052-scaled.jpg` | 361 KB | 2560×1440 |  | duplikátum: `media-extra/189-IMG_5052-scaled.jpg` |
| `media-extra/IMG_5078-scaled.jpg` | 593 KB | 2560×1440 |  |  |
| `media-extra/IMG_5080-scaled.jpg` | 365 KB | 2560×1440 |  |  |
| `media-extra/irodaavato.png` | 793 KB | 846×506 |  |  |
| `media-extra/logo_kor.png` | 144 KB | 1824×1823 |  |  |
| `media-extra/logo_transzparens_feher-1.svg` | 0 KB | – |  | **üres SVG** (nincs benne rajz), nem használható duplikátum: `media-extra/logo_transzparens_feher.svg` |
| `media-extra/logo_transzparens_feher-e1737576268984.png` | 159 KB | 1875×1200 |  |  |
| `media-extra/logo_transzparens_feher.svg` | 0 KB | – |  | **üres SVG** (nincs benne rajz), nem használható duplikátum: `media-extra/logo_transzparens_feher-1.svg` |
| `media-extra/logo_transzparens_fekete-e1737576136389.png` | 166 KB | 1875×1200 |  |  |
| `media-extra/material-symbols-group-1.png` | 16 KB | 512×512 |  |  |
| `media-extra/Orion_Nebula-scaled.jpg` | 240 KB | 1619×2560 |  |  |
| `media-extra/PXL_20241212_171635845-scaled.jpg` | 427 KB | 2560×1928 |  | **GPS az EXIF-ben!** |
| `media-extra/sstv_hajnal.png` | 653 KB | 576×533 |  |  |
| `media-extra/tarsas_csapep.png` | 861 KB | 886×880 |  |  |

## Drive: nem nyilvános képek (nincs letöltve)

43 fájl Google-bejelentkezést kér. A régi oldal az *Integrate Google Drive* pluginnal, a `bme.kozmosz` fiók nevében mutatja őket. **A fiók tulajdonosának kell letöltenie** a mappákat ZIP-ben (Drive → mappa → Letöltés), és a `D:\Claude\kozmosz-wp-export\drive\` alá tennie.

| Mappa | Hiányzik |
|---|---:|
| [Asztro content](https://drive.google.com/drive/folders/1Ye9x7I3ReobGIjRgYxbfMzQoY01bUz86) | 28 |
| [Időjárásműhold content](https://drive.google.com/drive/folders/1_D-ujPnjDbNtgyvB_Z2yjJP7R0uPbIO_) | 15 |

<details><summary>Hiányzó fájlok</summary>

- időjárás műhold: 2023.05.15_08.12_color.png (`1yy5JH8i4FKUzs6dkGQA0lYkW_4DKYM3l`)
- időjárás műhold: 2023.05.17_07.48_color.png (`1i6DOuplrmzgWnvP3Xxcr9QLl3QxAiYBu`)
- időjárás műhold: 2023.05.20_06.13_color.png (`1dbAxEM8gCNExpdaP4E3lCYwpSkQqULid`)
- időjárás műhold: 2023.05.22_07.02_color.png (`1NgjwgdhZ-wEfaovLBxmtzjIDJ0tgiXqf`)
- időjárás műhold: 2023.05.23_06.37_color.png (`1W-wrRRoPig0lOe_SykLHD26xfbVIOxuk`)
- időjárás műhold: 2023.05.24_06.11_color.png (`1kFmjbteXh3kTsh_NHy1JHUBCn4PpHRpg`)
- időjárás műhold: 2023.05.25_07.52_color.png (`19X2MFIc65hmKh1nHthB2EPdK401QBhL_`)
- időjárás műhold: 2023.05.27_09.51_color.png (`1q1ZVHWaFNSFHW1_4z5bUswkFwgKYcP7F`)
- időjárás műhold: 2023.06.02_07.56_color.png (`14tIuqwamqasg4LhlXeh_PP-bj0qpsQj6`)
- időjárás műhold: 2023.06.08_08.25_color.png (`14DdZBgZGSRfNtvidAT14Q7JyfpwUNWpk`)
- időjárás műhold: 2023.06.13_07.24_color.png (`15poyo9Ed9cOj1mrHosTN8D-2hKy_6iXJ`)
- időjárás műhold: 2023.06.19_06.48_color.png (`1Yg2otNfhgzaEqe7VbFRglRfjsfG9_3X1`)
- időjárás műhold: 2023.06.19_07.52_color.png (`1AJ99MeLeAliYqbsm4Q1x8GX01jXIaFrP`)
- időjárás műhold: 2023.06.20_06.22_color.png (`1d-3pCXz-sPfHH-PWfkzh49P6cm8XOluX`)
- időjárás műhold: 2023.06.22_11.38_color.png (`1OdNs51es-ToLHjSILF2peVjqMwiWl9pB`)
- asztrofotók: Vénusz.jpg (`1kS4mYUUkXnxf0UgZ4N0rnGB4xzBB8eZd`)
- asztrofotók: Jupiter Szaturnusz 12. 23..jpg (`1W8qvbIMrsvAcaHoDyrOUhGAxxbDOhxIW`)
- asztrofotók: 2024_10_15_Ustokos.png (`1-68dkkoFRaKMkf08nXizzDwiBNK_ARZr`)
- asztrofotók: 2024_10_15_Mellekhold.png (`1FhbdPharTSkKpUfDzPkHaHFv2x4HL6nN`)
- asztrofotók: Orion-köd.jpg (`1Iia95lCcQg0mu3_ixMOHZnbS5e8XaF4T`)
- asztrofotók: Lófej-köd.jpg (`1hFOuxZP3nvu2V843OQ60sIjS2y_YTbYe`)
- asztrofotók: Súlyzó-köd.jpg (`1H91wFeHlE6ERiMOO8ELpPTLvV52X1dXa`)
- asztrofotók: Gyűrűs-köd.jpg (`1Cz7VhY-qHIzjTcyO-MgVIENfSijFgeME`)
- asztrofotók: Nap.jpg (`1ZnoiaZ4ZyuU86sgvk0AtYjuobMefpqXW`)
- asztrofotók: Szaturnusz1.jpg (`1oteNtOY2fI3eeQZ0eXSUVo-v2KYw_UED`)
- asztrofotók: Jupiter.jpg (`1agqm_77L5lxOB1k31TRU9DtkFx6OVYg3`)
- asztrofotók: Szélkerék-galaxis.jpg (`1oP0zm_J0Ubc-O9B7zlAQRZ0Zi0ljK80j`)
- asztrofotók: California_Nebula.jpg (`12LHCgFOZclkpK_YEnr5CyXxm_3Ln1elS`)
- asztrofotók: Triangulum_Galaxy.jpg (`12LzJjSDBXy5VjztSE4rSO3asnUhWnV4Y`)
- asztrofotók: Crescent_Nebula.jpg (`12S8DRwcgu_w2WPHjo3e68EjRm09jED13`)
- asztrofotók: Orion_Nebula.jpg (`12TNazQlFEknYEYEtOQcz6Yy3pOYAEfQq`)
- asztrofotók: North_America_Nebula.jpg (`12XXsh4XRALyjyRFOydfgRmC1GlZ7ilm2`)
- asztrofotók: Rosette_Nebula.jpg (`12adZKnnQQ4-9N5ZDP9nw6N-fPhPQe9Pz`)
- asztrofotók: Moon.jpg (`12g8_OeriRCta9SLRTfDeReCttzCDDxb6`)
- asztrofotók: Moon2.jpg (`12iCdOwqVOHywnuZ9UgOFRn6fpFYU_Js3`)
- asztrofotók: Saturn.png (`12j-WAeE-G-8zG1BF5AW7oOxY36IKJt0-`)
- asztrofotók: Napfogyatkozás.jpg (`18ekZtnKGV39vnjKXq2PBWLCJHQY9mFIF`)
- asztrofotók: Markarján-lánc.jpg (`1ykvqBChW4lKOU6ejFm-e2YZDr4h8ZNUq`)
- asztrofotók: Sas-köd.jpg (`1fmaqJzYSUrrdZ3Rcfr3t3mtf7NOvU94y`)
- asztrofotók: Szaturnusz.jpg (`1rVQ5leXaVUIj3E4WTGvaSp2wrdioIk6o`)
- asztrofotók: Hold.jpg (`17YBnxaR0p4SmCoq5ZiRdZXkpqbPUqXOb`)
- asztrofotók: C2020F3 NEOWISE Őszeszék.jpg (`1gKhUaEuAfKyRcQa-klJboTLFUqT-K3yD`)
- asztrofotók: C 2020 F3 NEOWISE rendes.jpg (`12H_vaLaW_HAroBELY-L_ZK7d7Zw2r2_o`)

</details>
