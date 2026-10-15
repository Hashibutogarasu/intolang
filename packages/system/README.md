# Into System Modules

This project contains the language-specific system module projects under
`modules/c` and `modules/cpp`. Their package aliases are the import schemes:

```into
import "c://wayland/client";
import "c://vulkan/core";
import "cpp://standard/string";
```

Each project publishes its module surface through `src/main/lib.into` and
`src/main/into-info.into`. Foreign declarations use `pub c fun` or `pub cpp
fun` and keep C and C++ names in Into source.
