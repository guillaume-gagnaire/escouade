# Statistics

Statistics show what your agents have consumed: tokens, cost and prompts by day, week or month, broken down by project, model, agent, ticket and account.

## Open the statistics

Click “Stats”, at the bottom of the projects rail. The page replaces the project on screen; a second click on “Stats”, or a click on a project’s tile, takes you back to it. It has no shortcut.

![The Statistics page: tokens, total cost, average cost per prompt and prompts over 14 days, a chart of tokens per day stacked as input, cache and output, then the breakdowns by project and by model.](/images/en/stats.jpg)

The header recalls what is counted: “Statistics”, then for example “Agents started from the app · 4 projects, 12 agents”. Only agents started from Escouade count: what you do in Claude Code outside the app isn’t there. The page refreshes by itself when a turn ends.

## Choose the period

The selector at the top right offers “Day”, “Week” and “Month”. Your choice is kept from one launch to the next.

| Choice | Steps of the chart | Compared with |
|---|---|---|
| Day | The last 14 days, today included (“Sep 27”) | The 14 days before |
| Week | The last 12 weeks, Monday to Sunday (“W39”, the ISO week number) | The 12 weeks before |
| Month | The last 12 months (“Oct”, “Sep”…) | The 12 months before |

The last step is in progress: today, the current week or the current month.

## The indicators

Four cards sum up the period:

| Card | Value | Line below |
|---|---|---|
| “Tokens” | The period’s tokens: input, cache and output added up. | “Last 14 days · +12%”: the change from the previous period. With nothing to compare to, only the span is written. |
| “Total cost” | The period’s cost. | The all-time cost since the first recorded turn, for example “$412.80 since August 14, 2025” (or “$412.80 in total”). |
| “Average cost / prompt” | The period’s cost divided by its number of prompts. | “≈ 206.6k tokens / prompt”. With no prompt, “—” and “no prompts in this period”. |
| “Prompts” | The number of prompts in the period. | “10 per day on average” (per week, per month). |

A prompt is a message delivered to an agent.

## The chart

The “Tokens per day” chart (“per week”, “per month”) has one bar per step. Each bar stacks three series, recalled in the legend:

| Series | What it counts |
|---|---|
| “Input” | The tokens sent to the model, outside the cache. |
| “Cache” | The tokens read from the prompt cache or written to it. |
| “Output” | The tokens the model produced. |

Hover a bar to read the step’s details: the three series, the total, the cost and the number of prompts. The “Table” button replaces the chart with a table of the same figures (“Period”, “Input”, “Cache”, “Output”, “Total”, “Cost”, “Prompts”); “Chart” brings it back.

## The breakdowns

Under the chart, the period is cut up in several ways.

### By project and by model

Two lists side by side, ranked by tokens. Each line has a name (the project’s color in front of its name), a bar proportional to the heaviest line, the tokens and the cost. Until its history is reopened, a project closed since shows as “Closed project”.

A closed project’s figures are kept with the others. Add the project again in its folder (see [Closed projects](settings-projects#closed-projects)): it gets the same ID back, so its name, its agents, and all its figures. With “Start from a blank project”, the new project starts from zero and the old one stays “Closed project”.

### By account

With several accounts, the “By account” table puts the accounts side by side, from the most to the least expensive, with the columns “Account”, “Tokens”, “Cost”, “Turns” and “Agents”. A turn is one reply from Claude; “Agents” counts the distinct agents that ran on the account during the period. An account removed since stays listed as “Deleted account (pro)” (its id in parentheses).

### By harness

With accounts or turns of several harnesses, the “By harness” table puts the harnesses side by side, with the columns “Harness”, “Tokens”, “Cost”, “Turns” and “Agents”. A turn’s harness is the one that ran it, even if the agent has changed harness since. Each harness’s cost is written as it gives it: exact, estimated, or missing (see “Estimated cost” below).

### By agent

The “By agent” table lists the period’s agents, from the most to the least expensive, with their project, their tokens and their cost. It also counts archived agents; a deleted agent keeps its row, as “Deleted agent”.

### By ticket

The “By ticket” table adds up what each ticket’s agents cost: “Ticket”, “Title”, “Loops” and “Cost over the period”, from the most to the least expensive. With no ticket, “No tickets in this period.”

The agent and ticket lists show 20 rows. “Show all” unfolds the rest, “Show less” folds it back, and “20 of 57” tells you where you are.

### Code index

When a project has turned the [code index](code-index) on, the “Code index” table tells, per project and over the period, how many answers the index gave and what they weigh in tokens. Its last column, “File reading avoided (high estimate)”, counts the files the answers come from as if the agent had read them whole: it is not a measured saving, and the tooltip says so. It holds for every account and every harness, whatever the filter.

## Filter by harness and by account

With several harnesses (accounts of several, or turns recorded on several), a “Harness” menu shows at the top: “All harnesses”, then each one. Choosing a harness narrows the indicators, the chart, the lists and the prompts to it; the “By harness” table stays complete, with its row highlighted. The “Account” menu combines with it: it only offers the accounts of that harness, and the “By account” table only lists them.

With several accounts (or turns recorded on several), an “Account” menu appears to the left of the period selector. It offers “All accounts”, each account, and the removed accounts the statistics still know about.

Choosing an account narrows the indicators, the chart and the by-project, by-model, by-agent and by-ticket lists to it, as well as the comparison with the previous period. The “By account” table stays complete, with the chosen account’s row highlighted. The choice isn’t kept: the page always opens on “All accounts”.

For the accounts themselves, see [Several Claude accounts](accounts).

## Estimated cost

Each harness says how it gives the cost of a turn. Claude Code gives it exactly, and so does Copilot CLI: its AI credits (a credit, $0.01), else the per-token prices of its models. Codex only gives its tokens: the cost of its turns is estimated from the public price of OpenAI’s models, in the conversation (“≈ $0.02 estimated”) as in the statistics. With a ChatGPT subscription, it’s a theoretical cost, like Claude’s with a Max subscription. A harness that only gives its tokens has a cost Escouade works out from the public price of its models: it is marked “≈” and “estimated”, and a tooltip says so. A harness that gives no cost has “—” instead. As soon as an estimated harness counts in what is shown, the totals are marked “estimated” too.

## Where the figures come from

The figures are in `stats.db`, a SQLite database in Escouade’s data folder (`~/.escouade/`, see [local data](data)).

- At the end of each turn, Escouade writes one row per model used: the date, the agent, the project, the model, the input, cache and output tokens, the cost and the account. The cost is Claude Code’s exact figure: the status bar’s “≈” estimate doesn’t go in (see [Quotas and cost](usage)).
- Each message delivered to an agent is noted as a prompt, with its date, the agent, the project and the account.
- The conversations’ content isn’t in there.
- A turn’s account is the one the agent had at that moment: an agent resumed on another account counts on that one afterwards.
- Archiving or deleting an agent, or closing a project, doesn’t erase its rows: they stay in the totals.

When the statistics can’t be read, the page writes “Statistics unavailable:” followed by the error.
