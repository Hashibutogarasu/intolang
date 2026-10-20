# AGENTS.md

Guidance for agents working in this repository.

## What this project is

`intolang` is the compiler and toolchain for **Into**, a small programming
language whose source files use the `.into` extension. The project builds a
Rust CLI (binary `intolang`, usually symlinked/aliased as `into`) that:

1. parses and lowers `.into` source to an internal text "message",
2. compiles that message with LLVM (`llvm-sys`), and
3. links it with an embedded LLD to produce native executables or shared libraries,

or, instead of steps 2 and 3, writes the message to an `.inex` file that the
Into VM (`intovm`) runs without LLVM. The VM also runs the tests of a package.

It is a Cargo workspace (Rust edition 2024).

## Repository layout

| Path | Crate / contents | Role |
| --- | --- | --- |
| `src/` | `intolang` (root) | CLI entrypoint (`main.rs`) and `src/commands/*` (`new`, `build`, `install`, `format`, `inex`, `error_format`) |
| `apps/compiler/` | `compiler` | Drives `codegen`: target machine, object emission, native source compilation (`native`), links via `lld`; embeds resources in `build.rs` |
| `packages/errors/` | `errors` | Every error of the toolchain: the `Errors` enum (message and hex code of each variant), `Error`/`Result`, the stack-trace layers, the JSON-compatible `ErrorContext` and the human/JSON reports; a leaf crate without LLVM that every other crate may depend on |
| `packages/bundle/` | `bundle` | A file tree in memory (`Bundle`, `BundlePath`, `Entry`): normalized relative paths, equal contents stored once, trees and packages added from directories; no LLVM and depends only on `errors` |
| `packages/codegen/` | `codegen` | LLVM code generation from the lowered message: one node per record under `src/native/`, shared LLVM state in `src/context/` |
| `apps/compiler/res/` | — | `packages/intolang` runtime and `templates/hello_world` used by `into new` |
| `packages/types/` | `types` | Language type structures only (tokens, keywords, `Type`, message tags); no tokenizing or formatting behaviour |
| `packages/parser/` | `parser` | `FromStr`/`TryFrom`/`From` conversions from text to the `types` structures (lexemes, types, literals, hex, message `Record`s) |
| `packages/tokenizer/` | `tokenizer` | Source text to `Tokens` (with spans; comments only on request), built on `parser` |
| `packages/ast/` | `ast` | AST nodes and their token-based construction (`Program`, items, statements, expressions); independent of lines and whitespace |
| `packages/formatter/` | `formatter` | Token-based source formatter with `FormatOptions` |
| `packages/moduler/` | `moduler` | Import/export resolution, public items, link metadata and library lowering, on top of `ast` |
| `packages/native/` | `native` | C/C++ native integration: `#[link]` model, native call arguments/operands, ownership records (`OwnershipChange`, `StateCheck`, `Property`), native source compilation to objects or to a shared library |
| `packages/backend/` | `backend` | Lowering: walks `ast` nodes and emits the message (`Lowerer`, `stmt/`, `expr/`, `emit/`); `lower_test` lowers one test function |
| `packages/intovm/` | `intovm` | The Into VM: runs a lowered message (`machine/`), virtual memory, threads and processes, native calls through `dlopen` (`native_call/`) and the `.inex` format (`inex/`); no LLVM |
| `packages/testing/` | `testing` | Into `#[test]` runner on top of `intovm`, and the `eval` utilities that run Into from Rust tests (`eval_vm`, `eval_run` behind the `run` feature) |
| `packages/ipc/` | `ipc` | The contract between an editor side and the compiler side: editor-independent positions, diagnostics, hovers, symbols and edits, the `Request`/`Notification`/`Reply` messages, the `Handler` trait the compiler side implements, `serve`, `Client` and the transports (`Lines` over any reader and writer, `MemoryTransport`); depends only on `serde` |
| `packages/analysis/` | `analysis` | The compiler side of the IPC: `Analyzer` implements `ipc::Handler` on top of `tokenizer`, `ast`, `moduler`, `backend` and `formatter` (open documents, diagnostics, the index of declarations, definition, hover, `///` documentation, symbols, formatting); the binary `intolang-ipc` serves it on the standard streams; no LLVM |
| `packages/lsp/` | `lsp` | The language server `into-lsp`, an app independent of the compiler: it speaks LSP to the editor and the `ipc` protocol to `intolang-ipc` and only converts between the two (UTF-16 columns, LSP types, restarting the analyzer); depends on `ipc` and no compiler crate |
| `packages/lsp/packages/{vscode,zed}/` | editor extensions | Thin extensions that start `into-lsp` and pass settings on; `vscode` is TypeScript (`npm run compile`), `zed` is a `zed_extension_api` crate outside the workspace (its own `Cargo.lock`, built for wasm) and carries the tree-sitter grammar of Into (`grammars/tree-sitter-into`, generated `src/` committed, `[grammars.into]` in `extension.toml` pinned by `rev`) with the queries in `languages/into` that give Zed its highlighting |
| `packages/lld/` | `lld` | Thin wrapper that links objects with an embedded/`LLD_EXTERNAL_BINARY` LLD |
| `packages/intolang/` | package, alias `builtin` | Builtin `std` (`print`/`println`), `Object`, `List`, the primitive types and `String` (`primitives.into`, empty classes so that editors find a declaration to jump to) and the test modules under `src/main/tests/` (`assert`, `memory-check`, `ownership-check`, `dummy-process`, the `test` macro) |
| `packages/ps/` | package, alias `ps` | `Process`, `Signal`, `Usage`, the `started`, `exited` and `signalReceived` events, `dispatch` and `exit` |
| `packages/system/modules/{c,cpp,json}/` | native packages | Language system modules (C stdio, C++ http/filesystem/time/string, JSON) |
| `packages/{fs,path,ffmpeg,sdl2,wayland,vulkan}/` | native packages | File access, paths, FFmpeg decoding, SDL2 audio, Wayland windows and Vulkan rendering |
| `examples/` | Into projects | Sample programs (`HelloWorld`, `Fibonacci`, `Json`, `Networking`, `Memory`, `Ownership`, `Process`, `NativeLib`, `Types`, `Bad Apple!!`) and `examples/Errors/*`, projects that fail on purpose with the code and message listed in its README |
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
under `src/main/`. `packages/moduler` is the reference example. A module is the
file under `src/main/` that its import path names: `import "intolang://builtin/tests/assert";`
reads `src/main/tests/assert.into` of the builtin package.
`apps/compiler/build.rs` copies third-party packages while skipping
`src/rust`, `target` and `dist`, so only the language-facing parts are embedded.

