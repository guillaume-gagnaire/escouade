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
  - `src-tauri/src/codeindex/format.rs` (the « Retrieval » section, until ESC-298 `retrieval_text` of `src-tauri/src/mcp/index.rs`) ← `src/context/formatter.rs`
- What, too: the resolution of calls to symbols, the ties kept apart, the answer of a name definition by definition, and their tests (ESC-297).
- Ported into:
  - `src-tauri/src/codeindex/resolver.rs` ← `src/resolution/resolver.rs`, `src/resolution/js_specifier.rs`, `src/go_import.rs`
  - `src-tauri/src/codeindex/links.rs` ← `src/resolution/touched.rs`
  - `src-tauri/src/codeindex/receivers.rs` ← `src/extraction/rust_extractor.rs`, `typescript_extractor.rs`, `python_extractor.rs` (the typed receivers)
  - `src-tauri/src/codeindex/lang.rs` (the names of the calls and imports) ← the call sites and `Use` nodes of `src/extraction/*_extractor.rs`
  - `src-tauri/src/codeindex/calls.rs` ← `src/mcp/tools/handlers/symbol_target.rs`, `src/db/queries/edges.rs`
  - `src-tauri/src/codeindex_links_tests.rs` ← `tests/resolution_test.rs`, `tests/ambiguous_calls_test.rs`, `tests/ts_phantom_call_edges_test.rs`, `tests/python_phantom_call_edges_test.rs`, `tests/resolution_order_independence_test.rs`, `tests/incremental_resolution_test.rs`
- What, too: the answers of the tools (Markdown, the code blocks and their cut, the body of a symbol, the literal search, the path filters, the paths ignored by the queries), the count of what they save, and their tests (ESC-298).
- Ported into:
  - `src-tauri/src/codeindex/accounting.rs` ← `src/mcp/tools/accounting.rs`, the accounting of `src/mcp/server.rs`
  - `src-tauri/src/codeindex/format.rs` ← `src/context/formatter.rs`, the text formats of `src/mcp/tools/handlers/graph.rs` and `info.rs`, `truncate_response` of `src/mcp/tools/handlers/mod.rs`
  - `src-tauri/src/codeindex/answers.rs` ← `handle_search`, `handle_literal_search` of `src/mcp/tools/handlers/graph.rs`, `handle_body` of `src/mcp/tools/handlers/info.rs`, `body_kind_preference` of `symbol_target.rs`
  - `src-tauri/src/codeindex/filters.rs` ← `QueryIgnore` of `src/config.rs`, `path_lists_keep` of `src/context/builder.rs`
  - `src-tauri/src/codeindex/context.rs` (`exclude_node_ids`, `include_code`, the path filters, the `index_body` handle) ← `src/context/builder.rs`, `src/types.rs`
  - `src-tauri/src/mcp/tools.rs` (the descriptions of the index's tools, their arguments in `src-tauri/src/mcp/index.rs`) ← `src/mcp/tools/definitions.rs`

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
