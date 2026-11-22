# Device policies

An IT department can impose Escouade’s settings on the devices it manages, with its own tools: Group Policy or Intune on Windows, a configuration profile on macOS, configuration files on Linux.

> [!NOTE]
> This page is generated from Escouade’s list of policies (`crates/escouade-policy/src/list.rs`), as are the files it cites. No team server is needed.

## What a policy does

- A policy set holds for the whole app, in every project. It goes before the device’s setting, the repository’s `.escouade.yml` and what a team recommends.
- The setting it locks is grayed, with a padlock and the line “Imposed by this device’s policy”.
- Escouade’s core refuses, not only the window: a write through the MCP server or through the repository’s `.escouade.yml` is refused too, with the reason.
- Each policy has its sort:
  - **restrict**: it is imposed one way only (the statistics off, never on). Another value is ignored and said;
  - **value**: it sets a value (the default mode, the team server’s address);
  - **sensitive**: a proxy’s address, a file of authorities. Set by the managed device, it applies; from a team, it will wait for each member’s consent.
- A policy changed while the app runs applies within a few seconds, without a restart. An agent in a mode no longer allowed ends its turn, then goes to the nearest allowed mode, and says so in its conversation. An agent of a harness no longer allowed stops at the end of its turn.

## The policies

Each policy has a key, the same name everywhere: the registry value, the profile’s key, the JSON file’s key. A Boolean is given `true` or `false` (`1` or `0` in the registry). A list is an ADMX list, an array of the profile or the JSON; a lone string counts as a list of one item. What is “to come” is read and kept, and will apply with the feature that carries it.

### Permission modes

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Deny the Bypass mode | `permissions.allowBypass` · Boolean `false` (`0`) · restrict | The mode where a harness asks nothing (bypassPermissions) is no longer allowed to any agent. Only the value false (0) is imposed. | Settings › Agents › “Allow Bypass mode”, off and grayed; the Bypass mode refused everywhere. |
| Allowed permission modes | `permissions.modes` · list among `plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions` · restrict | The list of the modes agents may take: plan, default, acceptEdits, auto, bypassPermissions. An agent in another mode ends its turn, then goes to the nearest allowed mode. | The modes not allowed grayed in “Default mode of agents”, the board’s mode and an agent’s mode. |
| Default agent mode | `permissions.defaultMode` · string `plan`, `default`, `acceptEdits`, `auto` · value | The mode of new agents: plan, default, acceptEdits or auto. It stays within the allowed modes: outside them, the nearest allowed one. | Settings › Agents › “Default mode of agents”, grayed. |

### Harnesses and models

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Allowed harnesses | `harnesses.allowed` · list · restrict | The list of the harnesses agents may use, by their id (claude, codex, copilot, cursor, opencode, gemini…). An agent of a harness no longer allowed stops at the end of its turn. | The harness of a project, an agent, a ticket: a harness not allowed is grayed and refused. |
| Allowed models by harness | `harnesses.allowedModels` · JSON · restrict | A JSON object that gives, for each harness, the list of its allowed models. A harness absent from the object keeps all its models. | An agent or a ticket on a model not allowed is refused at its start. |
| Maximum effort | `harnesses.maxEffort` · string `low`, `medium`, `high`, `xhigh`, `max` · restrict | The highest effort an agent may be given: low, medium, high, xhigh or max. | The effort of the board’s agents brought down to this ceiling; a higher effort refused. |

### Plugins

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Allowed plugin origins | `plugins.allowedOrigins` · list among `store-verified`, `store`, `team-store`, `git`, `folder`, `dev` · restrict | Where a plugin may be installed from: store-verified (the store, verified versions), store (the store, any version), team-store (the team’s store), git, folder (a folder), dev (a folder in development). | Installing a plugin of another origin is refused and said. |
| Allowed plugins | `plugins.allowed` · list · restrict | The list of the plugin ids that may be installed. Absent, all may. | Installing a plugin outside the list is refused and said. |
| Denied plugins | `plugins.denied` · list · restrict | The list of the plugin ids that are never installed, even when allowed elsewhere. | Installing a plugin of the list is refused and said. |
| Install verified versions only | `plugins.verifiedOnly` · Boolean `true` (`1`) · restrict | A version of the store without the “Verified by Escouade” seal is refused, at an install as at an update. Only the value true (1) is imposed. | Plugins › “Install verified versions only”, on and grayed. |
| Deny the file of the plugins’ secrets | `secrets.fileFallback` · Boolean `false` (`0`) · restrict | On a device without a system keychain, a plugin’s secret can no longer be kept in a file readable by its user alone: only the keychain keeps it. A file already chosen is no longer read; its secrets move to the keychain or are erased with it. Only the value false (0) is imposed. | “Keep in a file only you can read” is no longer offered at a secret’s field, and Plugins › “Secrets kept in a file” bears the padlock. |

