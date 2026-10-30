# The code index

Escouade sorts a project’s code into symbols and links, so that its agents find what they look for in one call, faster and for fewer tokens than by reading whole files.

## What it saves

Without an index, an agent searches with grep, then reads the files it found, often whole. With the index, it asks in one call:

- where a symbol is (function, method, class, type, field, Markdown heading), with its signature and doc;
- the context of a task: the entry points and the related symbols, with the code of the entry points (five blocks at most, of 1,500 bytes at most each; a block cut short names the call that gives the rest);
- the whole body of a symbol, by its name or its id;
- a text inside the code (an error message, a string), file by file and line by line;

Each answer ends with what it saved, counted as tokensave counts it: the files it touched, read whole, but **capped at four times the answer**, against what the agent paid for the call (the answer, its request and, once per session, the tools’ descriptions). A call that costs more than it saves carries its shortfall over to the next ones. The [statistics](stats) add up what is left per project.

The languages read: Rust, TypeScript, JavaScript, Svelte, Python, Go, Java, C#, PHP and Markdown.

## How it searches

The index searches by the **words of the code**, as [tokensave](https://github.com/aovestdipaperino/tokensave) does at its commit `580936b`, whose method Escouade carries over: a full-text index of the names, qualified names, docs, signatures and bodies of the functions, weighted by column, then a ranking by kind, visibility and callers. There are **no vectors for the code**: they were measured on 54 probes of Escouade’s own code, then dropped.

What follows from it:

- a question told with the identifiers of the code (in English here) finds better than a paraphrase; the tools’ descriptions ask the agents to write it so;
- when a term finds nothing, the answer says so (`### Retrieval`), and the agent asks again with synonyms in `keywords`;
- a text that is no symbol (a message, a string) is found by the literal search, not the ranked one;
- the paths of `.escouade/queryignore` (one pattern per line, `*` for a glob, `#` for a comment) are left out of the searches, never of the index; the file is read in the agent’s folder, never through a link that leads out of it;
- grep still wins where a regex is needed, or where the words of a task are none of the code’s.

## Turning it on

The index is **off by default**. Escouade offers it:

- for every project at once, in **Settings › Projects › Every project** (**Code index on every project**);
- when a project is added, at the last step of the “New project” assistant (**Index the code**), which starts from that setting;
- in **Settings › Projects › Code index** (**Index this project**), project by project, at any time.

Once on, the project is read once, then kept up to date without a file watcher: at each question of the agents (every 3 seconds at most), on commits, branch changes and the ends of turns. The setting says what it holds: its files, its symbols, its size and when it was read. Turning it off erases the index.

### On every project

Each project follows **Code index on every project** until it makes its own choice. A project turned on or off in its settings keeps its choice, whatever the app’s switch says; its section says where its state comes from (“Follows the app’s setting” or “Chosen for this project”) and **Follow the app’s setting** gives it back.

- **Turning the switch on** reads the projects that follow it one by one, never all at once, without blocking the window. A message says how many will be read; each one’s section says whether it waits for its turn, is being read, then what it holds.
- **Turning it off** erases the index of the projects that follow it. Escouade asks first, saying how many projects lose their index; those that made their own choice keep theirs.
- The model of the [search by meaning](#search-by-meaning) is never downloaded for this switch: it is still downloaded by hand.

A project turned on before this switch keeps its index, as a choice of its own; a project that was off now follows the switch.

The agents’ worktrees are not indexed whole: the index of the base branch serves them all, and each worktree only adds what differs from it.

## Nothing leaves the machine

The index is computed on your machine and kept in [Escouade’s folder](data), never in the repository nor in an online service. Only the agents get excerpts of it, as they already get code. Files git ignores are not read.

## Private projects

The index of a [private project](mcp#private-projects) serves its own agents alone: those of the other projects never reach it, by name or by id. What its fence refuses is never read nor indexed; when another project sits inside its folder, the setting names it: “Left out of the index: …”. Likewise, an open project that holds a private project inside its folder leaves it out of its index.

## Search by meaning

Besides the words, a project’s [shared memory](shared-memory) and docs are searched by meaning, with a small multilingual model (multilingual-e5-base, MIT license) that runs on your machine. It is downloaded once for every project, from **Settings › Projects › Code index › Search by meaning** (282 MB):

- a gauge shows how far the download is, and **Cancel** stops it;
- what came is kept: **Resume** goes on from there;
- the file is checked by its SHA-256 and goes through the proxy of the [network settings](settings-network);
- **Delete** removes it; the search then goes by the words alone.

Once the model is downloaded, nothing leaves the machine any more.

## What the agents see

The agents go through six tools of the [MCP server](mcp), the same for [every harness](harness): `index_search`, `index_context`, `index_body`, `index_callers`, `index_callees` and `index_impact`. Their answers are tokensave’s: Markdown for the context, one line per symbol for the search, the body as text. In the Claude Code and Codex views, a search aimed at a symbol gets a line saying the index answers it in one call; elsewhere, it is a rule of their instructions. A search is never blocked: unlike tokensave, Escouade stops neither a grep nor an Explore agent.

## What it saved

The [statistics](stats) have a **Code index savings** part, over the period chosen: per project, the number of answers of the index, the tokens saved (as counted above, the shortfalls paid first) and their worth in dollars. The dollars are an estimate: the tokens saved at the input price of the model each agent ran on.