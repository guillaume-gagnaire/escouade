# Agents and browsers

An agent opens a real tab of “Browsers” beside its conversation and drives it while you watch: it scrolls the app it is building, highlights what it is talking about, clicks, changes the width. What it does without asking, what waits for your permission, what stays refused, and how to take over.

## What it is for

An agent that has finished a feature shows it to you instead of describing it. It opens the page of its launch in a pane beside its conversation, scrolls to what it changed, outlines it, opens the tab of the page that matters, switches to the mobile width, and tells you what it does at each gesture. You watch, take over with one gesture, and answer it in its conversation.

It can also read what you see: the HTML or the text of a page, a capture, the console and the failed requests, to understand an error without asking you to copy it.

This is the second of the two modes. The first, a page an agent writes and shows you (a review, a diagram, options to compare), is in [agents’ pages](agent-pages). Both go through the same rows of “Browsers”, the same driving and the same band.

## The two modes

| | An agent’s page | A real tab |
|---|---|---|
| What it is | A page the agent writes: a frozen copy, without network | An ordinary row of “Browsers”: the app being built, a site, an artifact |
| Who opens it | `show_page` | `open_browser`, or you, lending it |
| What it does there | Show, open a tab of the page, switch versions | Show, and act: click, type, choose, navigate |
| What comes back to it | Your choice, on your gesture alone | What it reads, on the launch or with your permission |

## Opening a tab beside

An agent opens a tab of its project with `open_browser`:

- **the page of its launch**: its test’s if it is open, otherwise the address of one of the project’s launch commands, at its ports (see [Launching the project](launch));
- **an address**: its launch, or another address, which it asks you first (below);
- **a Claude artifact it just published** (Claude Code only): Escouade finds its address in its conversation, and the agent reloads it after each new version.

The tab opens beside the agent’s conversation, or below it, or alone, as it asks. It keeps the project’s storage, never the Escouade window’s. An agent drives at most 3 browsers at a time and opens at most 10 an hour.

## Your screen is never taken

- **Beside only if you are looking at its conversation.** If you are elsewhere (the Kanban, another agent, another project), nothing opens: the row waits in “Browsers”, its name in amber, with “esc-412 wants to show you the page” as its tooltip. It opens where the agent wanted it when you come back to its conversation, or when you click the row.
- **The conversation stays the active pane**: you keep reading it or writing in it while the page opens beside it.
- **One pane per agent**: another browser, or another page it drives, takes the place of the previous one instead of opening one more.
- **Nothing reopened after you close it.** If you close its pane while you look at its conversation (the pane’s cross, “Close the pane”, its row’s cross, or going back to a single pane), nothing more from this agent opens on your screen, browser or page, until you have left its conversation and come back to it. The agent knows.

## What you see while it drives

- **Its row** in “Browsers” carries the agent’s initials in the project’s color; its tooltip says “Driven by esc-412”.
- **A band** above the page, which Escouade draws outside the page (the page cannot imitate it): “esc-412 is driving”, the number and caption of its last gesture (“② Highlights “Pro plan””), “Take over”.
- **A ghost cursor** shows where it acts, and what it highlights lights up in the project’s color.
- **One numbered row per gesture** in its conversation; clicking one shows the browser.
- **“Continue”**: when the agent wants you to have looked before going further, the band shows “Continue” and it waits for your click, 5 minutes at most.

Each gesture waits for the page to settle (1.5 s at most) before the next. At most 4 gestures a second and 600 an hour, per browser.

## Taking over

- **Your first gesture in the page** (a click, the wheel, a key) or **“Take over”** stops the agent at once: its next gesture is refused, without the page moving, and it knows.
- The band then says “You have control”; **“Hand it back”** lets it go on. What you did meanwhile stays.
- **“Stop letting it drive”**, in the row’s menu, stops its driving until the browser closes.

## The project’s launch and the rest

What an agent does depends on the page’s origin: your launch’s, or another.

**The project’s launch**: the addresses of its launch commands, and this machine (`localhost`) at the ports they declare, with its agents’ tests and their reserved ports. This is the app you are building: the agent reads, captures, reads the console and acts there without asking.

