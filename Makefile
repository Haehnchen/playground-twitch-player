.PHONY: setup build run install clean

setup:
	meson setup build

build:
	meson compile -C build

run:
	./build/twitch-player

install:
	meson install -C build

clean:
	rm -rf build