## Toolchain requirements

- **LLVM 23** and **LLD**. The compiler expects `LLVM_SYS_231_PREFIX` to point at
  an LLVM dev tree. `nix develop` (see `flake.nix`) provisions LLVM 23, lld,
  `cargo`/`rustc`, Wayland/Vulkan headers, `pkg-config`, `yt-dlp`/`ffmpeg`
  (for example assets), plus the `codex` CLI. `nix develop .#llvm` is the small
  shell for building and testing the Rust workspace: LLVM 23, lld, `cargo`/`rustc`,
  `zlib`, `libxml2` and `pkg-config`, without the example and codex dependencies
  and without building the `into` binary in the shell hook.
- Both shells provide `node`, `npm` and `pnpm` (versions pinned by `flake.lock`) for
  the VS Code extension; run them through the shell, for example
  `nix develop .#llvm -c pnpm --dir packages/lsp/packages/vscode run compile`.
- Native example packages need their system libraries (e.g. `wayland-client`,
  Vulkan loader). C/C++ native modules are compiled with `cc`/`c++` (override via
  the `CC`/`CXX` env vars) and dependencies are resolved through `pkg-config`.
  Running an `.inex` file that calls native code needs the same compilers.
- Start `nix develop` at the repository root. A shell started inside a directory
  whose name contains spaces (such as `examples/Bad Apple!!`) puts that path into
  `NIX_LDFLAGS` unquoted, and linking fails with `cannot open Apple!!/outputs/out/lib`.

## Build, test, run

```sh
nix develop                    # recommended shell; also builds `into` symlink
cargo build                    # debug build of the workspace
cargo build --bin intolang     # just the CLI
cargo test                     # Rust tests (each crate's tests/ directory)
cargo test --features testing/run   # also the tests that compile and run native programs
cargo fmt
cargo clippy
```

