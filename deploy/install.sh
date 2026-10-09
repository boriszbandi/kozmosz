#!/usr/bin/env bash
# First-time setup of the Kozmosz website on a Debian 12 or 13 machine. Run it from this
# checkout as a normal user who can use sudo:
#
#   ./deploy/install.sh
#
# What it does: installs the build tools and Caddy, installs Rust and cargo-leptos for this user,
# writes the systemd service, the settings file and the Caddyfile, then builds and starts the
# site (deploy/update.sh). Safe to run again: every step checks what is already done, and the
# settings file (/etc/kozmosz/kozmosz.env) is never overwritten.
set -euo pipefail
# shellcheck source=deploy/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# The cargo-leptos version the project is built with (see README).
CARGO_LEPTOS_VERSION=0.3.11

[ "$(id -u)" -ne 0 ] || die "Ne rootként futtasd, hanem a saját felhasználóddal (sudo joggal)."
command -v sudo >/dev/null || die "Kell a sudo: rootként apt install sudo, majd usermod -aG sudo <felhasználó>, és jelentkezz be újra."
sudo -v

if [ -r /etc/os-release ]; then
    # shellcheck source=/dev/null
    . /etc/os-release
fi
[ "${ID:-}" = "debian" ] || warn "Debian 12/13-ra készült, ez a gép: ${PRETTY_NAME:-ismeretlen}. Debian-alapú rendszeren valószínűleg működik."

# The release build (fat LTO) needs about 3 GB of memory; the running site needs ~100 MB.
mem_mb=$(awk '/^(MemTotal|SwapTotal):/ {kb += $2} END {print int(kb / 1024)}' /proc/meminfo)
if [ "$mem_mb" -lt 3500 ] && [ -z "${SKIP_MEMORY_CHECK:-}" ]; then
    die "Kevés a memória a fordításhoz: RAM + swap ${mem_mb} MB, kb. 3500 MB kell.
Adj hozzá swapot (deploy/README.md, \"Kevés memória\"), vagy próbáld így: SKIP_MEMORY_CHECK=1 ./deploy/install.sh"
fi
free_gb=$(df -Pk "$HOME" | awk 'NR == 2 {print int($4 / 1048576)}')
[ "$free_gb" -ge 8 ] || warn "Kevés a szabad hely a home könyvtárban (${free_gb} GB): a fordítás kb. 8 GB-ot használ."

# Caddy needs ports 80 and 443. Another web server there (Apache, nginx) would make it fail.
if command -v ss >/dev/null; then
    others=$(sudo ss -ltnpH '( sport = :80 or sport = :443 )' | grep -v '"caddy"' || true)
    [ -z "$others" ] || die "A 80-as vagy a 443-as portot már egy másik program használja:
$others
Állítsd le (pl. sudo systemctl disable --now apache2 vagy nginx), majd futtasd újra."
fi

step "Csomagok (apt)"
# A fresh machine often runs its automatic updates (unattended-upgrades) right after boot and
# holds the package lock meanwhile: wait for it instead of failing.
apt_wait=(-o DPkg::Lock::Timeout=900)
ok "Ha épp automatikus rendszerfrissítés fut, megvárom (legfeljebb 15 percig)."
sudo apt-get "${apt_wait[@]}" update
sudo apt-get "${apt_wait[@]}" install -y --no-install-recommends \
    build-essential pkg-config cmake nasm git curl ca-certificates caddy

step "Rust és cargo-leptos ($USER felhasználónak)"
if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
# shellcheck source=/dev/null
source "$HOME/.cargo/env"
rustup update stable --no-self-update
rustup target add wasm32-unknown-unknown
if cargo leptos --version 2>/dev/null | grep -qF "$CARGO_LEPTOS_VERSION"; then
    ok "cargo-leptos $CARGO_LEPTOS_VERSION már megvan."
else
    cargo install cargo-leptos --version "$CARGO_LEPTOS_VERSION" --locked
fi

step "Szolgáltatás és beállítások"
sudo install -d -m 755 /etc/kozmosz
if [ -e /etc/kozmosz/kozmosz.env ]; then
    ok "/etc/kozmosz/kozmosz.env már megvan, nem írom felül."
else
    sudo install -m 644 "$REPO_DIR/deploy/kozmosz.env" /etc/kozmosz/kozmosz.env
fi
sudo install -m 644 "$REPO_DIR/deploy/kozmosz.service" /etc/systemd/system/kozmosz.service
sudo systemctl daemon-reload
sudo systemctl enable kozmosz.service

step "Caddy"
if [ -e /etc/caddy/Caddyfile ] && ! cmp -s "$REPO_DIR/deploy/Caddyfile" /etc/caddy/Caddyfile; then
    backup="/etc/caddy/Caddyfile.$(date +%Y%m%d-%H%M%S).bak"
    sudo cp /etc/caddy/Caddyfile "$backup"
    ok "Az eddigi Caddyfile elmentve: $backup"
fi
sudo install -m 644 "$REPO_DIR/deploy/Caddyfile" /etc/caddy/Caddyfile
sudo caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
sudo systemctl enable caddy
sudo systemctl reload-or-restart caddy

if command -v ufw >/dev/null && sudo ufw status | grep -q "Status: active"; then
    step "Tűzfal (ufw): 80 és 443 nyitása"
    sudo ufw allow 80/tcp
    sudo ufw allow 443/tcp
    sudo ufw allow 443/udp
fi

"$REPO_DIR/deploy/update.sh" --no-pull

# The address to try in a browser. On Google Cloud the machine itself only knows its internal
# address; the external one comes from the metadata server.
ip=$(curl -fsS --max-time 2 -H "Metadata-Flavor: Google" \
    http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip 2>/dev/null ||
    hostname -I 2>/dev/null | awk '{print $1}' || true)
step "Kész"
cat <<EOF
Az oldal fut. Próbáld ki egy böngészőben: http://${ip:-<a gép IP-címe>}/

Ha a kozmosz.bme.hu már erre a gépre mutat, a Caddy magától kér HTTPS-tanúsítványt.
A további lépések (DNS, frissítés, naplók) a deploy/README.md-ben vannak.
EOF
