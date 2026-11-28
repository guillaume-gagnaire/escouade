# Connect to a server

Escouade can run on your server, day and night, and this computer drive it through an SSH tunnel. This page explains how to connect the first time, what the app says about the machine you are on, what happens when the connection drops, and how to come back to this computer or remove a server.

## Before you start

- On the server, `escouade-server` is installed and running. It listens only on a Unix socket that is its user’s alone (the service of `install.sh`) or on its `127.0.0.1` (port `46480`): nothing is opened to the Internet.
- This computer already connects to the server with `ssh`, with a key loaded in your SSH agent. Escouade runs this computer’s `ssh` (on Windows, the system’s OpenSSH), with your config, your agent and your known hosts. It reads, copies and keeps no key, and never types a password.
- The simplest: a host in `~/.ssh/config`, for example:

```
Host acme-escouade
  HostName escouade.acme.internal
  User escouade
```

## The first connection

1. Open Settings › “Servers” and choose “Connect to a server”. Once a server is known, the entry is also in the menu of the “+” of the projects bar.
2. **The SSH host**: pick a host from `~/.ssh/config` (those with a pattern, like `Host *`, and those without a `HostName` are not listed), or type `user@address:port`. Change “Escouade’s port on the server” only if `escouade-server` runs with another `--port`.
3. **The connection**: Escouade reads over SSH where the server listens (`escouade-server status --json`), then opens `ssh -N -L <local port>:<its socket> <host>`, or `ssh -N -L <local port>:127.0.0.1:46480 <host>` on a port, and checks that Escouade answers on the server, and in which version. If the SSH server refuses the forward to the socket, the dialog says so and offers the fallback: the service on a port (see [Troubleshoot the tunnel](server#troubleshoot-the-tunnel)).
4. **The pairing**, once: on the server, run `escouade-server pair` and type the code it shows (8 characters, valid 10 minutes, for one device). Or click “run it from here over SSH”: Escouade runs it through the same connection and fills in the code. Name the space (“Work”) and this computer, as the server will see it.
5. The window moves to the server: its projects, agents, accounts and settings are the server’s.

This computer’s token is kept in this computer’s keychain, one per server; without a keychain (Linux), in a file only you can read. It is never written in the settings or in the log.

> [!NOTE]
> Escouade never accepts a host key for you. If the host is not in your `known_hosts` yet, connect once with `ssh <host>` in a terminal, check the key, then try again.

## Knowing which machine you are on

Once a server is known, the machine shows wherever a mistake would cost:

- **The title bar**: the space’s plate and name before the project (“Work acme-escouade / billing”), or “This computer”. At 1024 pixels wide, the SSH host goes away, the name stays.
- **The status bar**: “Work · ssh acme-escouade · 38 ms”, the round trip time.
- **The projects bar**: the open space above its projects, the others as piles under the “+”, a click away (see [the spaces](spaces)).
- **The foot of the sidebar**: “Work:” before the project’s folder, a server icon instead of a folder.
- **A terminal**: “Work · escouade@vps.example.net” in its header.
- **“New project”**: “on Work · escouade.acme.internal”; the folders are the server’s, not this computer’s (see below).
- **A file an agent sends**: “sent from Work · copied to this computer in …”.
- **Secrets**: “Kept in a file on Work” wherever a key is given.
- **Notifications**: the space’s name first, “Work · billing · reminders”.

## What happens on the server, and what stays on this computer

On a server, the projects, the agents and their files are there; you, your browser, your microphone and your folders stay here. Nothing the server sends opens on this computer without your click.

- **A new project**: “Clone a repository” (the server clones with its own git keys), or “A folder of the server”: its folders, whether they are repositories and on which branch, those that are projects already, and the room left on its disk.
- **What you send**: a project’s logo, an attached file, a pasted image go from this computer to the server, always as a copy.
- **What an agent sends you**: “Open” downloads it then into `Downloads/Escouade/Work` and opens the copy. A program or a script never opens: “Download to this computer” copies it and shows it in its folder. “See on the server” shows it in the Files panel.
- **A test server** (Launch, or a link to `localhost`): its port is relayed to this computer, and the page opens here, hot reload included.
- **Notifications, sound, flashing** are this computer’s; their choices (Settings › Notifications) are the server’s.
- **Voice** is this computer’s: its microphone, its transcription, its keys and its settings. Only the text goes to the server.
- **Signing an account in** (Codex, Claude Code, Gemini CLI…): the harness runs on the server; “Open the page on this computer” opens the sign-in page here. When it comes back to `localhost` (Codex, Gemini CLI), Escouade relays that return to the server; otherwise paste the code it shows into the terminal (Claude Code), or type the device code on the page (Copilot CLI). The account stays on the server.
- **Secrets** (keys of Cursor, of services, of plugins) are kept in a file on the server, readable by its own account alone: a server has no keychain.

## When the connection drops

Agents keep working on the server: closing the app, turning this computer off or losing the network does not stop them. A band says “Connection to Work lost at 14:32”, the next try, and that what you see is from 14:32. Escouade restarts the tunnel and the connection by itself (after 250 ms, then further apart, every 5 s at most); “Try again” does it now, “Go to This computer” brings you back here.

During the cut, sending is paused: your draft stays in the input, nothing is queued. Once back, the screen is read again whole, terminals resume where they were, and “Reconnected to Work · caught up on 12 events” shows.

## What the failures say

Each failure says what happened, that nothing changed on this computer or on the server, and what to do:

| What you see | What it means | What to do |
| --- | --- | --- |
| “acme-escouade isn’t answering” | `ssh` did not reach the host (VPN off, server down, firewall, wrong `HostName`) | Check the network, “Try again”; the `ssh` command can be copied to try it in a terminal |
| “Unknown host key” | The host is not in your `known_hosts` | Connect once with `ssh <host>` |
| “The key of … changed” | The host’s key is no longer the one this computer knew | Check it with the administrator before replacing it |
| “… refuses the SSH connection” | No key of your agent is accepted | Load your key (`ssh-add`), then try again |
| “No Escouade on the server” | `escouade-server` isn’t answering on its port | Install it, or start its service |
| “That code is wrong.” / “That code has expired” | The code typed is wrong, or expired | Type it again, or run `escouade-server pair` again |
| “… runs another version of Escouade” | This computer and the server don’t have the same version | Update the side that is behind |
| “This computer is no longer paired with Work” | This computer was revoked, with the date and the device that did it | “Pair again” with a new code, or remove the server |
| “Refused by this computer’s policy” | A managed computer only allows some servers (`servers.allowed`) | See the [managed policies](managed-policies) |

## Coming back to this computer, removing a server

- “This computer” in the projects bar, or “Go to This computer” in the band of a cut, brings the window back here. The app reopens on the last machine you were on.
- Settings › “Servers” lists the servers this computer knows, their state and the matching gesture: “Open”, “Try again”, “Copy the command”, “Pair again”.
- “Remove” forgets the token and the connection on this computer, and asks the server to revoke this computer if it can be reached. The server’s agents keep working; coming back will take a new code.
