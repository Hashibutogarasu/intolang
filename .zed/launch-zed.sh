#!/bin/sh
# Starts a second Zed in the foreground on this repository, with a profile of
# its own, so the logs of the Into extension and of the language server show
# up in the debug terminal. Install the extension in that Zed once with
# `zed: install dev extension`.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd -P)"

zed=""
for candidate in zed zeditor zed-editor; do
    if command -v "$candidate" >/dev/null 2>&1; then
        zed="$candidate"
        break
    fi
done

if [ -z "$zed" ]; then
    printf '%s\n' 'Neither zed, zeditor nor zed-editor was found on the PATH' >&2
    exit 1
fi

export RUST_LOG="${RUST_LOG:-info}"
exec "$zed" --foreground --user-data-dir "$root/dist/zed-dev" "$root"