Inside `nix develop`, `into` is a symlink to `dist/debug/intolang`, so you can
run packages directly:

```sh
into new my-app                # scaffold from the hello_world template
into build examples/HelloWorld
into run examples/HelloWorld
into build --vm examples/HelloWorld   # write dist/<os>/<arch>/hello-world.inex
into run --vm examples/HelloWorld     # build the .inex file and run it in the VM
into hello-world.inex                 # run an .inex file (also ./hello-world.inex)
into build --test examples/Memory     # run Into #[test] files under src/tests in the VM
into format examples/Json --depth 2   # format .into files (also -format / -depth)
into install packages/json            # add a package to the current project
```

## Language specification

This section describes the language as implemented. A rule that is listed here
has a test in the `tests/` directory of the crate that implements it.

### Lexical structure and syntax

- Source is a sequence of tokens. The language is whitespace and newline
  independent: tokenizing, AST construction and formatting never look at source
  lines. A missing `;` is accepted before a `}` or a line break.
- `//` starts a line comment. Braces inside strings and comments are ignored by
  the formatter.
- `///` is a doc comment. A run of consecutive `///` lines documents the
  declaration that follows on the next line, attribute lines in between
  included; a blank line, a plain `//` comment or code on the same line as the
  comment documents nothing. The text after `///` (and one space) is Markdown.
  Only the language server reads it (`analysis`, `DocComments`); the tokenizer
  returns it as an ordinary comment.
- Keywords: `init`, `fun`, `pub`, `static`, `macro`, `into`, `now`, `late`,
  `const`, `unload`, `enum`, `if`, `else`, `while`, `break`, `return`, `throw`,
  `import`, `export`, `event`, `thread`, `await`, `abstract`, `default`,
  `override`, `as`. `as` is reserved; there is no cast expression yet.
- `=>` separates the parameters of a lambda from its body.
- Keyword, punctuation and operator spellings live in `parser`.

### Types

- Primitives: `Boolean`, `String`, `Path`, `Int8`, `Int16`, `Int32`, `Int64`,
  `Float32`, `Float64`, `Void` (results only).
- `Pointer` and `T*` are pointers, `List<T>` is a list, `T&` is a reference that
  owns nothing, and a generic parameter such as `T` stands for a type chosen by
  the caller.
- Classes, enums and the built-in classes `Object`, `Event`, `Listener`,
  `Thread` and `Task`.
- A value is assignable to its own type; integers convert to other integer or
  float types, floats to floats, `String` to `Path`, and an instance of a class
  to its ancestors where a parameter of an event expects the ancestor.
- `Object` declares an `onCast` event for conversions to come. Nothing converts
  with it yet.

### Declarations and memory

- `into x = ...;` declares a scope-local binding. `now into x = ...;` allocates
  it immediately, `late into x = ...;` allocates it when it is first used,
  `const now into x = ...;` is constant, and `unload x;` removes the memory.
- A local binding is in memory only inside an `init { ... }` block; `now` and
  `late` bindings are in memory outside it too. An unloaded binding cannot be
  printed.
- `into* x` declares a pointer binding.
- The entry point is `init { ... }`. Statements outside it run in module scope.
- A name is visible from its declaration to the end of the scope that declared it.

### Automatic destroy

- Every binding that owns an instance of a class inheriting `Object` is destroyed
  when its scope ends: the program (module scope), `init`, the branches of `if`,
  every iteration of `while`, inlined function bodies, event handlers and thread
  bodies. Bindings are destroyed in the reverse order of their declaration.
- `break` destroys what the scopes between it and its loop own.
- A binding that was moved or destroyed earlier is not destroyed again. When a
  nested scope moves a binding that an outer scope declared, a hidden `Int32`
  drop flag (`__drop_<name>`) decides at run time.
- `Process.destroy` and the other overriding `destroy` methods only release a
  handle; they never end a process.

### Control flow

- `if (cond) { } else { }`, `while (cond) { }`, `break;`, `return value;`,
  `throw "message";`.
