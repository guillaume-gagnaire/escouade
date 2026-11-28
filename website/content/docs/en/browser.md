# Browsers

Escouade has browsers, as it has terminals: the “Browsers” section of the sidebar, under “Terminals”, has one row per tab. You open the app you’re building there, next to the agents writing it, with its developer tools and three widths, without switching windows.

A browser opens on its own, from its “+”, or by itself when an agent’s test or a launch command is ready.

![A browser full screen: its bar (address, widths, Devtools) and the page of the app being built; on the left, the “Browsers” section and its three rows](/images/en/browser.jpg)

## The “Browsers” section

Like the terminals’ section:

- **One row per browser**, in the order they opened, with their count at the top. The row shows the page’s title (“Tableau — demo”), else its host; a launch’s row shows the agent and its process (“pagination-users · web”), with its worktree’s mark.
- **Its dot** shows the page’s state: amber while it loads, green once loaded, red when it isn’t answering (a load that doesn’t finish in 20 s).
- **A click** shows it in the main area, full screen, like a terminal; `Ctrl`+click (`⌘`+click on macOS) opens it next to what’s shown, half screen; you can also drag it onto the work area (see [the split screen](split-screen)).
- **Its menu** (right click): “Open beside”, “Open below”, “Reload”, “Copy the address”, “Open in the system browser”, “Close the browser”.
- **Its cross** closes the browser.
- **“+”** asks for an address, in the section itself: type `localhost:3000` (without `http://`, it’s `http://`) and `Enter`. `Esc` gives up.

In a short window, the section folds to its title, like “Terminals”; a click on the title unfolds it.

A browser lives while Escouade runs, like a terminal: neither comes back after a restart. The width you chose is kept for the page.

## What opens a browser

- **You**, from the section’s “+”.
- **An agent’s test**: as soon as the processes of “▶ Test” answer (see [Review and approve](testing)), the address that shows the feature opens in the agent’s row. If you were looking at the agent or at its “Test” window, Escouade shows it to you; otherwise the row waits for you. There is one row per agent: testing again reloads the same page. It closes with the agent and when its ticket moves to “Done”.
- **A launch command** with an address to open (see [Launch the project](launch)): its page opens in its row, one per command, reloaded at each launch rather than doubled.

Settings › Projects › Launch, “Open the launches’ pages”, chooses between Escouade’s browsers (default) and the system browser. The former “In a separate window” choice now opens the browser’s row: the “Pages” window is gone.

In the header of an agent’s conversation, “Page” shows its launch’s row; dragged onto the work area, it puts it next to the conversation.

## A browser’s bar

| Button | What it does |
|---|---|
| Back, Forward, Reload | As in a browser. |
| The address | Editable: type a path or an address and `Enter`. Without `http://`, the address keeps the page’s. The dot before it shows the page’s state. |
| `pagination-users · web` | For a launch’s page: the agent and the process serving it. |
| Full width, Tablet, Mobile | The page’s width (see below), next to the address. |
| Devtools | The developer tools of **this page**: console, network, elements. |
| Open in the system browser | The same address, in your browser. |

“…” opens the page beside or below what’s shown (next to its agent’s conversation when it’s a launch’s page shown alone) and copies its address. When the view is narrow, Forward and the system browser move there too, then the widths when it’s narrower still (a narrow pane).

A launch’s page also says when the launch is restarting (“The launch is starting…”) or has stopped (“The launch stopped”, with “Launch again” and “View logs”). When a menu, a list, the palette or an Escouade window passes over the page, it hides while they’re open, then comes back.

## Developer tools

The “Devtools” button, or `F12` and `Ctrl+Shift+I` (`⌥⌘I` on macOS) while the focus is in Escouade, open the page’s developer tools. From the page itself, the right-click “Inspect” does it everywhere, and `F12` on Windows. They only open on your gesture: the page can’t open them by itself. They are the system webview’s own, and each docks where it can:

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
|---|---|---|---|
| Where they open | In a window of their own | In a window of their own | In the page, at the bottom |
| At the bottom or right of the page | No: WebView2 doesn’t dock them in the app (it would take a debugging port open to every program on the machine) | No: docked in the page, WebKit’s inspector changes its width itself, and the Tablet and Mobile widths no longer hold | Yes: at the bottom by default, to the right or apart from the inspector’s buttons |

The “Devtools” button’s tooltip says so for your system. Escouade’s own developer tools stay closed.

## Full width, tablet, mobile

Three buttons, next to the address, change the page’s width:

- **Full width**: the page takes its whole area.
- **Tablet**: 768 px, centred on a neutral background, the area’s height.
- **Mobile**: 390 px, likewise.

The width shows next to them (“768 px”). If the area is narrower, the page shrinks to the area and says so (“Narrowed to 692 px”). `Ctrl+Shift+M` (`⇧⌘M` on macOS) moves to the next width, from the page or from Escouade. The choice is kept for the page: launching again keeps its width.

Only the width changes: it’s what triggers the page’s media queries. Nothing else the site sees moves:

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
|---|---|---|---|
| Mobile user agent | Possible | Possible | Possible |
| `devicePixelRatio` | Possible, through the devtools protocol | No | No |
| Touch screen | Possible, through the devtools protocol | No | No |

That isn’t done yet: to emulate a whole device, use the developer tools’ device mode where they have one (Edge’s on Windows).

## A page apart

The page is the app you’re building, or a site you typed: Escouade doesn’t trust it, without exception.

- It runs in a webview **apart**, with access to **no** Escouade command: a call to `__TAURI__` or `invoke` from the page is refused.
- Its cookies and storage (localStorage, IndexedDB…) are the **project’s**, never the Escouade window’s: a `pages/<project>` folder in Escouade’s data on Windows and Linux, a separate store on macOS 14 and later (a private session before).
- Only an `http://` or `https://` address opens in a browser, never an Escouade address, a file or another scheme.
- It stays on its starting address’s origin: a link to another origin (another port, `127.0.0.1` instead of `localhost`, another site) opens in your browser, and Escouade says so (“github.com opened in your browser: only localhost:4101 shows here.”); Escouade’s addresses, files and other schemes open nowhere.

## At a distance

A page only shows on the screen of the machine it runs on: browsers belong to the window. A window connected to a [server](connect-server) has its own, on your machine (a server test’s address is relayed there). The phone and the remote API don’t see them.
