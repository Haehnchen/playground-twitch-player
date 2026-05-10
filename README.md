# Twitch Player

A lightweight mpv-based desktop app for watching Twitch streams with Twitch account login,
chat, fullscreen controls, and an optional 2x2 stream grid.

<p align="center">
  <img src="docs/icon.webp" alt="Twitch Player icon" width="46">
  <br>
  <img src="docs/screenshot.webp" alt="Twitch Player GTK interface" width="720">
</p>


## Streams

Channels are loaded from the user settings file:

```text
~/.config/twitch-player/settings.json
```

The first start is intentionally empty. Add channels through the Settings button
in the top-left overlay.

Open Settings > Channels and use "Connect Twitch" to authorize the app. The app
uses Twitch's device-code flow and stores the resulting user token in the
settings file.

## Dependencies

On Ubuntu/Debian:

```bash
sudo apt install build-essential cargo rustc pkg-config libgtk-4-dev libmpv-dev libepoxy-dev libjson-glib-dev libsoup-3.0-dev mpv yt-dlp
```

Check the local machine:

```bash
./scripts/check-deps.sh
```

## Build

```bash
make build
```

This uses `cargo build --release` and strips the resulting binary.

Or build and copy to `~/.local/bin`

```bash
make install
```

## Run

```bash
./target/release/twitch-player
```

Start with a channel or URL:

```bash
./target/release/twitch-player papaplatte
./target/release/twitch-player https://www.twitch.tv/montanablack88
```

Switch between the normal player and the 2x2 grid from the overlay controls.
Start directly in grid mode with up to four channels:

```bash
./target/release/twitch-player --grid papaplatte rumathra
./target/release/twitch-player --grid papaplatte rumathra montanablack88 another_channel
```

## AppImage

Build a portable AppImage with bundled runtime libraries:

```bash
make appimage
```

The resulting file is written to `dist/`.
