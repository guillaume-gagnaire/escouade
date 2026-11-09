# Using isola

isola runs each worktree’s services on ports and `*.localhost` addresses of its own. Two tickets tested at the same time do not step on each other: each has its front end, its API, its database.

## Testing a ticket

“▶ Test” is on the “To review” card, in the header of the agent’s conversation and in the criteria summary. With isola:

1. The “Test” window shows the worktree’s `.isola.toml` under “isola configuration (.isola.toml)”, with the warning: isola runs the commands in this file in your shell, outside the harness’s permission mode, and the agent may have written or edited it. It also shows the address that will open.
2. Nothing runs before **Run**. Approved once, the file is not shown again while it does not change; changed, it is.
3. Escouade runs `isola up` in a terminal of the project. Its input is not a terminal: isola never asks you for a password there (to trust a certificate, it says how to do it).
4. Once `isola up` is done, Escouade asks the plugin for the worktree’s services (`isola ls --json`, those of the worktree’s branch). Each service has its line in the window.
5. Escouade waits for each service to answer, 3 minutes at most, then opens the address the agent gave, or else the first service’s that has one.

**Stop all** runs `isola down`.

## The services, their ports and their addresses

For each service of the worktree, isola gives:

- its **address**: its proxy’s (`http://ticket-dem-1.demo.localhost:3000`), else its own port’s (`http://localhost:8117`);
- its **own port**: that is what Escouade probes to know the service is ready, because the proxy answers (with an error page) before the service does;
- its **status**, as isola says it (`running`, `stopped`…).

A service without an address (a background process) counts as ready as soon as it runs. A service that does not run fails the test: “{name} isn’t running”.

The ports and the addresses are those isola picks from the `.isola.toml`: Escouade does not set them, and reserves no ports for an isola worktree.

## When the services stop

Escouade runs `isola down` for the worktree:

- when you click **Stop all**, or when the agent’s test launches stop (ticket validated, agent archived);
- when the agent is deleted and its worktree kept;
- when the project is closed.

These stops run in the background: a failure only goes to the app’s log.

## When the worktree goes

When a worktree is removed (agent deleted with its worktree, the board’s clean-up, “Clean up”), Escouade first runs the project’s “Before a worktree is removed” commands, then `isola destroy`, which tears the services down with what they keep (their databases). A failure is said once the agent is deleted: “Agent deleted, but …”.

## Its limits

- Each call to isola (`ls`, `down`, `destroy`) has 60 seconds. Past that, Escouade stops it.
- isola only sees the worktrees that have an `.isola.toml` at their root.
- Only a plugin shipped with Escouade can be a launch environment: `isola up` is a command of the app’s terminal, which a plugin from elsewhere is not given.
- It does not read the services’ logs: they stay isola’s.
