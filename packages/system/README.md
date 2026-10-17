# Into System Modules

This project contains the language-specific system module projects under
`modules/c`, `modules/cpp`, and `modules/json`. The separate Wayland and Vulkan
third-party packages live in `packages/wayland` and `packages/vulkan`.

```into
import "wayland://client";
import "vulkan://core";
import "wayland://window";
import "cpp://standard/string";
import "json://value";
```

Each project publishes its module surface through `src/main/lib.into` and
`src/main/into-info.into`. Both use `export "alias://module";` declarations;
declarations without the `pub` modifier are private. Foreign declarations use
`pub c fun` or `pub cpp fun` and keep C and C++ names in Into source.
