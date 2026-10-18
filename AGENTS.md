# AGENTS.md

Guidance for agents working in this repository.

## What this project is

`intolang` is the compiler and toolchain for **Into**, a small programming
language whose source files use the `.into` extension. The project builds a
Rust CLI (binary `intolang`, usually symlinked/aliased as `into`) that:

1. parses and lowers `.into` source to an internal text "message",
2. compiles that message with LLVM (`llvm-sys`), and
3. links it with an embedded LLD to produce native executables or shared libraries.

It is a Cargo workspace (Rust edition 2024).

## Repository layout

| Path | Crate / contents | Role |
| --- | --- | --- |
| `src/` | `intolang` (root) | CLI entrypoint (`main.rs`) and `src/commands/*` (`new`, `build`, `install`, `format`) |
| `apps/compiler/` | `compiler` | Drives `codegen`: target machine, object emission, native source compilation (`native`), links via `lld`; embeds resources in `build.rs` |
| `packages/codegen/` | `codegen` | LLVM code generation from the lowered message: one node per record under `src/native/`, shared LLVM state in `src/context/` |
| `apps/compiler/res/` | — | `packages/intolang` runtime and `templates/hello_world` used by `into new` |
| `packages/types/` | `types` | Language type structures only (tokens, keywords, `Type`, message tags); no tokenizing or formatting behaviour |
| `packages/parser/` | `parser` | `FromStr`/`TryFrom`/`From` conversions from text to the `types` structures (lexemes, types, literals, hex, message `Record`s) |
| `packages/tokenizer/` | `tokenizer` | Source text to `Tokens` (with spans; comments only on request), built on `parser` |
| `packages/ast/` | `ast` | AST nodes and their token-based construction (`Program`, items, statements, expressions); independent of lines and whitespace |
| `packages/formatter/` | `formatter` | Token-based source formatter with `FormatOptions` |
| `packages/moduler/` | `moduler` | Import/export resolution, public items, link metadata and library lowering, on top of `ast` |
| `packages/native/` | `native` | C/C++ native integration: `#[link]` model, native call arguments/operands, native source compilation |
| `packages/backend/` | `backend` | Lowering: walks `ast` nodes and emits the message (`Lowerer`, `stmt/`, `expr/`, `emit/`) |
| `packages/testing/` | `testing` | Into `#[test]` runner and the `eval` utilities that run Into from Rust tests (`eval_run` behind the `run` feature) |
| `packages/lld/` | `lld` | Thin wrapper that links objects with an embedded/`LLD_EXTERNAL_BINARY` LLD |
| `packages/intolang/` | package, alias `builtin` | Builtin `std` (`print`/`println`) and test macros/helpers |
| `packages/system/modules/{c,cpp,json}/` | native packages | Language system modules (C stdio, C++ http/filesystem/time/string, JSON) |
| `packages/wayland/`, `packages/vulkan/` | native packages | Third-party native example packages |
| `examples/` | Into projects | Sample programs (`hello_world`, `Fibonacci`, `json`, `networking`, `memory`, `native-lib`, `Bad Apple!!`) |
| `dist/` | Cargo target dir | Git-ignored; the flake sets `CARGO_TARGET_DIR=$PWD/dist` |

Each Into package has an `into-package.json` manifest, `src/main/`, and build
output under `dist/<os>/<arch>/` (also git-ignored).

### Package source layout

Packages that combine Rust, FFI and Into split their sources by role:

| Path | Contents |
| --- | --- |
| `src/rust/` | Rust crate source; the manifest sets `[lib] path = "src/rust/lib.rs"` |
| `src/ffi/` | FFI sources (C/C++ and their headers, referenced from Into via `#[link("c" \| "cpp", "src/ffi/...", "symbol")]` and keyed by that path in `nativeDependencies`) |
| `src/main/` | Into language sources (the manifest `library`/`runs` entry lives here) |

Every Into package uses these three directories; native sources never live
under `src/main/`. `packages/moduler` is the reference example. `apps/compiler/build.rs` copies
third-party packages while skipping `src/rust`, `target` and `dist`, so only the
language-facing parts are embedded.

