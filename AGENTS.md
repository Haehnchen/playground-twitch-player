# Repository Guidelines

## Project Layout

- `src/main.rs`: Rust executable entry point.
- `src/lib.rs`: Rust core module root.
- `src/app_main.rs`: GTK window, libmpv rendering, controls, stream selection.
- `src/chat_panel.rs`: chat panel UI and message rendering.
- `src/twitch_chat.rs`: anonymous/read-only Twitch IRC client.
- `Cargo.toml`: Cargo build definition.
- `build.rs`: pkg-config based system library linking for Cargo.
- `scripts/check-deps.sh`: local dependency check.

## Build and Run

```bash
./scripts/check-deps.sh
make build
make test
./target/release/twitch-player
./target/release/twitch-player papaplatte
```

Run `make build` after code changes. Run `make test` after changes that touch
tested helpers or behavior, and use failing tests to guide the fix before
finishing.

## Style

Use the existing Rust style: `rustfmt`, `snake_case` names, small focused
functions, and GLib helpers where already used. Keep changes in the matching
module: chat UI in `chat_panel.rs`, Twitch IRC in `twitch_chat.rs`, GTK/libmpv
app logic in `app_main.rs`.

## Testing

Automated tests live in `tests/` and are wired through Cargo. Use `make test` to
run them.

Manually smoke test startup, stream playback, chat connection, channel
switching, fullscreen, window dragging, and resizing when the change affects
runtime UI or playback behavior.
