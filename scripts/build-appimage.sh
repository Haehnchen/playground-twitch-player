#!/usr/bin/env bash
set -euo pipefail

APP_ID="local.twitch-player"
APP_NAME="Twitch Player"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$PROJECT_ROOT/build}"
APPIMAGE_BUILD_DIR="${APPIMAGE_BUILD_DIR:-$PROJECT_ROOT/build/appimage}"
APPDIR="${APPDIR:-$APPIMAGE_BUILD_DIR/AppDir}"
DIST_DIR="${DIST_DIR:-$PROJECT_ROOT/dist}"
BUNDLE_YTDLP="${BUNDLE_YTDLP:-1}"

log() {
  printf '[appimage] %s\n' "$*"
}

die() {
  printf '[appimage] error: %s\n' "$*" >&2
  exit 1
}

find_tool() {
  command -v "$1" 2>/dev/null || true
}

download_tool() {
  local name="$1"
  local url="$2"
  local output="$3"
  local tmp_output="$output.tmp"

  mkdir -p "$(dirname "$output")"
  log "downloading $name"

  rm -f "$tmp_output"
  if command -v curl >/dev/null 2>&1; then
    if ! curl -L --fail -o "$tmp_output" "$url"; then
      rm -f "$tmp_output"
      return 1
    fi
  elif command -v wget >/dev/null 2>&1; then
    if ! wget -O "$tmp_output" "$url"; then
      rm -f "$tmp_output"
      return 1
    fi
  else
    die "curl or wget is required to download $name"
  fi

  chmod +x "$tmp_output"
  mv "$tmp_output" "$output"
}

detect_arch() {
  local machine
  machine="$(uname -m)"
  case "$machine" in
    x86_64 | amd64)
      printf 'x86_64\n'
      ;;
    aarch64 | arm64)
      printf 'aarch64\n'
      ;;
    *)
      die "unsupported AppImage architecture: $machine"
      ;;
  esac
}

require_packaging_tools() {
  local arch="$1"

  LINUXDEPLOY="$(find_tool linuxdeploy)"
  if [ -z "$LINUXDEPLOY" ]; then
    LINUXDEPLOY="$(command -v "linuxdeploy-$arch.AppImage" 2>/dev/null || true)"
  fi
  if [ -z "$LINUXDEPLOY" ] && [ -x "$PROJECT_ROOT/tools/linuxdeploy-$arch.AppImage" ]; then
    LINUXDEPLOY="$PROJECT_ROOT/tools/linuxdeploy-$arch.AppImage"
  fi
  if [ -z "$LINUXDEPLOY" ]; then
    LINUXDEPLOY="$PROJECT_ROOT/tools/linuxdeploy-$arch.AppImage"
    download_tool \
      "linuxdeploy" \
      "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$arch.AppImage" \
      "$LINUXDEPLOY" || LINUXDEPLOY=""
  fi

  APPIMAGETOOL="$(find_tool appimagetool)"
  if [ -z "$APPIMAGETOOL" ]; then
    APPIMAGETOOL="$(command -v "appimagetool-$arch.AppImage" 2>/dev/null || true)"
  fi
  if [ -z "$APPIMAGETOOL" ] && [ -x "$PROJECT_ROOT/tools/appimagetool-$arch.AppImage" ]; then
    APPIMAGETOOL="$PROJECT_ROOT/tools/appimagetool-$arch.AppImage"
  fi
  if [ -z "$APPIMAGETOOL" ]; then
    APPIMAGETOOL="$PROJECT_ROOT/tools/appimagetool-$arch.AppImage"
    download_tool \
      "appimagetool" \
      "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$arch.AppImage" \
      "$APPIMAGETOOL" || APPIMAGETOOL=""
  fi

  if [ -z "$LINUXDEPLOY" ] || [ -z "$APPIMAGETOOL" ]; then
    cat >&2 <<EOF
[appimage] Missing packaging tools.

This script downloads linuxdeploy and appimagetool into tools/ automatically.
The download did not complete. Check your network connection and rerun:

  make appimage
EOF
    exit 1
  fi

  export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"
}

write_launchers() {
  mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib/twitch-player"

  rm -f "$APPDIR/usr/bin/twitch-player" "$APPDIR/AppRun"

  cat >"$APPDIR/usr/bin/twitch-player" <<'EOF'
#!/usr/bin/env sh
set -eu

APPDIR="${APPDIR:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}"

prepend_env_path() {
  var_name="$1"
  value="$2"
  eval "current=\${$var_name:-}"
  if [ -n "$current" ]; then
    eval "export $var_name=\$value:\$current"
  else
    eval "export $var_name=\$value"
  fi
}

prepend_env_path PATH "$APPDIR/usr/bin"
prepend_env_path XDG_DATA_DIRS "$APPDIR/usr/share"

for libdir in "$APPDIR/usr/lib" "$APPDIR/usr/lib/"*; do
  [ -d "$libdir" ] || continue
  prepend_env_path LD_LIBRARY_PATH "$libdir"
done

exec "$APPDIR/usr/lib/twitch-player/twitch-player" "$@"
EOF
  chmod +x "$APPDIR/usr/bin/twitch-player"

  cat >"$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env sh
set -eu

APPDIR="${APPDIR:-$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)}"
exec "$APPDIR/usr/bin/twitch-player" "$@"
EOF
  chmod +x "$APPDIR/AppRun"
}