## Toolchain requirements

- **LLVM 23** and **LLD**. The compiler expects `LLVM_SYS_231_PREFIX` to point at
  an LLVM dev tree. `nix develop` (see `flake.nix`) provisions LLVM 23, lld,
  `cargo`/`rustc`, Wayland/Vulkan headers, `pkg-config`, `yt-dlp`/`ffmpeg`
  (for example assets), plus the `codex` CLI.
- Native example packages need their system libraries (e.g. `wayland-client`,
  Vulkan loader). C/C++ native modules are compiled with `cc`/`c++` (override via
  the `CC`/`CXX` env vars) and dependencies are resolved through `pkg-config`.

## Build, test, run

```sh
nix develop                    # recommended shell; also builds `into` symlink
cargo build                    # debug build of the workspace
cargo build --bin intolang     # just the CLI
cargo test                     # Rust tests (each crate's tests/ directory)
cargo fmt
cargo clippy
```

Inside `nix develop`, `into` is a symlink to `dist/debug/intolang`, so you can
run packages directly:

```sh
into new my-app                # scaffold from the hello_world template
into build examples/hello_world
into run examples/hello_world
into build --test examples/memory   # run Into #[test] files under src/tests
into format examples/json --depth 2 # format .into files (also -format / -depth)
into install packages/json          # add a package to the current project
```

CLI notes:

- Commands: `new`, `install`, `uninstall` (`rm`), `build` (default), `run`, `format`. A bare path
  defaults to `build`.
- Legacy single-dash flags are accepted and normalized: `-test`, `-run`,
  `-format`, `-depth`.
- `into new` flags: `--licence` (default `MIT`), `--version` (default `0.1.0`),
  `--alias`.
- `into install <dir>` links `.into/packages/<alias>` to the package directory
  (a symbolic link, no copy) and records it in the project manifest's
  `packages` map and in `into-package-lock.json`. The lock file lists each
  package as a `moduler::PackageInfo` (alias, name, version, licence, content
  `hash` that ignores `.git` and `dist`, and the `location` of the package,
  relative to the project). Builds reference the links: the temporary package
  environment symlinks every installed package instead of copying it.
- `into install` without a directory restores the links of the packages in the
  lock file from their locations and fails if a package's hash differs from the
  lock.
- `into uninstall <alias>...` (alias `into rm`) removes the link (never the
  package itself), its `packages` entry and its lock entry.

## The Into language (essentials)

- Imports use `scheme://module`, e.g. `import "intolang://builtin/std";`,
  `import "json://value";`, `import "cpp://http/client";`. The scheme is a
  package alias (`intolang` resolves to the `builtin` package). Installed
  packages are matched by their manifest `alias`.
- Entry point is `init { ... }` (functions use `pub fun`). Test files in
  `src/tests/**/*.into` use `#[test]` on functions and must import the test
  macro, `assert`, and `memory-check` helpers.
- Declarations: `into x = ...` (scope-local), `now into x = ...` (allocate
  immediately), `late into x = ...` (lazy), `const now into ...`, `unload x;`.
- Control flow: `if (cond) { } else { }`, `while (cond) { }`, `break;`.
- Enums: `enum Format { Text, JSON }` (uppercase names/variants, optional `= n`).
- Data carriers: `pub JsonObject() { kind: Int32; text: String; }`; field access
  via `value.field`.
- Foreign declarations: `pub c fun puts(value: String): Int32;` or
  `pub cpp fun ...`, preceded by `#[link("c" | "cpp", "relative/source.cpp", "symbol")]`.
- A package exposes its API through `src/main/into-info.into` with
  `export "alias://module";` lines (every `pub` declaration in the module becomes
  an exported name) or `export "alias://module/item";` for an explicit item.
  Declarations without `pub` are private. Library packages list modules to export
  in `src/main/lib.into` (`export "alias://module";`).

## Architecture / data flow

