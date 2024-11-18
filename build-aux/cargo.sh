#!/bin/bash

export MESON_BUILD_ROOT="$1"
export MESON_SOURCE_ROOT="$2"
export CARGO_TARGET_DIR="$MESON_BUILD_ROOT"/target
export CARGO_HOME="$CARGO_TARGET_DIR"/cargo-home

if [[ $4 = "Devel" ]]
then
    echo "DEBUG MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml && \
        cp "$CARGO_TARGET_DIR"/debug/$5 $3
elif [[ $4 = "Windows" ]]
then
  echo "WINDOWS MODE"
      cargo build --target=x86_64-pc-windows-gnu --release --manifest-path \
          "$MESON_SOURCE_ROOT"/Cargo.toml
else
    echo "RELEASE MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml --release && \
        cp "$CARGO_TARGET_DIR"/release/$5 $3
fi