### Editor

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Editor’s allowed extensions | `extensions.allowed` · JSON · restrict | A JSON object like VS Code’s extensions.allowed: its keys are * (all), a publisher of Open VSX or an extension (publisher.name), its values true, false, stable (no pre-release) or the list of the versions allowed. The most precise key says it. An object that names anything allows only what it names. It adds to the lists of the settings and of the .escouade.yml files: an extension must be allowed by each. | General › “Editor’s allowed extensions”, grayed; an extension outside the list is refused before any download, and an installed extension it leaves out is no longer loaded. |

### Features

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Kanban feature | `features.kanban` · Boolean `true`, `false` · value | Tickets, the autopilot and ticket sources: true turns it on, false turns it off (its tab, its MCP tools, its autopilot). Its tickets are kept. | Settings › Features › Kanban, grayed. |
| Editor feature | `features.editor` · Boolean `true`, `false` · value | Opening, editing and searching the project’s files: true turns it on, false turns it off (the editor view, Ctrl P, its language servers). | Settings › Features › Editor, grayed. |
| Docs space feature | `features.docs` · Boolean `true`, `false` · value | The specification, decisions, documents and discussions: true turns it on, false turns it off (its tab, its MCP tools). Its pages are kept. | Settings › Features › Docs space, grayed. |
| Cost feature | `features.cost` · Boolean `true`, `false` · value | The cost of turns, agents and tickets, and the cache temperature: true turns it on, false turns it off. | Settings › Features › Cost, grayed. |
| Statistics feature | `features.stats` · Boolean `true`, `false` · value | Activity and spending over time: true turns it on, false turns it off (its view). It has nothing to do with telemetry (telemetry.enabled). | Settings › Features › Statistics, grayed. |
| Workflows feature | `features.workflows` · Boolean `true`, `false` · value | A project’s chains of agents and skills: true turns it on, false turns it off (its tab, its MCP tool, their reading). | Settings › Features › Workflows, grayed. |
| Scheduled tasks feature | `features.scheduled` · Boolean `true`, `false` · value | Tickets that come back at a set time: true turns it on, false turns it off (its tasks no longer run). It also goes with the Kanban. | Settings › Features › Scheduled tasks, grayed. |
| Overseer feature | `features.overseer` · Boolean `true`, `false` · value | The agent that drives every project: true turns it on, false turns it off (its screen, its shortcut, its MCP tools; it stops). | Settings › Features › Overseer, grayed. |
| Voice feature | `features.voice` · Boolean `true`, `false` · value | Dictation and spoken conversation: true turns it on, false turns it off (the microphone, its shortcut, the listening). | Settings › Features › Voice, grayed. |

### Projects

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Private projects by default | `projects.forcePrivate` · Boolean `true` (`1`) · restrict | Every project is private: its agents see no other project, and no other project sees it. Only the value true (1) is imposed. | “New projects are private” and each project’s “Private project”, on and grayed. |
| Ceiling of the tickets in parallel | `projects.maxParallel` · number 1–64 · restrict | The most tickets “In progress” at once in each project’s Kanban, from 1 to 64. It lowers the machine’s bound (16 on a desktop, 64 on a server), never raises it; between this device and the team, the lower one holds. | Settings › Kanban › “In parallel” goes no higher, the ceiling and where it comes from said under the setting; set_board refuses higher, and a Kanban set higher starts no more tickets than the ceiling. |
| Denied file patterns | `projects.deniedFiles` · list · restrict | File patterns (the .gitignore way) agents neither read nor write, added to those of the projects. | Read and kept; applied to the agents with a project’s fence of files (to come). |

