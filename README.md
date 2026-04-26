# Twitch Player

Simple Linux/Wayland Twitch player using GTK4, libmpv, and a read-only Twitch
chat panel.

The player embeds mpv through `libmpv` instead of trying to reparent an external mpv
window. That matters on Wayland, where `mpv --wid` style embedding is not a reliable
application model.

## Streams

The default dropdown contains three Twitch channels:

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

Or use the Makefile wrapper:

```bash
make setup
make build
```

## Run

```bash
./build/twitch-player
```

Start with a channel or URL:

```bash
./build/twitch-player papaplatte
./build/twitch-player https://www.twitch.tv/montanablack88
```
