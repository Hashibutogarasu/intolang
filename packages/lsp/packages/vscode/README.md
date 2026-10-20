# Into for VS Code

A thin extension: it starts `into-lsp` and passes the settings on. Everything else is done by the language server.

Build the two programs and put them on the `PATH`:

```sh
cargo install --path packages/lsp
cargo install --path packages/analysis
```

Node.js, npm and pnpm come from the Nix shell of the repository (`nix develop` or `nix develop .#llvm`), so their versions are pinned by `flake.lock`. Build the extension from the repository root:

```sh
nix develop .#llvm -c pnpm --dir packages/lsp/packages/vscode install
nix develop .#llvm -c pnpm --dir packages/lsp/packages/vscode run compile
```

Then press `F5` in this folder, or run `pnpm run package` for a `.vsix` file.

| Setting | Meaning |
| --- | --- |
| `into.lsp.path` | The language server program (`into-lsp`). |
| `into.analyzer.path` | The program that answers the server (`intolang-ipc`). |
| `into.packagesDir` | The directory that holds the builtin packages. Empty looks for `INTOLANG_PACKAGES` and then for a `packages/intolang` directory above the project. |
