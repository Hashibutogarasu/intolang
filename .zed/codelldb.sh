#!/bin/sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd -P)"
exec nix develop "$root#llvm" -c codelldb "$@"
