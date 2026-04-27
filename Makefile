.PHONY: setup build run run-grid install clean

setup:
	meson setup build

build:
	meson compile -C build

run:
	./build/twitch-player

run-grid:
	./build/twitch-player --grid papaplatte rumathra

install:
	meson install -C build

clean:
	rm -rf build
