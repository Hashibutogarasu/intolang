# Into for Zed

A thin extension: it starts `into-lsp` and passes the settings of the editor on. Everything else is done by the language server.

Install `into-lsp` and `intolang-ipc` and put them on the `PATH`:

```sh
cargo install --path packages/lsp
cargo install --path packages/analysis
```

Install this directory with `zed: install dev extension`.

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
