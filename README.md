# Twitch Player 2

Simple Linux/Wayland Twitch player using GTK4 and libmpv.

The player embeds mpv through `libmpv` instead of trying to reparent an external mpv
window. That matters on Wayland, where `mpv --wid` style embedding is not a reliable
application model.

## Streams

The first MVP has three selectable Twitch channels:

- https://www.twitch.tv/montanablack88
- https://www.twitch.tv/papaplatte
- https://www.twitch.tv/rumathra

## Dependencies

On Ubuntu/Debian:

```bash
sudo apt install build-essential meson ninja-build pkg-config libgtk-4-dev libmpv-dev libepoxy-dev mpv yt-dlp
```

Check the local machine:

```bash
./scripts/check-deps.sh
```

## Build

```bash
meson setup build
meson compile -C build
```

## Run

```bash
./build/twitch-player-2
```

Start with a channel or URL:

```bash
./build/twitch-player-2 papaplatte
./build/twitch-player-2 https://www.twitch.tv/montanablack88
```

The app prints startup, libmpv, GL, and stream-loading diagnostics to the
console.

## Chat

The app joins the selected Twitch channel chat anonymously/read-only via Twitch
IRC and displays incoming chat messages in the right panel. Fullscreen hides the
chat and controls so only the video remains visible.

## Notes

- Twitch playback is delegated to mpv, which uses yt-dlp for stream URL resolving.
- The app forces mpv to `vo=libmpv` and `config=no` so user mpv config cannot
  open a separate video window.
- If a stream is offline or Twitch changes extraction behavior, update `yt-dlp` first.
- A future overlay should be built as UI above the GL area first. A separate transparent
  Wayland overlay window is compositor-dependent and should be treated as a second step.
