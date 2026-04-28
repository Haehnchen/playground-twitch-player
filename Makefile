PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: setup build run run-grid install clean

setup:
	meson setup build

build:
	meson compile -C build

run:
	./build/twitch-player

run-grid:
	./build/twitch-player --grid papaplatte rumathra

install: build
	install -d "$(BINDIR)"
	install -m 755 build/twitch-player "$(BINDIR)/twitch-player"

clean:
	rm -rf build
