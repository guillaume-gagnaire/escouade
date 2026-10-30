# The Docs space

The Docs tab holds the product’s specification, by subject, with what sets it apart from the code; its decisions and its documents. Everything lives in the repository, and you share it with the team in one click.

## The tab

The sidebar has three tabs: “Agents”, “Kanban” and “Docs”. Under “Docs”, “New discussion”, [the discussions](#the-discussions), then three spaces:

- **Specification**: the pages of the spec, with the number of gaps with the code.
- **Decisions**: the register of the decisions on the product, numbered and dated.
- **Documents**: the files dropped for the project (requirements, minutes, mockups).

Everything is written under `.escouade/` of the [docs branch](#the-docs-branch): `.escouade/spec/<subject>/<page>.md`, `.escouade/decisions/`, `.escouade/documents/`. They are Markdown files you can also read and edit by hand.

## The specification

The space opens on **the map of the subjects**: a card per subject, with where it stands against the code.

- “Up to date”: the last comparison with the code found nothing.
- “1 gap”: a page of the subject says something else than the code.
- “Draft”: all its pages are still drafts.

Each card also says how many pages, tickets and decisions the subject has. The last card, “What changed”, gives the last shared changes of the spec, and the pages changed here, not shared yet. At the head of the space: the pages up to date, the gaps, the drafts, and when the spec was last compared to the code.

On the left, **the tree of the pages**, by subject as in a wiki: a click on a subject collapses or expands it, a click on a page opens it. A page marked “gap” says something else than the code; “to share”, it was changed here and not shared yet.

“New page” creates a page: a subject (“Payment”) and a title (“In instalments”). It opens ready to be written.

## A page

A page shows its Markdown text formatted and, beside it, its sources (the documents, files or links it comes from), its tickets and decisions, and when it was last compared to the code.

When the comparison found **gaps with the code**, they are at the top of the page, each with what differs and the files concerned:

- “Update the spec” opens a [discussion](#the-discussions) with the spec writer, told the gap: it reads the code and proposes the page that says what the code does; poured into the spec, the gap goes.
- “Open a ticket for the code” opens the window of a new ticket, its title and description already written from the gap: the code is what must follow the spec.
- “See the code” opens the file the gap names in the [editor](editor).

“Edit” opens the page in a simple editor: its title and its text. “Save” writes the file, its header (sources, tickets, decisions, gaps) kept as it is. With the current branch, “Open in the editor” opens the page’s file in Escouade’s editor.

## The discussions

“New discussion”, at the top of the tab (Ctrl+B), lets you talk about the spec with an agent. You choose one of two:

- **Discussion**: the spec writer, to ask the spec or update it simply (“What does the spec say about paying in instalments?”, “Add that the tax receipt goes by email”).
- **Brainstorm**: a product agent, in the manner of BMAD, that matures an idea: uses and journeys, discovery, compliance and GDPR, what other trades do. It proposes features, spec pages, decisions, and tickets, which it names without making them.

You give a subject. No need to choose documents: the agent has them all, through the index. You can **join new files**: they are filed and indexed by themselves in the Documents. The window’s footer shows the agent’s harness, account and model, the project’s by default, and lets you change them.

They are agents of Escouade like the others: the same conversation, and their questions are answered from the list of agents too. They work in the folder of the docs branch, read the code without ever changing it and only write the docs, through Escouade’s docs tools. Their harness itself keeps them from it, in every mode: Claude Code has its tools that write taken away, Codex only reads in its sandbox, OpenCode, Copilot CLI and Antigravity CLI have their writes and commands refused. Cursor CLI and Mistral Vibe change files without going through Escouade: no discussion runs on them, and the window says so.

The discussions are cards under “New discussion”, each marked “Brainstorm” or “Discussion”, with its state: “Question”, “Answering”, “Draft” (changes proposed not poured yet), “Poured”.

A discussion open shows the conversation on the left, and on the right **“What will go in the spec”**: the changes proposed page by page (a new section, the sentences taken away struck through and the new ones in green) and the decisions proposed. A cross takes a proposal out. **“Pour into the spec”** writes the pages on the docs branch, and the decisions in the register, proposed: they are accepted in one click in the Decisions. Nothing is shared: the status bar says how many changes wait for “Share”.

“Ask the spec”, at the top of the specification, opens a discussion with the spec writer. A project without a spec offers “Write a first spec from the code”: the spec writer reads the code and proposes a first version, filed by subject (it costs tokens, hence the gesture).

## Plan tickets: from the spec to the tickets

Once the spec changed, **“Plan tickets”** asks a planner for the tickets that will implement it. The button is on a discussion whose changes were poured, and at the top of the specification while changes have no tickets (“3 changes without tickets · Plan tickets”): the pages poured or written by hand, and the decisions proposed, since the last planning.

The planner reads these changes and decisions, the code (the files the code index links to them, and the project’s folder, without changing anything) and the tickets of the board. It makes the tickets in “To do”, each with its description, its acceptance criteria, the tickets it comes after (“After”) and the pages and decisions it implements, which its description names for its agent. A ticket the board already has, even done, is not made again: it is said “On the board already”. The planner does not start the tickets: the autopilot and the board’s rules apply as to any ticket.

What it made shows under the discussion, or on the map of the subjects, with **“Undo”**: the tickets not started yet are removed, and the changes are to plan again. A subject’s card shows the keys of its tickets, and a ticket’s card on the board the pages it implements, which open in one click in the Docs tab.

## The spec always up to date

The spec follows the code in two ways.

- **The tickets’ agents keep it.** Each ticket’s instructions ask its agent, when the ticket changes how the product behaves, to update the pages it touches and to leave a decision that says what changed and why, before handing it in. Nothing stops the approval: the review is where it shows. With the current branch, those changes go in the ticket’s branch and are merged with its code. With a [fixed branch](#the-docs-branch), they wait in a copy Escouade keeps, and are poured on the docs branch **only when the ticket is approved**: the spec never tells of a behavior not merged yet. What the team wrote on the branch meanwhile stays.
- **The comparison to the code.** After each ticket approved and merged, a short agent reads the code, changing nothing, and compares the pages the ticket touches: those it changed, those that name it, and those that talk of its files or their symbols (by the [code index](code-index) when it is on). What it finds shows on the map of the subjects (“1 gap”, “compared to the code 2 h ago”) and at the top of the page. Escouade keeps it on this computer, not in the page: a comparison leaves nothing to commit.

“Compare to the code” does it on demand: at the top of the specification for every page (40 at most, those compared longest ago first), or at the top of a page for that page alone. Each page compared costs tokens, on the account and harness of “Model for written texts”.

## The decisions

The Decisions space is a register, not a document: a row per decision on the product, the latest first, with its number, its title, where it comes from (a brainstorm, a discussion, a ticket, a document), its state, what it changed in the spec and its date. On top: how many decisions are accepted, proposed and superseded. The “All” menu filters the register by state.

A click on a row opens its detail on the right: the context, the decision, its consequences, the spec pages it changed (with the lines added and removed; a click opens the page), its links (tickets, origin, documents), and what it replaces or what replaces it.

- **Proposed**: an agent, or someone of the team, proposes it; it is a decision only once accepted. “Accept”, on its row or in its detail, accepts it in one click, in your name.
- **Accepted**: it is no longer rewritten. To go back on it, “Replace it” proposes a new decision that replaces it.
- **Superseded**: the new decision was accepted; the old one stays in the register, as it was, and says by which.

“Propose a decision” writes one by hand: its title, its context, the decision, its consequences, and the accepted decision it replaces if there is one.

The agents propose their decisions through the [MCP server](mcp). The one a ticket’s agent leaves is accepted when you approve the ticket: it goes in the same commit as its code. Each decision is a file of `.escouade/decisions/`, numbered (`0014-family-rate.md`), shared with the other docs.

## Importing an existing spec

A project without a spec starts from an empty space. When Escouade finds a spec in the project (a `SPEC.md` at the root or in `docs/`, a `spec` or `specs` folder, a `docs/` folder of Markdown files), it offers it:

1. “See how it would be cut” shows how Escouade would cut it, by its titles: a subject per main title, a page per section; in a folder, a subject per file. A lone title at the top of the file, the document’s name, becomes the introduction’s title.
2. Untick the pages you don’t want.
3. “Import” writes the pages. Each one cites the file it comes from and keeps its place in the order of the original spec.

Nothing is written before “Import”, and a page that already exists stays as it is. The original file is neither changed nor removed: removing it from the repository is up to you.

Escouade keeps the imported paths with the docs, without duplicates, and shares them with the team. As soon as the project has a spec in Escouade, every agent Escouade launches on it (ticket, discussion, planner…), whatever its harness, is told that the spec and the project’s decisions are in Escouade and are the only ones to follow; the old file is named: it is no longer read as the spec nor updated.

## The documents

The Documents space is the project’s drive: requirements, meeting notes, mockups and legal papers, in folders under `.escouade/documents/`.

- On the left, **the folders**, with how many files each holds; “All documents” shows them all. A folder that holds others unfolds.
- On the right, **the files** of the folder open, each with who added it, its size, its date, and what the agents did with it: “✓ read by the agents” once an agent read it, “cited 3 times” when pages of the spec or decisions cite it.
- A click on a file shows the text Escouade read of it.

**Dropping files.** Drag files onto the space, or “Drop files”. Each file is **read and indexed on your machine**: nothing goes to the Internet. Escouade reads the text of PDFs, Word (`.docx`) and OpenDocument (`.odt`) files, Markdown and text; of an image, it keeps its format and size only. The text read stays in Escouade’s folder, never in the repository; the file itself goes into the repository, on the docs branch.

**Filing.** A file dropped in an open folder stays there. Dropped on “All documents”, Escouade files it by itself, in this order:

1. the folder whose name its name or its title bears (“Meeting notes workshop” goes into “Meeting notes”);
2. the folder of its nature, when the drive has one: meeting notes, requirements, a mockup, a legal paper, a quote or an invoice, a presentation, a planning;
3. the folder whose files are nearest to it by the meaning, when the [vectors’ model](shared-memory) is downloaded, then by the words;
4. otherwise, a **new folder**, made for it: the one of its nature (“Planning”), “Images” for an image, or “Other”.

Escouade says where it filed it and why; “Move to” puts it in another folder. You can also drag a card onto a folder on the left. “New folder” makes a folder in the one open.

**Search the documents** searches the names and the text of all the documents, by the meaning and the words when the vectors’ model is there, by the words otherwise, and shows the passage that answers.

**Heavy files.** When the repository has Git LFS rules (`.gitattributes`) that take the file, they apply: nothing to do. Otherwise, past 50 MB, Escouade warns before adding it, because git keeps every version in full and everyone on the team downloads it: “Add anyway” or “Don’t add them”.

**The agents** search and read the documents through Escouade’s tools (`search_documents`, `read_document`), whatever their harness, and cite the ones what they write comes from in the sources of a page of the spec. A private project’s documents are seen by its own agents only.

## Sharing the docs

While the Docs tab is open, or while the docs have changes, the status bar says “Docs on the branch main · 2 changes to share”. The changes are the files of `.escouade/spec`, `decisions` and `documents` changed and not sent yet.

“Share” first brings in what the others shared, then saves (commits) the files of the docs, and them alone, and sends them (push). The code you change meanwhile is left alone.

- When the branch also has commits of code not sent yet, Escouade tells you first: “Share everything” sends them with the docs, “Cancel” does nothing.
- When two people changed the same pages, the status bar names them: “The agent resolves” hands the conflict to a writer agent, who keeps what each one meant; “Cancel” puts the docs back as they were. You share again once it is done.

> [!NOTE]
> Escouade never shares by itself: nothing is committed or pushed without a click on “Share”.

## The docs branch

Settings › Projects › Git, “Docs branch”:

- **The current branch** (by default): the docs live in the branch of each folder. Those of a ticket follow its branch and are merged with its code.
- **A fixed branch** (`docs`, for instance): the docs always live on that branch, which Escouade keeps in a worktree of its own, in its system folder. Escouade makes it when needed, from the remote branch when there is one, and never merges it anywhere: that is up to the team.