`into build <pkg>`:
`src/commands/build.rs` reads `into-package.json` → assembles a package
environment in a temp dir (embedded builtins + project `.into/packages` deps) →
`backend::lower_file` tokenizes (`tokenizer`), builds the AST (`ast`) and
walks it to produce the lowered message → `compiler::compile_with_packages`
runs LLVM codegen, `native::compile_sources` compiles any imported C/C++
sources, and `lld` links a binary/shared library.

- The language is whitespace and newline independent: tokenizing, AST
  construction and formatting work on tokens only, never on source lines. A
  missing `;` is accepted before a `}` or a line break.
- The lowered "message" is a line-based text format. Compiler directives are
  `parser::Record`s: the record separator `\u{1e}`, a tag from
  `types::MessageTag` (`NATIVE_CLASS`, `NATIVE_LINK_ASSIGN`, `NATIVE_IF_BEGIN`,
  ...) and tab separated fields. `backend` writes them (`backend/src/emit`) and
  `packages/codegen/src/program/message.rs` reads them; keep both in sync
  through `MessageTag`, `Record` and the shared `native` types (`LinkCall`,
  `Comparison`, `Operand`, `NativeArg`) when adding a tag.
- `backend` must not depend on LLVM. `codegen` owns the LLVM node code and
  `compiler` depends on `backend`, `codegen`, `native` and `lld`.
- `apps/compiler/build.rs` copies resources and embedded packages into `OUT_DIR`
  and exports `COMPILER_BUNDLED_RES`. It re-runs when `res/`,
  `packages/intolang`, `packages/system/modules`, or workspace packages change —
  so edits to bundled packages take effect after a rebuild.
- Diagnostics use `anyhow`; the backend wraps errors in an
  `Into language backtrace:` format via `parser::diagnostic`.

## Conventions and gotchas

- Edition 2024; use `anyhow` for errors in the tool, `serde`/`serde_json` for
  manifests. No comments unless necessary.
- `mod.rs` (and `lib.rs` of the library crates under `packages/`) only declare
  modules and decide what is re-exported; they hold no logic. Group files into
  directories by responsibility rather than keeping them flat.
- Conversions between text and language structures use `From`, `TryFrom` and
  `FromStr` (see `packages/parser`); do not add ad-hoc string helpers. Other
  crates cannot implement foreign traits for `types`, so `parser` wraps them in
  `*Lexeme` newtypes.
- Do not hardcode layout choices such as indentation; formatter settings live
  in `formatter::FormatOptions`. Keyword, punctuation and operator spellings
  live in `parser`, not in string comparisons elsewhere.
- Rust tests live in each crate's `tests/` directory (integration tests, one
  file per concern), not in `src/`. Tests must not read the repository's own
  projects or other external files; embed the Into source in the test, and use
  `testing::eval`, `testing::eval_with` (lower to a message) or
  `testing::eval_run` (compile and run; needs the `run` feature and LLVM, so run
  it inside `nix develop`).
- Manifests: a package must specify exactly one of `runs` (executable) or
  `library`. Library entry must be `src/main/lib.into`. Aliases must be
  non-empty, not `.`/`..`, and `[A-Za-z0-9._-]` only.
- The formatter uses 4-space block indentation and collapses consecutive blank
  lines; `//` are line comments (ignore braces inside strings/comments).
- Adding a system module: create the module package, register it in
  `packages/system/into-package.json` under `modules`, add `lib.into` /
  `into-info.into`, native sources, and any `nativeDependencies` keyed by the
  source path relative to the package root.
- There is no CI configuration, test script, or lint config beyond `cargo`
  defaults; run `cargo fmt` / `cargo clippy` / `cargo test` manually.
- Only commit when explicitly asked. `dist/`, `.into/`, and `target/` are
  generated and git-ignored.
- Do not propose next steps, follow-up actions, or "what to do next" in
  responses.
- Do not use `echo`/`printf` for decoration or section separators.
- Do not print or echo exit codes.
- Do not filter command output through `grep` (e.g. for `error`/`warning:`).
  Redirect the full output to a log file in a temporary directory under a
  non-sequential (descriptive/unique, not `1.log`/`2.log`) filename, then read
  that file with the Read tool.
