# Shared by the deploy scripts (sourced, not run).

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The installed release: binary, hash.txt and site/ (read-only for the service).
APP_DIR=/opt/kozmosz

step() { printf '\n\033[1;33m==> %s\033[0m\n' "$*"; }
ok() { printf '\033[32m%s\033[0m\n' "$*"; }
warn() { printf '\033[1;35mFigyelem: %s\033[0m\n' "$*" >&2; }
die() {
    printf '\n\033[1;31mHiba: %s\033[0m\n' "$*" >&2
    exit 1
}
