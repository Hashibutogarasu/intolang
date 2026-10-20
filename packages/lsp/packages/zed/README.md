# Into for Zed

A thin extension: it starts `into-lsp` and passes the settings of the editor on. Everything else is done by the language server.

Install `into-lsp` and `intolang-ipc` and put them on the `PATH`:

```sh
cargo install --path packages/lsp
cargo install --path packages/analysis
```

Install this directory with `zed: install dev extension`.

## Debugging from this repository

Open the repository in Zed and press `F5`, then pick `Into: Zed with the language server`. It builds `into-lsp` and `intolang-ipc` with the task `Into: build language server` and starts a second Zed in the foreground on the repository (`.zed/launch-zed.sh`, which needs `zed` on the `PATH`), so the logs of the extension and of the server appear in the debug terminal. `.zed/settings.json` points the server at `dist/debug/into-lsp` and `dist/debug/intolang-ipc`, and at the builtin packages in `packages`; relative paths are relative to the project.

Install this directory once with `zed: install dev extension`; Zed keeps it for the next starts. `Into: attach to into-lsp` attaches CodeLLDB to a running `into-lsp` or `intolang-ipc` that you pick in the process list.

## Settings

The server is configured in the Zed settings:

```json
{
  "lsp": {
    "into-lsp": {
      "binary": { "path": "/path/to/into-lsp" },
      "initialization_options": {
        "analyzerPath": "/path/to/intolang-ipc",
        "packagesDir": "/path/to/intolang/packages"
      }
    }
  }
}
```

`packagesDir` is the directory that holds the builtin packages. Without it, the server looks for `INTOLANG_PACKAGES` and then for a `packages/intolang` directory above the project.
