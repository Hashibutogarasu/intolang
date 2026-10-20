# Errors

Projects that fail on purpose. Each one is built with the VM target, so no LLVM is needed, and ends with the error in the table.
Run one from the repository root:

```sh
into build --vm examples/Errors/SyntaxError
into build --vm --error-format json examples/Errors/SyntaxError
```

| Project | What is wrong | Domain | Code | Message |
| --- | --- | --- | --- | --- |
| `SyntaxError` | `into message = ;` has no value | Into language | `0x0001` | `Expected an expression` |
| `TypeMismatch` | a `String` is stored in an `Int32` | Into language | `0x100a` | `Type mismatch: expected `Int32` but found `String`` |
| `OutOfScope` | a binding outside `init` is printed | Into language | `0x1040` | `Variable `number` is out of scope here` |
| `MovedBinding` | `counter` is read after `counter.move()` | Into language | `0x2002` | `` `counter` was moved `` |
| `InvalidManifest` | the manifest has both `runs` and `library` | Compiler | `0xb303` | `Package manifest must specify exactly one of `runs` or `library`` |

Errors of the Into language print the `Into language backtrace:` with the file and the line, and the JSON report carries the same place as `span`.
The error of the compiler has no Into language layer: its JSON report has an empty `trace` unless `RUST_BACKTRACE` asks for the Rust layer.
