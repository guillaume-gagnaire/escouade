# Quotas and cost

The status bar, at the bottom of the window, shows how many agents are working, where the quotas of your Claude subscription stand and what you have spent today. This page explains what each figure means, where it comes from and what Escouade does when a quota nears its limit.

## The status bar

The bar sits at the bottom of the window. From left to right:

| Item | What it shows |
|---|---|
| “3 active” | The agents that are working, across all projects. Archived agents don’t count. |
| “1 waiting” | The agents that are waiting for your answer. A click, or `Ctrl+J`, puts the focus on the card of the next agent that is waiting or needs a look. As soon as there is one, the counter turns amber and its dot pulses. |
| “✓ 2 done” | The agents whose task is done. |
| “4 Claude · 1.2 GB · 6% CPU” | The running Claude processes, with the memory and processor they take. Missing when none runs. The tooltip lists each agent, the one using the most memory first. |
| Quotas | The two quota windows of the current account: see the next section. |
| “Today $4.12” | What the agents have cost since midnight: see the “Cost” section. |
| “⎇ main ↑1 ↓3” | The branch of the project on screen, when it is a git repository. A click opens the branch picker ([branches](branches)). |
| On the right | The state of an [update](updates), “♪ On” or “♪ Off” for the sound of [notifications](notifications), and ⚙ for the settings (`Ctrl+,`). |

> [!NOTE]
> On macOS, `Ctrl` becomes `⌘` in shortcuts (except `Ctrl+Tab`).

## Quotas

### Reading the bars

Escouade follows the two quota windows of your Claude subscription: the 5-hour window and the weekly window. Each one is written compactly, a label, a bar and a countdown:

`5h … reset 3h01 | W … reset 4d 12h`

- “5h” is the 5-hour window, “W” the weekly window (7 days).
- The bar fills with the share of the quota used. It turns amber above 80%.
- “reset 3h01” is the time left before the window resets: 3 hours and 1 minute. From one day on, the countdown reads “4d 12h”. The word “reset” is the same in both languages.
- The percentage isn’t written: it shows when you hover the label, the bar or the countdown, with the date of the reset. For example: “42% · resets on 10/10 at 6:00 PM”.
- With a single account, `Tab` focuses each bar, which screen readers announce as “5-hour quota” or “7-day quota” with the same value.
- As long as Escouade has read nothing, the bar is empty, a “—” replaces the countdown and the tooltip says “Quota unavailable”.

### A model’s own quota

On some plans, Claude counts one model apart: Fable’s week, for instance, has its own cap and its own reset. Escouade gives it one more bar, beside the others and under the name Anthropic gives it: “Fable 7d”. It reads like the others (tooltip, screen readers: “Fable 7-day quota”), in the status bar, the quota panel and Settings › Accounts.

That quota holds back that model’s agents and tickets alone. When Fable’s week passes the threshold, a Fable ticket goes to another account or waits, and the Opus or Sonnet tickets go on on the same account. The autopilot is not paused for it. A Fable agent stopped by its limit waits for the reset without holding the other models back. Escouade knows the model by its name (“fable”, “claude-fable-5-1”…), whichever name it goes by.

The bars follow what the server tells: a plan without a 5-hour window (some Max plans) has no “5h” bar, and a window that leaves the plan leaves the bar at the next reading.

### With several accounts

As soon as there is more than one account in [the list of accounts](accounts), the bar writes the current account’s name in front of the two quotas: it is the account a new agent would start on right now, leaving out the accounts reserved to projects.

The group then becomes a button, “Quota by account (current account: Pro)”. A click (or `Enter` with the keyboard) opens the “Accounts quota” panel above the bar. `Esc` or a click outside closes it. It shows each account, the current one first, then the others in the order of the settings:

- its name, with the “current” tag for the current account and “past the pause threshold” when one of its windows has passed “Pause above quota”;
- what its turns cost today, for example “Today: $1.20” (with “≈” while a turn is running);
- its two windows, this time with the percentage written next to the bar;
- dimmed, the reason when the account isn’t fully read: “Inactive”, “Not signed in”, “Sign-in expired: run Claude Code again for this account.”.