- A condition compares a numeric or `Boolean` variable with a literal or another
  variable (`==`, `!=`, `<`, `<=`, `>`, `>=`), calls a function returning
  `Boolean`, or reads a property of a binding (`x.isOwned`, `!x.isMoved`,
  `x.isInMemory == false`). A comparison between two literals is decided while
  lowering.
- `throw` only runs in the VM (tests); compiled code rejects it.

### Functions

- `pub fun name(params) { ... }`; `static` for functions without a receiver.
- A function body is inlined where it is called, with its arguments substituted.
  Recursion is rejected after 32 nested calls, and the statements allowed in an
  inlined body are declarations, assignments, `while`, `if`, calls, `print`,
  `return` and `throw`.
- A foreign function is declared with `pub c fun` or `pub cpp fun`, no body, and
  `#[link("c" | "cpp", "src/ffi/file", "symbol")]`. Its arguments are strings,
  32-bit integers, slots and fields of instances.
- A lambda `(a, b) => { ... }` is the handler of an event; it captures the
  names it mentions.

### Classes and objects

- `pub Name(): Parent { ... }` declares a class; `: Parent` is optional and
  every class without a parent inherits `Object`. Fields are
  `pub into: Type name = default;` and static fields (constants) are
  `pub static into: Type NAME = value;`. A class with only fields is built
  with `Name()`.
- Fields of the parent come first in the layout of a subclass.
- `Object` provides `toString`, `hash`, `copy`, `move`, `borrow` and `destroy`.
- Members are final unless they are `abstract` or `default`. A subclass changes
  one with `override`; the signature must match, except that a member returning
  the class that declares it may be overridden by one returning the overriding
  class. `static` members follow the same rules. An abstract class cannot be
  instantiated, and a concrete class must implement every abstract member.
- Methods are inlined with the receiver as `this`.

### Ownership

- A binding owns the instance it holds. `x.copy()` makes an independent owner,
  `x.move()` hands the instance over and leaves `x` moved, `x.borrow()` makes a
  reference (`T&`) that owns nothing and locks the owner against `move` and
  `destroy`, and `x.destroy()` ends the instance.
- A parameter of type `T&` borrows the argument for the call and gives it back
  afterwards. A function that returns `T&` may only return a reference to an
  instance that outlives it.
- `#[consumes]` on a method says the call moves the receiver, and
  `#[consumes("detached")]` says it detaches it (`Process.detach`). A reference
  cannot call such a method.
- `#[inspects]` on a function lets it receive moved or destroyed bindings and
  read their state without changing it; the test assertions use it.
- Diagnostics: `` `x` was moved ``, `` `x` was destroyed ``, `` `x` was detached ``,
  `` `x` is borrowed ``, `` `x` does not own its instance (`m` requires ownership) ``,
  `returns a reference to a local`.

### Events

- `pub event name(a: Type, b: Type) {}` declares an event; the body is its
  default handler, which runs first.
- `name.on(handler)` registers a handler and returns a `Listener`;
  `listener.destroy()` unregisters it. `name.emit(args)` calls the handlers in
  registration order with arguments checked against the declaration.
  Handlers registered during an emit wait for the next one. An event keeps at
  most 64 handlers. `name.destroy()` removes them all and blocks later use.
- A binding holds an event by name, so an event assigned to a binding is the same
  event. A listener leaving its scope unregisters its handler.
- Events can be exported with `pub`.

### Threads

- `thread { ... }` is a thread expression; `t.run()` starts it and returns a
  `Task`. `await task;` or `await t.run();` waits for the thread. Without
  `await` the program does not wait.
- A thread body captures the names it mentions, as a handler does, and destroys
  its own bindings when it ends.
- In the VM threads are cooperative: they switch when a thread waits with
  `await`, ends, or calls `sleep`.

### Enums, macros and attributes

- `enum Format { Text, JSON }` has uppercase names and optional `= n` values.
- `pub macro test { #[test] }` declares an attribute for the test runner.
- Attributes: `#[test]`, `#[link(...)]`, `#[entrypoint]`, `#[consumes]`,
  `#[inspects]`.

### Modules and packages

- `import "scheme://module";` where the scheme is the alias of a package;
  `intolang` resolves to the builtin package (`alias` `builtin`).
