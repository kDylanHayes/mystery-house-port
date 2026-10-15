#!/bin/sh
# Build the game for web pages: a static folder (index.html, the JavaScript glue and the
# WebAssembly) to copy to any web server.
#
# usage: tools/build-web.sh [out dir]          (default: dist/web)
#
# Needs the wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown) and
# wasm-bindgen-cli at the version of the wasm-bindgen crate in Cargo.lock
# (cargo install wasm-bindgen-cli --version <that version> --locked).
set -eu
cd "$(dirname "$0")/.."
out=${1:-dist/web}
want=$(grep -A1 'name = "wasm-bindgen"$' Cargo.lock | sed -n 's/version = "\(.*\)"/\1/p')
have=$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2)
if [ "$have" != "$want" ]; then
    echo "wasm-bindgen-cli $want is needed (found ${have:-none}):" >&2
    echo "  cargo install wasm-bindgen-cli --version $want --locked" >&2
    exit 1
fi
cargo build --release --target wasm32-unknown-unknown -p mystery-house-app
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" --out-name mystery-house \
    target/wasm32-unknown-unknown/release/mystery-house.wasm
cp web/index.html "$out/"
ls -l "$out"