### Where the quotas come from

Escouade reads each account’s quotas again every minute, a first time two seconds after startup. The figures come from two sources:

- **A Claude process of the account**, when an agent of the account is running: Escouade queries it, and Claude Code also reports the state of the windows by itself during a turn. It is the most direct source.
- **Anthropic’s usage service**, for an active account no agent is running on. Escouade queries it with the sign-in Claude Code keeps for the account, which it reads without changing, and at most once every 5 minutes per account. The request goes through the proxy of the [network settings](settings-network) if there is one.

When an account has just been signed in, its quota is read right away.

### When a quota isn’t read

| What you see | What it means |
|---|---|
| “Quota unavailable”, empty bar, “—” | Escouade hasn’t read anything for this account yet, or the reading failed. |
| “Not signed in” | Escouade finds no claude.ai sign-in for this account. Sign it in from [the accounts](accounts). For Main: “Not signed in with a claude.ai account (API key?)”. |
| “Sign-in expired: run Claude Code again for this account.” | The sign-in is out of date. Claude Code gets a new one when it runs; “Sign in…” in the accounts settings renews it. |
| “Inactive” | The account is switched off. Its quota is only read through an agent running on it. |
| Figures that no longer move | A reading that fails (network, server, proxy) leaves the last values in place. |

For an account that is “Not signed in” or whose sign-in has expired, the last values read stay shown until the end of their window.

> [!NOTE]
> On macOS, Claude Code keeps the sign-in in the keychain, and macOS may ask you to allow Escouade to read it. If you refuse, Escouade doesn’t ask again for 10 minutes.

## Cost

Amounts are in US dollars.

### Today

“Today” is the sum of the turns finished since midnight, your computer’s time, across all accounts, including those you have removed since. It updates at the end of each turn. With several accounts, the quota panel gives the same total account by account.

### Live during a turn

While Claude works, tokens and cost rise live. Claude Code only gives a turn’s exact cost when it ends; until then, Escouade estimates it from the models’ public prices (input, output, cache read and cache write). An estimate is written with “≈” in front of the amount, and the tooltip explains it: “Estimate (public prices) while Claude works; exact cost at the end of the turn”.

When the turn ends, Claude Code’s exact figure replaces the estimate. For a model whose price Escouade doesn’t know, tokens rise but the cost only appears at the end of the turn.

### Where to see an agent’s figures

| Where | What is shown |
|---|---|
| The agent’s card, in the sidebar | Its tokens (“tok”), its cost and the number of modified files. |
| The conversation header | “Tokens” and “Cost”, with “Context” and “Duration”. |
| End of a turn | The “Task done” card (duration, tokens, cost, edited files), or a duration · tokens · cost line between two turns. |
| A Kanban ticket’s card | Its agent’s cost. |
| [Overview](overview) | Each agent’s cost. |

The [statistics](stats) add all this up by day, week or month.

## When a quota nears its limit

Escouade reacts in three ways. The first two depend on the “Pause above quota” threshold, which you set in Settings › Kanban, “Autopilot” group: 80, 90, 95 or 100% of a window, 100% by default, the same for all projects ([the Kanban settings](settings-board)). An account is at the threshold when one of its windows reaches it and its reset is still to come.

1. **New agents change account.** When the current account passes the threshold and another one is available, new agents and tickets start on the other. A notification says so: “The Main account has reached 91%: new agents now go to Pro.” ([Several Claude accounts](accounts)).
2. **The autopilot pauses.** When every account a ticket can use is at the threshold, no ticket starts. The Kanban then writes, under its header, a line such as “Autopilot paused: 5-hour quota at 92% (resumes at 3:00 PM)”, with the names of the accounts concerned when there are several, and the “Resume now” button to override it. The pause lifts by itself at the reset. When a ticket’s agent hits the usage limit with no resume planned (auto-resume off, or the reset time unknown), no ticket starts either for 30 minutes on its account. See [the autopilot](autopilot).
3. **An agent reaches the limit mid-turn.** It stops and resumes by itself after the reset, or on another account if you ask for it. See [Resume an agent stopped by the usage limit](accounts#resume-an-agent-stopped-by-the-usage-limit).
