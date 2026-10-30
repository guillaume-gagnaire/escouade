# Third-party notices

Escouade is licensed under GPL-3.0-or-later (see [LICENSE](LICENSE)). Parts of its code are ported
from other projects, under their own licenses, reproduced here. The libraries it links are listed,
with their licenses, in [CREDITS.md](CREDITS.md).

## tokensave

The code index (`src-tauri/src/codeindex/`) ports the method of tokensave: the resolution of calls to
symbols (`resolver.rs`, `links.rs`), the typed receivers of method calls (`receivers.rs`), the names
of the call sites and imports (`lang.rs`), the answer by definition of a name (`calls.rs`), and its
tests (`codeindex_links_tests.rs`). Each of these files says in its header which file of tokensave
it comes from.

- Project: https://github.com/aovestdipaperino/tokensave
- Commit: `580936b` (version 7.17.0)
- License: MIT

```
MIT License

Copyright (c) 2025 Enzo Lombardi

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