- A package exposes its API through `src/main/into-info.into` with
  `export "alias://module";` lines (every `pub` declaration in the module becomes
  an exported name) or `export "alias://module/item";`. Declarations without
  `pub` are private. Library packages list the modules to export in
  `src/main/lib.into`.
- Imports are followed transitively.

### The Into VM

- `intovm` loads a lowered message, resolves structured control flow to jumps
  and runs it. Slots, instances, events and threads are virtual; there is no
  LLVM.
- Memory: the VM tracks, per binding, the storage (`local`, `now`, `late`),
  whether the memory exists, and the owner, reference, borrow, moved, destroyed
  and detached state. The properties are `isInMemory`, `isOwned`, `isReference`,
  `isBorrowed`, `isMoved` and `isDestroyed`.
- Threads switch at `await`, at the end of a thread and at the native function
  `standard_sleep`, in round-robin order; threads that wait for each other are
  reported as a deadlock.
- Processes: tests and `eval_vm` run on a virtual process table with a virtual
  clock, a signal queue per process and programs that behave like `true`,
  `false` and `sleep` (`ChildBehavior` scripts others). `ps` functions, `sleep`
  and `monotonicMilliseconds` act on it, so nothing real is started, signalled or
  waited for. An application run from an `.inex` file works with the real
  process, signals and time.
- `DummyProcess` (`intolang://builtin/tests/dummy-process`) extends `Process` for
  tests, with `SIGNALED`, `current`, `spawn`, `isAlive` and `advance`. Its
  functions exist only in the VM; compiling code that uses them fails with
  `DummyProcess requires the Into VM`.
- Native calls: the `.inex` file carries the manifests and the `src/ffi`
  directories of the packages whose native sources a message calls, under
  `.into/packages/<alias>/`. The VM unpacks the tree, finds the sources the way
  `native::collect_sources` does, compiles them into a shared library under
  `INTO_CACHE_DIR`, `XDG_CACHE_HOME/into` or `~/.cache/into`, keyed by the
  content of the tree, and loads it with `dlopen`. A call passes at most six
  integer or pointer arguments (text as C strings, instances the VM made as C
  structs that are read back after the call). Instances that native code
  allocated are read in place from the layout of their class. A function the VM
  has as an intrinsic wins over the library.
- `.inex` format: the line `#!/usr/bin/env into`, then binary data in little
  endian: the magic `INEX`, the format version (`u32`, 2), the name of the tool
  that wrote the file (`u32` length and bytes), the number of entries (`u32`),
  the entry table and the data. An entry is its path (`u32` length and bytes),
  its kind (`u8`, 0 a directory and 1 a file), its mode (`u32`), its size
  (`u64`), the offset of its contents in the data (`u64`) and the SHA-256 of its
  contents (32 bytes); equal contents are stored once. The entries are a
  `Bundle`, a copy of the project: `into-package.json`, `src/**`, `res/**`,
  `.into/packages/<alias>/` for the packages with native sources, and `intoc`,
  the program, which is the lowered message as text. A file that is not `.inex`,
  has another version (the text format of version 1 included), is truncated,
  holds an entry outside its data or has contents that do not match their hash
  is rejected before it runs; so is one without `intoc`.
- Choose `eval_vm` for tests that need no LLVM and no real process (or
  `eval_vm_bundled`, behind the `run` feature, when they import the bundled
  packages such as `ps`), and `eval_run` for tests that compile and run a native
  program.

### Testing

- A test file under `src/tests/` imports the `test` macro and `assert`, plus the
  helpers it uses. Functions or methods marked `#[test]` are run one by one: the
  runner lowers the body of each as a program (`backend::lower_test`, where a
  call to a function that was not imported is an error) and runs it in the VM.
  A failed assertion, or a lowering error such as using a moved binding, fails
  the test.
- Helpers: `assert`, `assertTrue`, `assertEquals`, `assertMemoryExists`,
  `assertMemoryNotExists`, `assertOwned`, `assertNotOwned`, `assertReference`,
  `assertNotReference`, `assertBorrowed`, `assertNotBorrowed`, `assertMoved`,
  `assertNotMoved`, `assertDestroyed`, `assertNotDestroyed`. Their names start
  with `assert`.

### Assets

