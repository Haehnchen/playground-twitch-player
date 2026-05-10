PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
CARGO ?= cargo
CARGO_TARGET_DIR ?= target
STRIP ?= strip
STRIP_FLAGS ?= --strip-unneeded
RELEASE_BIN := $(CARGO_TARGET_DIR)/release/twitch-player

.PHONY: setup build test run run-grid appimage install clean

setup:
	CARGO_TARGET_DIR="$(CARGO_TARGET_DIR)" "$(CARGO)" fetch

build:
	CARGO_TARGET_DIR="$(CARGO_TARGET_DIR)" "$(CARGO)" build --release
	"$(STRIP)" $(STRIP_FLAGS) "$(RELEASE_BIN)"

test:
	CARGO_TARGET_DIR="$(CARGO_TARGET_DIR)" "$(CARGO)" test --release

run: build
	"$(RELEASE_BIN)"

run-grid: build
	"$(RELEASE_BIN)" --grid

appimage:
	./scripts/build-appimage.sh

install: build
	install -d "$(BINDIR)"
	install -m 755 "$(RELEASE_BIN)" "$(BINDIR)/twitch-player"
	"$(STRIP)" $(STRIP_FLAGS) "$(BINDIR)/twitch-player"

clean:
	rm -rf "$(CARGO_TARGET_DIR)" build build-release
