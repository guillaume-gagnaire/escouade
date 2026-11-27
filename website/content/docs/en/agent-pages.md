# Agents’ pages

An agent shows you a page in Escouade, beside its conversation: a pre-merge review with its choices, a diagram, options to compare or its own HTML. It scrolls it and highlights what it is talking about, and you decide in the page, without the page being able to answer for you.

![A pre-merge review shown by an agent beside its conversation: the band “montre-moi-fake-en is driving”, the criterion highlighted, the numbered markers on the rail and in the conversation](/images/en/agent-pages.jpg)

## What it is for

Some things read better as a page than as text: the review of a batch of tickets with its criteria and changes, an architecture diagram, three design directions side by side. The agent fills a template with its data, or writes its HTML, and Escouade opens the page in a row of “Browsers”, beside its conversation. Then it guides you through it, gesture by gesture: “I scroll to criterion 3, the one that is stuck”.

This is the first of the two modes. The second, a real tab of the app being built or of a site, which the agent opens and drives, is in [agents and browsers](agent-browsers).

## What an agent shows

The agent calls `show_page` with a template and its data, or with HTML:

| Template | What it shows |
|---|---|
| “Review” (`review`) | An end-of-batch or pre-merge review: the ticket and its branch, the tests, the acceptance criteria (met, partly, unmet), the changes with their diffs, the risks, and a tab per ticket for a batch. |
| “Diagram” (`diagram`) | A Mermaid diagram, with zoom and panning. |
| “Comparison” (`compare`) | From 2 to 8 options side by side, their images (light and dark), their pros and cons, the recommended one marked. |
| “HTML page” (`html`) | The agent’s HTML, or an `.html` file of its working folder with the files it names. |
| “Own page” (`custom`) | The page you set for the project, filled with the agent’s data (below). |

The templates follow the app’s language and theme, and the project’s color. The agent only gives data: Escouade checks it and writes it as text, never running what it holds.

## Where it opens

- **Beside the agent’s conversation, if you are looking at it**, and the conversation stays the active pane: you keep reading it or writing in it. If you are elsewhere, the page’s row waits in “Browsers” with an amber dot, and opens when you come back to the conversation or click the row. Your screen is never taken (see [agents and browsers](agent-browsers#your-screen-is-never-taken)).
- **Its row** `page:p-12` carries a page icon in the project’s color, its title and its version (“v2”). Its gestures are a browser’s: click, `Ctrl`+click, drag into a pane, and its menu (“Open beside”, “Open below”, “Close the page (it stays kept)”).
- **Its card in the conversation** says “Shown” or “Updated”, where the page is (“beside”, “waiting in Browsers”, “closed”) and offers “Show”, or “Reopen” once closed.

## Its versions

The agent updates its page instead of opening another one: the new version reloads in its place. The page’s bar carries **“v2 ▾”**, the menu of the versions kept (the last 10). On an older version, a band says “You’re looking at v1; the latest is v3”, with “Back to v3”.

The bar also has the widths (full, tablet, mobile), like a browser, but no address, back or reload: the page is a copy, not a site.

## Driving a page

On its page, the agent only shows: it scrolls, highlights a part (a criterion, a file, a node of the diagram), changes the width, opens a tab of the page (a ticket’s, in a batch) or shows another version. It clicks, types and chooses nothing.

- **The band** above the page says “esc-412 is driving” and the caption of its last gesture, with “Take over”.
- **The markers**: each gesture is a numbered row in the conversation (“③ Highlights “Criterion 3””) and the same number on a rail to the left of the page. Clicking a marker, or the row, brings the page back to where the agent had put it.
- **Your first gesture in the page** or “Take over” stops it at once; “Hand it back” lets it go on. “Continue” shows when it waits for you to have looked.

The hand, the band and the limits are the same as for a tab: see [what you see while it drives](agent-browsers#what-you-see-while-it-drives).

## Your choices

A page can carry a question, left at the same time in your [“Your turn”](inbox) box: the review has its “Your verdict” section with its options, each option of a comparison its “Choose” button.

- **A click in the page proposes, nothing more.** Under the page, a band of Escouade says what you chose with the question’s words, never the page’s (“Chosen in the page: A · Merge into main”).
- **“Send to esc-412”**, in this band, sends your answer; “Cancel” drops it. Nothing goes without this gesture: a page can neither answer alone nor change your choice under your click.
- **A go-ahead** (merge, publish, sign) is never given from the page: the band says so and carries its own “Yes” and “No”.
- Your answer reaches the agent like any answer of the box, and the page shows it, its options frozen. In “Your turn”, the request shows a thumbnail of the page and “Open the page”.

## A page apart, without network

The band’s notice says so: “Written by esc-412 · offline, no access to Escouade or your files”.

- Escouade **copies** the page at the time of the call and serves it in its own webview, at an address of its own. It loads nothing from the web, calls no Escouade command, keeps nothing from one opening to the next, and opens neither window nor download.
- The files it names come from the agent’s working folder only: a path that leaves it, a hidden file (`.env`, `.git`) or a missing one is left out.
- A link to the web is not followed: a band says “The page wants to open…”, with “Open in the system browser” and “Ignore”.
- What it says comes from the agent: read it again before copying a command. Expect no secret in it, and the agent only puts in it what it has read again.

A page belongs to the agent that showed it: another agent does not know it, and nothing you do in it reaches the agent, except the answer you send. It stays kept as long as its agent exists, then goes after the delay of “Keep the files of closed requests”.

## Your own page

Settings › Projects › *your project* › Launch, group “Agents’ pages”: **“Page of your own”** is an `.html` document of the project your agents fill with their data (the “Own page” template). It runs in the same sandbox as the other pages and reads its data in `window.escouade`. A repository can propose one through `pages.custom` of [.escouade.yml](escouade-yml), applied only after your approval; your machine’s comes first.

## The switch

Settings › Agents, under “Agents can drive Escouade”, **“Agents’ pages”** (on by default). Off, the agents no longer have `show_page`, `get_page` or `close_page`, the pages’ rows close and their cards say “Agents’ pages are off (Settings › Agents).” Nothing is erased: switched back on, the pages are there. The tools are described in [the MCP server](mcp#agents-pages).

> [!TIP]
> Claude Code’s artifacts, published on claude.ai, do not change: a Claude Code agent keeps them for what gets published, and can open the last one for you in a tab (see [agents and browsers](agent-browsers#opening-a-tab-beside)).