- `import "asset://require";` gives `require(directory)`, `asset(path)` and the
  class `Assets`. `require("res");` and `asset("res/a.txt")` name what to
  bundle: a directory of the package, or one file of it. Every such call with a
  literal path in the lowered message (the native functions `asset_require` and
  `asset_path`) adds that directory or file, below the same path, to the bundle
  of the `.inex` file or of the native executable; a path the package lacks is
  an error. `asset` gives its path back, so `Assets.read(asset("res/a.txt"))`
  bundles the file and reads it.
- `Assets.read(path)` returns the contents of a file of the bundle as a
  `Binary`, and fails with `The asset ... is not in the bundle` when the bundle
  lacks it. Nothing is unpacked: the VM reads the bundle it holds in memory, and
  a native executable carries the bundle as the object `into_bundle_data`
  (written by `native::embed_bundle` with `.incbin`) and copies the contents of
  the file out of it. Embedding needs a GNU compatible assembler and an ELF
  target.

## Packages and the CLI

- Commands: `new`, `install`, `uninstall` (`rm`), `build` (default), `run`,
  `format`. A bare path defaults to `build`, except a path ending in `.inex`,
  which runs.
- Legacy single-dash flags are accepted and normalized: `-test`, `-run`,
  `-format`, `-depth`.
- The progress line of a command (`progress::Progress`) shows a spinner in its
  brackets and a green check mark when `finish` ends it. A `Progress` that is
  dropped before `finish`, as when the build fails, keeps its last message and
  shows a red cross instead (`[✗]`, also on a stream that is not a terminal).
  `build`, `install` and `uninstall` start their `Progress` before they look for
  the manifest, the project or the package, so a package that is not found fails
  inside it. The CLI writes the first line of a failure (the notes and the message)
  in the same line format with `progress::fail`, through `ErrorReport::emit_with`;
  the stack trace and the error code follow it.
- `build --vm` writes an `.inex` file instead of calling LLVM and the linker,
  for executables only; `run --vm` builds it and runs it. `build --test` runs the
  tests in the VM.
- `into new` flags: `--licence` (default `MIT`), `--version` (default `0.1.0`),
  `--alias`.
- Manifests: a package must specify exactly one of `runs` (executable) or
  `library`. Library entry must be `src/main/lib.into`. Aliases must be
  non-empty, not `.`/`..`, and `[A-Za-z0-9._-]` only.
- `into install <dir>` links `.into/packages/<alias>` to the package directory
  (a symbolic link, no copy) and records it in the project manifest's
  `packages` map and in `into-package-lock.json`. The lock file lists each
  package as a `moduler::PackageInfo` (alias, name, version, licence, content
  `hash` that ignores `.git` and `dist`, and the `location` of the package,
  relative to the project). Builds reference the links: the temporary package
  environment symlinks every installed package instead of copying it.
