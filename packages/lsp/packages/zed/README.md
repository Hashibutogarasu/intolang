# Into for Zed

A thin extension: it starts `into-lsp` and passes the settings of the editor on. Everything else is done by the language server.

Install `into-lsp` and `intolang-ipc` and put them on the `PATH`:

```sh
cargo install --path packages/lsp
cargo install --path packages/analysis
```

Install this directory with `zed: install dev extension`. Zed builds the grammar of `packages/tree-sitter-into` while it installs the extension, and the queries in `languages/into` (`highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`) color and outline the code. `highlights.scm` is a link to `packages/tree-sitter-into/queries`, where the web playground reads it too.

## Syntax highlighting

Zed highlights with tree-sitter only, so the extension uses the grammar of Into in `packages/tree-sitter-into`. `extension.toml` names it in `[grammars.into]` by the repository, the `path` of the grammar and the commit `rev`, and Zed fetches that commit. After a change to the grammar, commit it, run `tree-sitter generate` in `packages/tree-sitter-into` (the generated `src/` is committed because Zed does not generate it), and set `rev` to the commit that holds the new `src/parser.c`. Install a dev extension from a checkout with `repository` set to `file:///path/to/intolang` to try a grammar before it is pushed.

```sh
cd packages/tree-sitter-into
tree-sitter generate
tree-sitter test
```

`tree-sitter parse` on every `.into` file of the repository is the check that the grammar covers the language; `examples/Errors/SyntaxError` is the one file that is meant to fail.

## Debugging from this repository

Open the repository in Zed and press `F5`, then pick `Into: Zed with the language server`. The task `Into: prepare Zed debug session` (`.zed/prepare-zed-profile.sh`) builds `into-lsp` and `intolang-ipc`, builds this extension for `wasm32-wasip2` with `rustup` (the way Zed builds a dev extension, so `rustup` must be installed) and links it into the profile, so the extension is there at the first start. Then F5 starts a second Zed in the foreground on the repository with its own profile in `dist/zed-dev` (the scenario starts `/run/current-system/sw/bin/zeditor`, the Zed command on NixOS; LLDB needs an absolute path, so change `program` in `.zed/debug.json` to the path of `zed` elsewhere), so the logs of the extension and of the server appear in the debug terminal. `.zed/settings.json` points the server at `dist/debug/into-lsp` and `dist/debug/intolang-ipc`, and at the builtin packages in `packages`; relative paths are relative to the project.

The task `Into: build language server` only rebuilds the two programs. `Into: attach to into-lsp` and `Into: attach to intolang-ipc` make CodeLLDB wait for a process of `dist/debug/into-lsp` or `dist/debug/intolang-ipc` to start and attach to it; start one of them, then open an `.into` file in the second Zed so that it starts the server.

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