### Proxy and certificates

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Proxy address | `proxy.url` · string · sensitive | The HTTP(S) proxy of the app, the plugins and the agents, for example http://proxy.example.com:3128. | Settings › Advanced › Network, the proxy’s address grayed. |
| Proxy exclusions | `proxy.exclusions` · list · sensitive | The hosts reached without the proxy (NO_PROXY), one per line. | Settings › Advanced › Network, the exclusions grayed. |
| Deny skipping the TLS check | `proxy.denyInsecureTls` · Boolean `true` (`1`) · restrict | “Skip TLS certificate verification” is off and can no longer be turned on. Only the value true (1) is imposed. | Settings › Advanced › Network › “Skip TLS certificate verification”, off and grayed. |
| Certificate authorities file | `proxy.authorities` · string · sensitive | The path of a .pem file of authorities trusted besides the system’s store (a corporate proxy’s root). | Settings › Advanced › Network › “Extra certificate authorities”, grayed. |

### Telemetry and voice

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Turn the usage statistics off | `telemetry.enabled` · Boolean `false` (`0`) · restrict | The anonymous statistics are no longer sent. Only the value false (0) is imposed. | Settings › General › “Usage statistics”, off and grayed. |
| Deny online voices | `voice.onlineAllowed` · Boolean `false` (`0`) · restrict | The dictation and the conversation’s voice stay on the device (Whisper and Piper): no online service. Only the value false (0) is imposed. | Settings › Voice, “Online” brought back to local (Whisper, Piper) and refused. |
| Turn the dictation off | `voice.dictation` · Boolean `false` (`0`) · restrict | The dictation no longer starts. Only the value false (0) is imposed. | Settings › Voice › Dictation, grayed; the dictation refused. |

### Remote Control

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Deny the Remote Control | `remoteControl.enabled` · Boolean `false` (`0`) · restrict | No agent is reachable from claude.ai. Only the value false (0) is imposed. | “Remote control for all agents”, a project’s and an agent’s, off and grayed. |

### Updates

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Stable channel only | `updates.channel` · string `stable`, `beta` · restrict | No beta version is offered. Only the value stable is imposed. | Settings › General › “Receive beta versions”, off and grayed. |
| Turn automatic updates off | `updates.automatic` · Boolean `false` (`0`) · restrict | A new version is offered, never installed by itself: the IT department deploys its own. Only the value false (0) is imposed. | Settings › General › “Install updates automatically”, off and grayed. |

### Servers

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Allowed servers | `servers.allowed` · list · restrict | The addresses of the team servers and the servers in server mode the app may reach. Absent, all. | A server outside the list is refused, with the reason. |
| Deny the personal space | `servers.personalSpace` · Boolean `false` (`0`) · restrict | Only the servers of the allowed servers’ list are added as spaces; without the list, none. This device stays a space. Only the value false (0) is imposed. | A server outside the list is refused before any connection, with the reason. |
| Deny the aggregate view | `servers.aggregateView` · Boolean `false` (`0`) · restrict | The “Everywhere” view shows only the numbers of a work server (of the allowed servers’ list, or any server without one), no text nor gesture. Set on a server, it holds for it. Only the value false (0) is imposed. | “Everywhere” gives only the numbers of these spaces. |
| Notifications without content | `servers.contentlessNotifications` · Boolean `true` (`1`) · restrict | The notifications from a work server (of the allowed servers’ list, or any server without one) say an agent is waiting, never what. Set on a server, it holds for it. Only the value true (1) is imposed. | The notifications of these spaces carry no text. |
| Deny the remote API | `servers.remoteApi` · Boolean `false` (`0`) · restrict | The core’s remote API (the server mode, the phone) does not turn on on this device: no other device drives it. Only the value false (0) is imposed. | The remote API refused at its start and at each connection. |
| Deny the access from the phone | `servers.phone` · Boolean `false` (`0`) · restrict | The phone’s WireGuard tunnel (ESC-567) does not turn on on this device nor on a server that receives it: no UDP port opened, nothing in the box, no API on the Tailscale address. The server mode through SSH stays allowed. Only the value false (0) is imposed. | Settings › Phone, off and grayed; a tunnel that is on turns off at once. |

### Outside MCP clients

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Outside MCP clients | `mcp.externalClients` · string `denied`, `readOnly`, `allowed` · restrict | What Claude outside Escouade (an outside MCP client) may do through Escouade’s MCP server: denied (nothing), readOnly (read only), allowed (everything). Escouade’s own agents are not concerned. | An outside client refused at its connection (denied), or its tools that write refused (readOnly). |

### Enrollment

