# Bad Apple!!

Renders the Bad Apple!! video as character art on a Vulkan (Wayland) window.

The video is decoded at runtime with FFmpeg (`packages/ffmpeg`), converted to a
character grid, and drawn with the Vulkan window's text renderer
(`packages/vulkan`).

```sh
into run "examples/Bad Apple!!"
```

`res/badapple.mp4` is required at runtime.

## License

The FFmpeg decoding and character-mapping approach in this example is ported
from [bad-apple-lab/Bad-Apple](https://github.com/bad-apple-lab/Bad-Apple)
(`badapple`, `decode_ffmpeg.hpp`, `encode_rt.hpp`, `font.hpp`), which is
distributed under the MIT License:

```
MIT License

Copyright (c) 2022 Elaina

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

The "Bad Apple!!" video itself is a Touhou Project derivative work; it is not
covered by the license above and is the property of its respective rights
holders.
