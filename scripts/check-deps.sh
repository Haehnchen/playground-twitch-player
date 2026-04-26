#!/usr/bin/env bash
set -euo pipefail

missing=0

check_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing command: %s\n' "$1"
    missing=1
  fi
}

check_pkg() {
  if ! pkg-config --exists "$1"; then
    printf 'missing pkg-config package: %s\n' "$1"
    missing=1
  fi
}

check_cmd meson
check_cmd ninja
check_cmd mpv
check_cmd yt-dlp

check_pkg gtk4
check_pkg mpv
check_pkg epoxy

if [ "$missing" -ne 0 ]; then
  cat <<'EOF'

Ubuntu/Debian packages:
  sudo apt install build-essential meson ninja-build pkg-config libgtk-4-dev libmpv-dev libepoxy-dev mpv yt-dlp
EOF
  exit 1
fi

printf 'all dependencies found\n'
