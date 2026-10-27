#!/usr/bin/env bash
# Usage: ./build.sh [web|native|hot] [--example NAME] [--serve|--run] [-- EXTRA_ARGS]
#   web (default): builds the wasm game into dist/web/; --serve also starts a local web server.
#   native:        builds a native binary into dist/native/; --run also starts it.
#   hot:           runs the native game under `dx serve` with subsecond hotpatching.
#   --example ecs builds examples/ecs.rs instead of src/main.rs (into dist/web-ecs/ or dist/native-ecs/).
#   Anything after -- is passed on to cargo build (web, native) or dx serve (hot).
set -euo pipefail
cd "$(dirname "$0")"

platform=web
example=
action=
extra=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    web|native|hot) platform=$1 ;;
    --example) example=$2; shift ;;
    --serve|--run) action=$1 ;;
    --) shift; extra=("$@"); break ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
  shift
done

name=raywasm
cargo_args=()
if [[ -n "$example" ]]; then
  name=$example
  cargo_args=(--example "$example")
fi

if [[ "$platform" == hot ]]; then
  exec dx serve --hot-patch --platform linux --features hotpatch "${cargo_args[@]}" "${extra[@]}"
fi

dist=dist/$platform${example:+-$example}
rm -rf "$dist"
mkdir -p "$dist"

case "$platform" in
  web)
    cargo build --release --target wasm32-unknown-emscripten "${cargo_args[@]}" "${extra[@]}"
    out=target/wasm32-unknown-emscripten/release
    [[ -n "$example" ]] && out=$out/examples
    sed "s/raywasm\.js/$name.js/" web/index.html > "$dist/index.html"
    cp "$out/$name.js" "$out/$name.wasm" "$dist"/
    if [[ "$action" == --serve ]]; then
      python3 -m http.server -d "$dist" 8080
    fi
    ;;
  native)
    cargo build --release "${cargo_args[@]}" "${extra[@]}"
    out=target/release
    [[ -n "$example" ]] && out=$out/examples
    cp "$out/$name" "$dist"/
    if [[ "$action" == --run ]]; then
      "./$dist/$name"
    fi
    ;;
esac
