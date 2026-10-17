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
| `apps/compiler/` | `compiler` | LLVM codegen, target machine, object emission, native source compilation, links via `lld`; embeds resources in `build.rs` |
| `apps/compiler/res/` | — | `packages/intolang` runtime and `templates/hello_world` used by `into new` |
| `packages/backend/` | `backend` | Lexer, parser/lowering, import resolution, formatter, validation, test runner, library lowering |
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
| `src/ffi/` | FFI sources (C ABI shims referenced from Into via `#[link]`) |
| `src/main/` | Into language sources (the manifest `library`/`runs` entry lives here) |

`packages/moduler` is the reference example. `apps/compiler/build.rs` copies
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
cargo test                     # Rust unit tests (e.g. packages/backend/src/tests.rs)
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

- Commands: `new`, `install`, `build` (default), `run`, `format`. A bare path
  defaults to `build`.
- Legacy single-dash flags are accepted and normalized: `-test`, `-run`,
  `-format`, `-depth`.
- `into new` flags: `--licence` (default `MIT`), `--version` (default `0.1.0`),
  `--alias`.
- `into install <dir>` copies the package into `.into/packages/` and records it
  in the project manifest's `packages` map; it skips `.git` and `dist`.

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
`backend::lower_file` produces the lowered message → `compiler::compile_with_packages`
runs LLVM codegen, compiles any imported C/C++ native sources, and links a
binary/shared library with embedded LLD.

- The lowered "message" is a line-based text format. Compiler directives are
  prefixed with the record separator `\u{1e}` followed by tags such as
  `NATIVE_CLASS`, `NATIVE_LINK_ASSIGN`, `NATIVE_IF_BEGIN`, `NATIVE_IF_ELSE`,
  `NATIVE_LOOP_BEGIN`, `NATIVE_ARITH`, `NATIVE_FIELD_PRINT`, `NATIVE`, etc.
  `apps/compiler/src/codegen.rs` consumes these; keep producer (`backend`) and
  consumer (`compiler`) in sync when adding a tag.
- `backend` must not depend on LLVM. `compiler` depends on `backend` + `lld`.
- `apps/compiler/build.rs` copies resources and embedded packages into `OUT_DIR`
  and exports `COMPILER_BUNDLED_RES`. It re-runs when `res/`,
  `packages/intolang`, `packages/system/modules`, or workspace packages change —
  so edits to bundled packages take effect after a rebuild.
- Diagnostics use `anyhow`; the backend wraps errors in an
  `Into language backtrace:` format via `parser::diagnostic`.

## Conventions and gotchas

- Edition 2024; use `anyhow` for errors in the tool, `serde`/`serde_json` for
  manifests. No comments unless necessary.
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
