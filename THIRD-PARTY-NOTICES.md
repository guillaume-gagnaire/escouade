# Third-party notices

Escouade carries code ported from the projects below. Each ported file says at its top which file it comes from. The licences of the libraries Escouade links are listed in [CREDITS.md](CREDITS.md).

## tokensave

- Project: [github.com/aovestdipaperino/tokensave](https://github.com/aovestdipaperino/tokensave)
- Commit ported: `580936b` (version 7.17.0)
- What: the full-text index of the code index, the reading of a question, the context of a task and its ranking (ESC-296).
- Ported into:
  - `src-tauri/src/codeindex/text.rs` ← `src/text.rs`
  - `src-tauri/src/codeindex/ranking.rs` ← `src/context/ranking.rs`, `src/tokensave/util.rs`
  - `src-tauri/src/codeindex/search.rs` ← `src/db/queries/search.rs`, `src/tokensave/query.rs`
  - `src-tauri/src/codeindex/context.rs` ← `src/context/builder.rs`, `src/types.rs`, `src/mcp/tools/handlers/graph.rs`, `src/graph/traversal.rs`
  - `src-tauri/src/codeindex/store.rs` (the full-text tables) ← `src/db/migrations.rs`
  - `src-tauri/src/codeindex/lang.rs` (`Walker::visibility`) ← the visibility rules of `src/extraction/*_extractor.rs`
  - `src-tauri/src/mcp/index.rs` (`retrieval_text`) ← `src/context/formatter.rs`

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
