PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: setup build run run-grid appimage install clean

setup:
	meson setup build

build:
	meson compile -C build

run:
	./build/twitch-player

appimage:
	./scripts/build-appimage.sh

install: build
	install -d "$(BINDIR)"
	install -m 755 build/twitch-player "$(BINDIR)/twitch-player"

clean:
	rm -rf build
