# Telepítés szerverre (Debian 12 / 13)

Az oldal a szerveren fordul le, és systemd-szolgáltatásként fut. Előtte a [Caddy](https://caddyserver.com) intézi a HTTPS-t: amint a `kozmosz.bme.hu` erre a gépre mutat, magától kér és megújít egy Let's Encrypt-tanúsítványt. Addig az oldal a gép IP-címén, sima HTTP-n érhető el.

## Ami kell hozzá

- Debian 12 vagy 13, és egy sima felhasználó sudo joggal (nem root).
- Fordításhoz legalább 3,5 GB memória (RAM és swap együtt) és 8 GB szabad hely. Ha kevesebb van, lásd lent: [Kevés memória](#kevés-memória). Futás közben az oldal kb. 100 MB-ot használ.
- Kimenő internet (GitHub, crates.io, Google Naptár és Drive).
- Bejövő 80-as és 443-as port (TCP, és 443 UDP is a HTTP/3-hoz). Ha egyetemi tűzfal van a gép előtt, ezeket ott is nyitni kell.

## Első telepítés

```bash
sudo apt install -y git
git clone -b rust-leptos https://github.com/boriszbandi/kozmosz.git ~/kozmosz
cd ~/kozmosz
./deploy/install.sh
```

Az első futás 20-40 perc, mert az elejétől lefordít mindent. A script ezeket csinálja:

1. Telepíti a fordításhoz kellő csomagokat és a Caddyt (`apt`).
2. A te felhasználódnak telepíti a Rustot (rustup) és a `cargo-leptos`-t.
3. Létrehozza a szolgáltatást (`/etc/systemd/system/kozmosz.service`) és a beállításokat (`/etc/kozmosz/kozmosz.env`), és beállítja a Caddyt (`/etc/caddy/Caddyfile`). Ha már volt Caddyfile, előbb elmenti.
4. Lefordítja és elindítja az oldalt (`deploy/update.sh`).

A végén kiírja a címet. Nyisd meg böngészőben: `http://<a gép IP-címe>/`.

A script bármikor újrafuttatható. Amit már megcsinált, azt nem csinálja újra, a beállítási fájlt pedig sosem írja felül.

## Frissítés

Amikor új commit kerül a `rust-leptos` branchre:

```bash
cd ~/kozmosz
./deploy/update.sh
```

Letölti a változásokat, lefordítja (ez már csak pár perc), kicseréli a futó verziót, és ellenőrzi, hogy az oldal válaszol-e. Az előző verzió a `/opt/kozmosz.prev` könyvtárban marad. Ha valami elromlott, így állíthatod vissza:

```bash
sudo rm -rf /opt/kozmosz && sudo mv /opt/kozmosz.prev /opt/kozmosz && sudo systemctl restart kozmosz
```

## Átállás a kozmosz.bme.hu-ra

1. Kérd meg a BME domainkezelőjét, hogy a `kozmosz.bme.hu` A rekordja mutasson erre a gépre. Ha a gépnek van IPv6-címe, az AAAA rekord is. A gép IP-címét a `hostname -I` írja ki.
2. Amint a DNS átállt, a Caddy pár percen belül megszerzi a tanúsítványt. Ezt így követheted: `journalctl -u caddy -f`.
3. Ellenőrzés: `curl -I https://kozmosz.bme.hu` (a válasz `HTTP/2 200` legyen).
4. Ha az oldal már csak HTTPS-en legyen elérhető, töröld a `:80 { … }` blokkot a `/etc/caddy/Caddyfile`-ból, majd `sudo systemctl reload caddy`.

A régi WordPress-címek (például `/asztrofotoink/`) maguktól átirányítanak az új oldalakra.

## Hasznos parancsok

| Mit | Parancs |
|---|---|
| Fut-e? | `systemctl status kozmosz` |
| Napló (élőben) | `journalctl -u kozmosz -f` |
| Újraindítás | `sudo systemctl restart kozmosz` |
| Beállítások szerkesztése | `sudo nano /etc/kozmosz/kozmosz.env`, utána újraindítás |
| Caddy napló | `journalctl -u caddy -f` |

## Hol mi van

| Hely | Mi van benne |
|---|---|
| `~/kozmosz` | a forráskód (git), itt fordul |
| `/opt/kozmosz` | a futó verzió: `kozmosz` (a szerver), `hash.txt`, `site/` |
| `/opt/kozmosz.prev` | az előző verzió (visszaálláshoz) |
| `/etc/kozmosz/kozmosz.env` | beállítások (Leptos, Drive) |
| `/var/lib/kozmosz/drive-cache` | a Drive-galériák kódolt képei. Megmarad frissítéskor és újraindításkor. |
| `/etc/caddy/Caddyfile` | a Caddy beállítása |

A szolgáltatás egy ideiglenes, jogok nélküli felhasználóként fut. Csak a `/var/lib/kozmosz` könyvtárba írhat, és csak a gépen belülről érhető el (`127.0.0.1:3000`). Kívülről csak a Caddyn keresztül lehet elérni.

## Drive service account

Ha a Drive-mappákat privátan szeretnétek tartani (a fő README „Drive-galériák” része írja le, miért ez a javasolt mód), a service account JSON-kulcsát így add a szervernek:

```bash
sudo install -m 600 drive-key.json /etc/kozmosz/drive-key.json
sudo systemctl edit kozmosz
```

A megnyíló szerkesztőbe ezt írd:

```ini
[Service]
LoadCredential=drive-key.json:/etc/kozmosz/drive-key.json
Environment=KOZMOSZ_DRIVE_CREDENTIALS=/run/credentials/kozmosz.service/drive-key.json
```

Majd mentsd el, és indítsd újra: `sudo systemctl restart kozmosz`. A naplóban (`journalctl -u kozmosz`) ekkor ez a sor jelenik meg: `drive: reading 3 folders via Drive API as service account …`. A kulcsot csak a root olvashatja, a szolgáltatás a systemd-n keresztül kapja meg.

## Kevés memória

Ha a telepítő kevés memóriát jelez, adj a géphez 4 GB swapot:

```bash
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

Utána futtasd újra a `./deploy/install.sh`-t.
