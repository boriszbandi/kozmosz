#!/usr/bin/env bash
# Builds this checkout, installs it to /opt/kozmosz and restarts the server.
#
#   ./deploy/update.sh             pulls the latest commit of the branch first
#   ./deploy/update.sh --no-pull   builds what is checked out
#
# The previous release stays in /opt/kozmosz.prev for a quick rollback.
set -euo pipefail
# shellcheck source=deploy/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[ "$(id -u)" -ne 0 ] || die "Ne rootként futtasd, hanem a saját felhasználóddal (sudo joggal)."
[ -x "$HOME/.cargo/bin/cargo" ] || die "Nincs Rust ennél a felhasználónál: előbb futtasd a deploy/install.sh-t."
# shellcheck source=/dev/null
source "$HOME/.cargo/env"
cd "$REPO_DIR"

if [ "${1:-}" != "--no-pull" ]; then
    step "Frissítés a GitHubról"
    git pull --ff-only
fi

step "Fordítás (release; első alkalommal 10-20 perc)"
features=(--bin-features ssr)
if command -v nasm >/dev/null; then
    # rav1e's assembly kernels: several times faster AVIF encoding of the Drive photos.
    features+=(--bin-features fast-avif)
fi
cargo leptos build --release --precompress --cargo-locked "${features[@]}"
[ -f target/release/hash.txt ] || die "Hiányzik a target/release/hash.txt: a fordítás nem a várt fájlokat adta."

step "Telepítés: $APP_DIR"
new="$APP_DIR.new"
sudo rm -rf "$new"
sudo install -d -m 755 "$new"
sudo install -m 755 target/release/kozmosz "$new/kozmosz"
sudo install -m 644 target/release/hash.txt "$new/hash.txt"
sudo cp -r target/site "$new/site"
sudo chmod -R a+rX,go-w "$new"
if [ -d "$APP_DIR" ]; then
    sudo rm -rf "$APP_DIR.prev"
    sudo mv "$APP_DIR" "$APP_DIR.prev"
fi
sudo mv "$new" "$APP_DIR"

step "Újraindítás"
sudo systemctl restart kozmosz
# Startup renders every cached page and downloads the calendar: give it a minute.
for _ in $(seq 1 60); do
    if curl -fsS -o /dev/null http://127.0.0.1:3000/; then
        ok "Fut: $(git rev-parse --short HEAD) ($(git log -1 --format=%s))"
        exit 0
    fi
    sleep 1
done
sudo journalctl -u kozmosz -n 40 --no-pager || true
die "A szerver nem válaszol. Visszaállás az előző verzióra:
  sudo rm -rf $APP_DIR && sudo mv $APP_DIR.prev $APP_DIR && sudo systemctl restart kozmosz"
