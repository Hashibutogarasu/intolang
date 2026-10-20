#!/bin/sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd -P)"
extension="$root/packages/lsp/packages/zed"
installed="$root/dist/zed-dev/extensions/installed"

nix develop "$root#llvm" -c cargo build -p lsp -p analysis --manifest-path "$root/Cargo.toml"

rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2 --manifest-path "$extension/Cargo.toml"
cp "$extension/target/wasm32-wasip2/release/into_zed.wasm" "$extension/extension.wasm"

mkdir -p "$extension/grammars"
tree-sitter build --wasm --output "$extension/grammars/into.wasm" "$extension/grammars/tree-sitter-into"

mkdir -p "$installed"
ln -sfn "$extension" "$installed/into"
rm -f "$root/dist/zed-dev/extensions/index.json"
