# Repository Guidelines

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

Use `rustfmt`: `make fmt` formats the code, `make fmt-check` checks formatting.