copy_desktop_files() {
  local desktop_src="$PROJECT_ROOT/data/$APP_ID.desktop"
  local icon_src="$PROJECT_ROOT/data/icons/hicolor/scalable/apps/$APP_ID.svg"

  install -Dm644 "$desktop_src" "$APPDIR/usr/share/applications/$APP_ID.desktop"
  install -Dm644 "$icon_src" "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"

  cp "$desktop_src" "$APPDIR/$APP_ID.desktop"
  cp "$icon_src" "$APPDIR/$APP_ID.svg"
}

bundle_ytdlp_zipapp() {
  local output="$1"

  command -v python3 >/dev/null 2>&1 || die "python3 is required to bundle yt-dlp as a zipapp"

  python3 - "$output" <<'PY'
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

try:
    import yt_dlp
except Exception as exc:
    raise SystemExit(f"yt_dlp Python package is not importable: {exc}") from exc

output = pathlib.Path(sys.argv[1])
source = pathlib.Path(yt_dlp.__file__).parent

with tempfile.TemporaryDirectory(prefix="twitch-player-ytdlp-") as tmp:
    root = pathlib.Path(tmp) / "app"
    shutil.copytree(
        source,
        root / "yt_dlp",
        ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "*.pyo"),
    )
    (root / "__main__.py").write_text(
        "from yt_dlp import main\n"
        "raise SystemExit(main())\n",
        encoding="utf-8",
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.check_call([
        sys.executable,
        "-m",
        "zipapp",
        str(root),
        "-p",
        "/usr/bin/env python3",
        "-o",
        str(output),
    ])

os.chmod(output, 0o755)
PY
}

bundle_ytdlp() {
  local output="$APPDIR/usr/bin/yt-dlp"
  local source="${YTDLP_BIN:-}"

  [ "$BUNDLE_YTDLP" != "0" ] || return 0

  if [ -n "$source" ]; then
    [ -x "$source" ] || die "YTDLP_BIN points to a non-executable file: $source"
    install -Dm755 "$source" "$output"
    return 0
  fi

  source="$(command -v yt-dlp 2>/dev/null || true)"
  if [ -n "$source" ] && file "$source" | grep -q 'ELF'; then
    install -Dm755 "$source" "$output"
    return 0
  fi

  log "bundling yt-dlp Python package as usr/bin/yt-dlp"
  bundle_ytdlp_zipapp "$output"
}

compile_schemas_if_present() {
  local schema_dir="$APPDIR/usr/share/glib-2.0/schemas"

  [ -d "$schema_dir" ] || return 0
  command -v glib-compile-schemas >/dev/null 2>&1 || return 0

  glib-compile-schemas "$schema_dir"
}

main() {
  local arch
  local output
  local output_tmp

  arch="$(detect_arch)"
  require_packaging_tools "$arch"

  log "building project"
  if [ ! -d "$BUILD_DIR" ]; then
    meson setup "$BUILD_DIR"
  fi
  meson compile -C "$BUILD_DIR"

  log "creating AppDir"
  rm -rf "$APPDIR"
  mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib/twitch-player" "$DIST_DIR"
  install -Dm755 "$BUILD_DIR/twitch-player" "$APPDIR/usr/lib/twitch-player/twitch-player"
  ln -s ../lib/twitch-player/twitch-player "$APPDIR/usr/bin/twitch-player"
  copy_desktop_files

  log "collecting shared libraries with linuxdeploy"
  "$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --executable "$APPDIR/usr/lib/twitch-player/twitch-player" \
    --desktop-file "$PROJECT_ROOT/data/$APP_ID.desktop" \
    --icon-file "$PROJECT_ROOT/data/icons/hicolor/scalable/apps/$APP_ID.svg"

  rm -f "$APPDIR/usr/bin/twitch-player"
  write_launchers
  copy_desktop_files
  bundle_ytdlp
  compile_schemas_if_present

  output="$DIST_DIR/Twitch_Player-$arch.AppImage"
  output_tmp="$DIST_DIR/.Twitch_Player-$arch.AppImage.tmp"
  log "writing $output"
  rm -f "$output_tmp"
  ARCH="$arch" "$APPIMAGETOOL" "$APPDIR" "$output_tmp"
  chmod +x "$output_tmp"
  mv -f "$output_tmp" "$output"

  log "done: $output"
}

main "$@"
