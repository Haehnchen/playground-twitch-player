# Repository Guidelines

## Project Layout

- `src/main.c`: GTK window, libmpv rendering, controls, stream selection.
- `src/chat_panel.c`: chat panel UI and message rendering.
- `src/twitch_chat.c`: anonymous/read-only Twitch IRC client.
- `meson.build`: build definition.
- `scripts/check-deps.sh`: local dependency check.

## Build and Run

```bash
./scripts/check-deps.sh
meson setup build
meson compile -C build
meson test -C build
./build/twitch-player
./build/twitch-player papaplatte
```

Run `meson compile -C build` after code changes. Run `meson test -C build`
after changes that touch tested helpers or behavior, and use failing tests to
guide the fix before finishing.

## Style

Use the existing C style: 4-space indentation, `snake_case` names, small focused
functions, and GLib helpers where already used. Keep changes in the matching
module: chat UI in `chat_panel.c`, Twitch IRC in `twitch_chat.c`, GTK/libmpv app
logic in `main.c`.

## Testing

Automated tests live in `tests/` and are wired through Meson. Use
`meson test -C build` to run them.

Manually smoke test startup, stream playback, chat connection, channel
switching, fullscreen, window dragging, and resizing when the change affects
runtime UI or playback behavior.