| Policy | Key and type | What it does | On the device |
| --- | --- | --- | --- |
| Team server | `team.server` · string · value | The address of the company’s team server: the member only has to accept their invitation. | Kept by the app; the screen to join the team will fill it in (to come). |
| Team fingerprint | `team.fingerprint` · string · value | The fingerprint of the team expected on the team server: a team of another fingerprint is refused. | Kept by the app; checked when joining the team (to come). |
| Refuse any other team server | `team.onlyThisServer` · Boolean `true` (`1`) · restrict | Only the team server the policy sets may be joined. Only the value true (1) is imposed. | Another team server is refused, with the reason. |

## Deploy

### Windows: Group Policy and Intune

1. Take `escouade.admx` and its translations `fr-FR/escouade.adml` and `en-US/escouade.adml` from [the policies/windows folder of the repository](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/windows).
2. Put them in `C:\Windows\PolicyDefinitions` (with the `fr-FR` and `en-US` folders) for one device, or in your domain’s central store (`\\<domain>\SYSVOL\<domain>\Policies\PolicyDefinitions`).
3. In the Group Policy editor, the policies are under Administrative Templates › Escouade, for the computer as for the user.
4. In Intune: Devices › Configuration › Import ADMX, then a profile of the Imported Administrative templates type.

Escouade reads `HKLM\SOFTWARE\Policies\Escouade`, then `HKCU\SOFTWARE\Policies\Escouade`; for a same key, the machine wins. Without the ADMX, a script can write the registry itself: the value is named after the key, a Boolean is a `REG_DWORD` of `0` or `1`, a string a `REG_SZ`, a JSON setting a `REG_SZ` that opens with a brace, and a list a subkey named after the key whose values are named `1`, `2`, `3`…

```bat
reg add "HKLM\SOFTWARE\Policies\Escouade" /v telemetry.enabled /t REG_DWORD /d 0 /f
reg add "HKLM\SOFTWARE\Policies\Escouade\permissions.modes" /v 1 /t REG_SZ /d plan /f
reg add "HKLM\SOFTWARE\Policies\Escouade\permissions.modes" /v 2 /t REG_SZ /d default /f
```

> [!WARNING]
> Escouade refuses a `Policies\Escouade` key the user could write without being an administrator: its ACL was loosened. When the app runs as an administrator, it can no longer tell and reads everything.

### macOS: configuration profile

Escouade reads the managed preferences domain `dev.gagnaire.escouade`. Only the values a configuration profile forces count: a `defaults write` of the same domain is ignored. [The policies/macos folder of the repository](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/macos) gives:

- `dev.gagnaire.escouade.plist`: the manifest of the keys in the ProfileManifests format, which ProfileCreator and iMazing Profile Editor read;
- `dev.gagnaire.escouade.json`: the schema for Jamf Pro (Application & Custom Settings › External Applications › Custom Schema);
- `escouade-example.mobileconfig`: an example profile with every key, to cut down to what you mean to impose before installing it or sending it through your MDM (Jamf, Intune, Kandji).

### Linux: /etc/escouade/policies

Escouade reads the files `/etc/escouade/policies/*.json`, in the order of their names; for a same key, the last one wins. Each file is a JSON object of keys and values. The folder, those above it and each file must belong to root, with no write for the group or the others; a symbolic link is refused. The schema `escouade-policies.schema.json` and an example file are in [the policies/linux folder of the repository](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/linux).

```sh
sudo mkdir -p /etc/escouade/policies
echo '{"telemetry.enabled": false, "permissions.modes": ["plan", "default"]}' \
  | sudo tee /etc/escouade/policies/10-company.json
sudo chmod 644 /etc/escouade/policies/10-company.json
```

## Enrollment

The managed device can set the address of the company’s team server (`team.server`) and the fingerprint of its team (`team.fingerprint`): the member will only have to accept their invitation. With `team.onlyThisServer`, the app refuses any other team server; with `servers.allowed`, any server outside the list. Today, the app keeps these values and holds the refusal; the screen to join the team, which will fill them in, comes with the team server.

## What the user sees

- An imposed setting is grayed, its control inert, with a padlock and “Imposed by this device’s policy”.
- In a bounded list (the modes, the harnesses), the choices not allowed are grayed, with the reason in their tooltip.
- A refused write says so: “This setting is imposed by this device’s policy (telemetry.enabled): it cannot be changed on this device.”
- What the reading could not apply (an unknown key, a malformed value, a registry key whose ACL was loosened) is listed in Settings › General › “Policies of this device”.
