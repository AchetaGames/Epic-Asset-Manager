#!/bin/bash

if [ "$1" = "clean" ]; then
    rm -rf .~/.cargo/bin/fenv/ .flatpak-builder/ _build/
    ~/.cargo/bin/fenv gen build-aux/io.github.achetagames.epic_asset_manager.Devel.json
    ~/.cargo/bin/fenv exec -- meson --prefix=/app -Dprofile=development _build
    exit 0
fi

if [ "$1" = "build" ]; then
    ~/.cargo/bin/fenv exec -- ninja -C _build
    exit 0
fi

if [ "$1" = "install" ]; then
    ~/.cargo/bin/fenv exec -- ninja -C _build install
    exit 0
fi

if [ "$1" = "dist" ]; then
    meson dist -C _build
    exit 0
fi

~/.cargo/bin/fenv exec -- ninja -C _build install
~/.cargo/bin/fenv run
