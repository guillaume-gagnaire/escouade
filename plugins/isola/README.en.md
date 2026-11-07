# isola

Each worktree gets services of its own, on ports and `*.localhost` addresses of its own, with [isola](https://github.com/cyucelen/isola). When the project has an `.isola.toml`, “▶ Test” runs `isola up`, waits for each service to answer and opens the address. Escouade stops the services with `isola down` and tears them down with `isola destroy` when the worktree goes.

**Who it is for**: a project whose tickets are tested by starting several services (a front end, an API, a database), and that does not want a recipe on reserved ports for each agent.

**What it needs**:

- the `isola` program on your PATH (on Windows, an `isola.exe`);
- an `.isola.toml` file committed at the root of the project.

No account, no secret, no network.

**What it can do**: run, in an agent’s worktree and nowhere else, three exact commands: `isola ls --json`, `isola down` and `isola destroy`. Escouade refuses any other. `isola up` is run by Escouade itself, in a terminal, once you have read the `.isola.toml`.

It ships with Escouade and is on in every project except private ones, where you turn it on yourself.

The rest is in the documentation tabs:

1. Install: what the machine and the project need.
2. Use: a worktree’s services, their ports and their addresses.
3. Permissions: what it runs, what it reads, what it never does.
4. Troubleshooting: its messages and what to do.
5. Develop: its folder, its tests.