- The compiler embeds the builtin packages and every build links them into the
  package environment by default; they need no `into install`. An installed
  package replaces the embedded one only when the project manifest's `packages`
  map gives it an explicit, non-empty version (which must equal the installed
  package's version); without such an entry the compiler's version is used.
- The compiler embeds the `intolang` (alias `builtin`) package, so projects
  need not list it. `into install builtin` (or `intolang`, or the package's
  directory) references the compiler's copy: nothing is linked, and the lock
  entry has no `location` (nor `hash`). Restoring such an entry only checks
  that the compiler's version equals the locked version.
- `into install` without a directory restores the links of the packages in the
  lock file from their locations and fails if a package's hash differs from the
  lock.
- `into uninstall <alias>...` (alias `into rm`) removes the link (never the
  package itself), its `packages` entry and its lock entry.
- Adding a system module: create the module package, register it in
  `packages/system/into-package.json` under `modules`, add `lib.into` /
  `into-info.into`, native sources, and any `nativeDependencies` keyed by the
  source path relative to the package root.

## Language server

The language server is an app of its own; the `into` CLI has no command for it.

```
editor ── LSP (stdio) ── into-lsp ── ipc (stdio) ── intolang-ipc (analysis::Analyzer)
```

- `into-lsp` (`packages/lsp`) starts `intolang-ipc` as a child process, found by
  `initializationOptions.analyzerPath`, `INTOLANG_IPC` or the `PATH`, and starts
  it again with the open documents replayed when it died. It converts positions
  (the IPC counts characters, LSP counts UTF-16 units) and types, nothing else;
  it must not depend on a compiler crate (`cargo tree -p lsp`).
- `ipc` is the only contract. Its positions and messages know no editor. Every
  `document/open`, `document/change` and `document/close` is answered by one
  `analysis/diagnostics` notification; requests are answered in order.
- `analysis` owns what Into means: it keeps the open documents, prefers their
  text over the files for imported modules (`moduler::SourceProvider`), reports
  a syntax error (its token range), imports that name nothing (`0x10AA`) and the
  first error of `backend::lower_file` (the line of its Into backtrace frame),
  and keeps the last analysis that parsed for definition, hover and symbols.
  Names resolve by the token under the cursor: a binding visible at that line,
  a top-level declaration, a public declaration of an imported module, or a
  member found through the type of the receiver (`a.b.c` through the declared or
  inferred type of `a`, then the parents, then `Object`).
- The packages the analyzer resolves imports against are linked into a temp
  directory like a build does: the builtin packages from `packagesDir`,
  `INTOLANG_PACKAGES` or a `packages/intolang` above the project, then the
  project's `.into/packages`.
- Tests of `lsp` use a scripted `ipc::Handler` over `memory_pair`; tests of
  `analysis` call the `Analyzer` as a `Handler`.

## Architecture / data flow

`into build <pkg>`:
`src/commands/build.rs` reads `into-package.json` → assembles a package
environment in a temp dir (embedded builtins + project `.into/packages` deps) →
`backend::lower_file` tokenizes (`tokenizer`), builds the AST (`ast`) and
walks it to produce the lowered message → `compiler::compile_with_packages`
runs LLVM codegen, `native::compile_sources` compiles any imported C/C++
sources, and `lld` links a binary/shared library.

`into build` and `into build --vm` lower the entry point the same way and hand
the message to a `Target` (`src/commands/target.rs`): `NativeTarget` compiles
and links an executable, `InexTarget` makes a `Bundle` of the project
(`src/commands/inex.rs`, with the native sources found by
`native::collect_sources`) and writes the `.inex` file. `into <file>.inex` and
`into build --test` hand a message to `intovm::Vm`.

- The language is whitespace and newline independent: tokenizing, AST
  construction and formatting work on tokens only, never on source lines.
- The lowered "message" is a line-based text format. Compiler directives are
  `parser::Record`s: the record separator `\u{1e}`, a tag from
  `types::MessageTag` and tab separated fields; any other line is program
  output known at compile time. `backend` writes them (`backend/src/emit`);
  `packages/codegen/src/program/message.rs` and `packages/intovm/src/program/load.rs`
  read them. Keep all three in sync through `MessageTag`, `Record` and the shared
  `native` types (`LinkCall`, `Comparison`, `Operand`, `NativeArg`,
  `OwnershipChange`, `StateCheck`) when adding a tag.
- Tags: `NATIVE_CLASS`, `NATIVE_ASSIGN`, `NATIVE_ARITH`, `NATIVE_PRINT`,
  `NATIVE_PRINT_CONST`, `NATIVE_FIELD_PRINT`, `NATIVE` (plain call),
  `NATIVE_LINK_ASSIGN`, `NATIVE_CONSTRUCT`, `NATIVE_MODULE`, the branches
  `NATIVE_IF_BEGIN`/`_CALL`/`_CONST`/`_STATE`/`_ELSE`/`_END`, the loops
  `NATIVE_WHILE_BEGIN`, `NATIVE_LOOP_BEGIN`, `NATIVE_LOOP_END`, `NATIVE_BREAK`,
  `NATIVE_ALIAS` (`move`, `borrow`) and `NATIVE_COPY`, the events
  `NATIVE_HANDLER_BEGIN`/`_END`, `NATIVE_EVENT_ON`, `NATIVE_EMIT`,
  `NATIVE_EVENT_OFF`, `NATIVE_EVENT_DESTROY`, the threads `NATIVE_THREAD_RUN`
  and `NATIVE_AWAIT`, and `NATIVE_OWNERSHIP` and `NATIVE_THROW`.
- `NATIVE_OWNERSHIP`, `NATIVE_IF_STATE` and `NATIVE_THROW` exist for the VM.
  Codegen treats the ownership records as no-ops and rejects the other two, as
  it rejects the functions `native::is_vm_only` names.
- `backend` tracks ownership statically (`Tracking`, `Ownership`), rejects
  invalid use while lowering and emits the scope-end `destroy` calls, with drop
  flags where a nested scope may have moved a binding. The scope stack is
  `state.scopes`.
- `backend` must not depend on LLVM. `codegen` owns the LLVM node code and
  `compiler` depends on `backend`, `codegen`, `native` and `lld`. `intovm` depends
  on `native`, `parser` and `types`, never on LLVM, `compiler`, `codegen` or `lld`.
- `apps/compiler/build.rs` copies resources and embedded packages into `OUT_DIR`
  and exports `COMPILER_BUNDLED_RES`. It re-runs when `res/`,
  `packages/intolang`, `packages/system/modules`, or workspace packages change —
  so edits to bundled packages take effect after a rebuild.
- Errors are values of `errors::Errors`, carried by `errors::Error`; `anyhow` is
  not a dependency. Each variant has its message (`#[error]`) and its code, a hex
  discriminant written in the enum (`Syntax { .. } = 0x0001`). `Errors::Unknown`
  (`0xFFFF`) is the only variant without a code. The variants belong to two
  domains declared in `declare_errors!`:
  - `IntoLang` (`0x0001..=0x7FFF`): errors of the user's Into program. `0x0xxx`
    lexing and syntax, `0x1xxx` names and types, `0x2xxx` ownership, `0x3xxx`
    events and signals, `0x4xxx` classes and enums, `0x6xxx` VM run time, `0x7xxx`
    the test runner. Their stack trace goes Into → Rust → Native → raw panic; the
    Into layer (`IntoLangStackTrace`) is the one `Into language backtrace:` format.
  - `Compiler` (`0x8000..=0xFFFE`): errors of the toolchain. `0x8xxx` packages and
    manifests, `0x9xxx` native build and records, `0xAxxx` codegen, LLVM and LLD,
    `0xBxxx` IO, bundle, inex and the CLI, `0xCxxx` internal inconsistencies. They
    never carry the Into layer: Rust → Native (only when an external tool or
    library failed, with its stderr as the output) → raw panic.
  - To add an error, add a variant with the next free code to the right domain in
    `packages/errors/src/errors.rs` (messages are written once, there), raise it
    with `Errors::X { .. }.into()` and add a test where its crate keeps its tests.
    `tests/catalog.rs` of `errors` checks that codes are unique and in range.
    Do not add variants that only wrap a `String`.
  - `errors::Context` adds notes (`.context(..)`, `.with_context(..)`) that print
    before the message, outermost first. Syntax errors collect an `ErrorContext`
    (path, `SourceSpan`, expected, found, hint), the shape a future LSP reads.
  - The CLI writes a failure through `ErrorReport`, chosen once by
    `ReportFormat::reporter()`: `--error-format human` (the default, stderr, with
    `error code: 0x....`) or `--error-format json` (stdout, one object with
    `context`, `notes` and `trace`). `errors::install_panic_hook()` keeps the raw
    panic information.

## Conventions and gotchas

- Edition 2024; use `errors::Result` and `errors::Errors` for errors, `serde`/`serde_json`
  for manifests. No comments unless necessary.
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
  `testing::eval`, `testing::eval_with` (lower to a message),
  `testing::eval_vm` (lower and run in the VM) or `testing::eval_run` (compile
  and run; needs the `run` feature and LLVM, so run it inside `nix develop`).
  Tests that start programs create their projects in a temporary directory.
- Handlers and thread bodies capture the names they mention: a captured slot
  becomes a global shared with the main flow, and a name the body declares itself
  stays local to it.
- The formatter uses 4-space block indentation and collapses consecutive blank
  lines; `//` are line comments (ignore braces inside strings/comments).
- There is no CI configuration, test script, or lint config beyond `cargo`
  defaults; run `cargo fmt` / `cargo clippy` / `cargo test` manually. The nix
  shells carry the `clippy` and `rustfmt` that match their `rustc`.
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
