#!/bin/sh
# Starts Zed in the foreground on this repository so the logs of the Into
# extension and of the language server show up in the debug terminal.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd -P)"

if ! command -v zed >/dev/null 2>&1; then
    printf '%s\n' 'zed was not found on the PATH' >&2
    exit 1
fi

export RUST_LOG="${RUST_LOG:-info}"
exec zed --foreground "$root"