**Outside the launch** (another site, an artifact, an identity provider the tab went through), three levels, per origin:

| Level | What it does there | When |
|---|---|---|
| Show | Open, place, change the width, scroll, reload, go back, highlight | Without asking. The agent only learns whether the element was found, and the page’s origin: never its title, its path or its text |
| Read and act | Read the HTML or the text, capture, read the console, click, type, choose, navigate within the origin | With your permission, once per origin and per browser |
| Forbidden | Password, file, download, cookies, storage, Escouade’s secrets | Never, even with your permission |

A page that goes by itself to another origin (a redirect, a sign-in) gives the agent nothing more: what it reads is judged on the origin the tab is at when it reads.

## Permissions

Everything that waits for your permission asks you in a band, and your answer reaches the agent as a message in its conversation. Asking again before your answer adds nothing, and a refusal holds: it cannot ask again.

- **Opening another address**: a band above its composer, “esc-412 wants to open docs.stripe.com (outside the project’s launch, in an Escouade browser)”, with “Refuse” and “Allow once”. Nothing opens before your answer; the allowed origin only holds for this browser, until it closes.
- **Reading and acting outside the launch**: a band under the browser’s bar, “esc-412 wants to read app.example.com (with your session, in this browser)”, with “Refuse” and “Allow”. The permission holds for this agent, this origin and this browser, until it closes.
- **Typing a password on the launch**: “esc-412 wants to fill in a password (in this browser, until it closes; the value is never read back)”, with “Refuse” and “Allow”.
- **“Revoke the permissions”**, in the row’s menu, drops this browser’s permissions, requests and refusals. They also drop when it closes.

## What is always refused

Even with your permission, an agent cannot:

- read a page’s cookies, storage, headers or response bodies; a password’s value is never given back to it (`«masqué»`), nor that of a page’s hidden fields and tokens;
- type a password outside the launch: you sign in there yourself;
- upload a file, download, or fill in a form that is sent to another origin;
- type a secret of Escouade (the MCP server’s tokens, your accounts’, the proxy’s password), or what looks like a key;
- open an Escouade address, a file (`file:`), `data:`, `javascript:` or another scheme, nor this machine at a port your project does not declare: it may be a service of your machine;
- drive a browser you cannot see, another agent’s, or a browser you opened without lending it.

What it reads from a page reaches it marked as third-party data: it follows no instruction it finds there.

## Signing in to a site in a tab

An agent never signs in for you. If the page it must show you needs a session, sign in yourself in the tab: its sign-in page, its sign-in window (Google’s or an OAuth’s, which opens in Escouade) and the link received by email, which you paste into the bar, stay in the tab (see [signing in to a site](browser#signing-in-to-a-site)). The session is the project’s; the agent does not read it, and the sign-in windows are never read, captured or driven.

## Lending one of your browsers

A browser you opened yourself is not the agent’s. To let it drive, open its row’s menu and choose **“Let drive…”**: the list of the project’s live agents, the one driving checked, and “Nobody” to take it back. The agent learns it by a message; nothing moves on screen. The same rules apply then: the launch without asking, the rest with your permission. When it is done, it stops driving and the row stays yours.

## The switch

Settings › Agents, under “Agents can drive Escouade”, **“Agents and browsers”** (on by default) gives the agents the tools of the browsers. Off, they no longer list, read, open or drive any tab, and their instructions no longer mention them; [agents’ pages](agent-pages) have their own switch. The tools (`list_browsers`, `read_browser`, `capture_browser`, `browser_console`, `open_browser`, `drive_browser`, `close_browser`) are detailed in [the MCP server](mcp#browsers).

The browsers of a private project are known to its agents alone. Claude started outside Escouade lists the browsers of a project that is not private, without reading or driving them.

> [!NOTE]
> On Windows, the agent clicks and types with real input events, which the page hears as yours. On macOS and Linux, its clicks and typing are simulated in the page, and the console comes from a relay in the page, which the page could silence.
