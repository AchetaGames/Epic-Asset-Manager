# Building

There are multiple ways to build the package during development

## Flatpak

It is recommended to use [fenv](https://gitlab.gnome.org/ZanderBrown/fenv) to manage the flatpak environment

 1. fenv gen build-aux/io.github.achetagames.epic_asset_manager.Devel.json
 2. fenv exec -- meson --prefix=/app -Dprofile=development _build
 3. fenv exec -- ninja -C _build
 4. fenv exec -- ninja -C _build install
 5. fenv run

## Meson

Alternatively you can use meson

1. meson _build --prefix=/usr
2. ninja -C _build
3. ninja -C _build install
