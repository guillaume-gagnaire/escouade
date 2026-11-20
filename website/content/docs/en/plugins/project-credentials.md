# Credentials per project

A VPN tunnel per project, end to end: the manifest, what the user sees, what the plugin reads, what `.escouade.yml` can and cannot do, the programs it runs and what is refused.

## What we want

A plugin opens, for each project, a tunnel to its client’s network. Two projects of the same client share a server and credentials; a third one has others. Three tools of Escouade 2.2.5 make it possible:

- **a profile** (`contributes.profiles`): a server, a login, a password, an OpenVPN configuration, which the user creates as many times as they want on the plugin’s page;
- **a setting of each project** (`"scope": "project"`): the profile this project takes, and its routes;
- **jokers** in `permissions.commands` and **a limit of programs** (`limits.processes`): the programs of the tunnel, each word that varies checked by the host.

A secret has no project scope: a secret of `"scope": "project"` is read, but the plugin stays disabled. A project’s credentials go through a profile, which several projects can share.

## The manifest

```json
{
  "id": "dev.exemple.tunnel",
  "name": "Tunnel",
  "version": "1.0.0",
  "escouade": { "api": "1", "min": "2.2.5" },
  "run": { "js": "dist/plugin.mjs" },
  "activation": ["event:agent.created"],
  "permissions": {
    "events": ["agent.created"],
    "secrets": ["profile:vpn"],
    "files": [{ "root": "temp", "access": "write" }],
    "commands": [
      "unshare -Urn openvpn --config {path:temp} --script-security 1",
      "slirp4netns --configure --mtu=65520 {pid} tap0"
    ]
  },
  "limits": { "processes": 8, "reason": { "fr": "Deux programmes par tunnel, un tunnel par projet.", "en": "Two programs a tunnel, a tunnel a project." } },
  "contributes": {
    "profiles": [{
      "id": "vpn",
      "title": { "fr": "Profil VPN", "en": "VPN profile" },
      "nameField": "name",
      "fields": [
        { "key": "name", "type": "string", "title": { "fr": "Nom du profil", "en": "Profile name" } },
        { "key": "server", "type": "string", "title": { "fr": "Serveur", "en": "Server" } },
        { "key": "proto", "type": "select", "title": { "fr": "Protocole", "en": "Protocol" }, "options": ["udp", "tcp"], "default": "udp" },
        { "key": "password", "secret": true, "title": { "fr": "Mot de passe", "en": "Password" } },
        { "key": "ovpn", "secret": true, "multiline": true, "import": [".ovpn", ".conf"], "title": { "fr": "Configuration OpenVPN", "en": "OpenVPN configuration" } }
      ]
    }],
    "settings": [
      { "key": "profile", "type": "profile", "of": "vpn", "scope": "project", "title": { "fr": "Profil VPN", "en": "VPN profile" } },
      { "key": "routes", "type": "string", "scope": "project", "default": "10.0.0.0/8", "title": { "fr": "Routes par le tunnel", "en": "Routes through the tunnel" } }
    ]
  }
}
```

- `profile:vpn` in `permissions.secrets` asks for the secrets of the profiles of the kind `vpn`; without it, `secrets/get` refuses them (`-32001`).
- `nameField` names the field that gives the profile its name, in the lists.
- `multiline` and `import` make the configuration an area of several lines, pasted or imported from a file read once.
- `escouade.min` says `2.2.5`: an older Escouade serves neither the profiles, nor the jokers, nor the limit ([versioning](plugins/versioning#the-contract-of-2-2-5)).

## What the user sees

- **At the installation**, the window shows each command, each joker as a pill of words (“a file of its temporary folder”, “a program it started”), then “Up to 8 programs at once”, “4 without asking” and the author’s reason.
- **On the plugin’s page**, “Settings” tab: “VPN profile”, “Add a profile…”. Each profile opens in a window, its simple fields then its secrets; the configuration is pasted or imported (“Import a file…”). Once kept, a secret is not read back: “Kept · 84 lines · 5.1 KB”, “Replace…”, “Forget”.
- **In Settings › Projects › *project* › Plugins**, each project chooses its profile and sets its routes. Deleting a profile erases its secrets, and the projects that took it go to “No profile”.

## What the plugin reads

A project’s profile is a setting like any other: `settings/get` of that project gives its id, drawn by Escouade, or `null` while it chose none. The plugin then reads the profile’s simple fields (`profiles/get`) and its secrets (`secrets/get` with `profile`):

`sdk/doc-examples/ts/src/profiles.ts` (excerpt)

```ts
// The profile the project chose (a `profile` setting of `"scope": "project"`): its id, or null while it chose none.
// Only the user chooses it, never a `.escouade.yml`.
export function profileOf(p: Plugin, project: string) {
  return p.profiles.of(project, 'profile');
}

// Its simple fields, then its secrets: a configuration on several lines comes whole.
export async function credentials(p: Plugin, profile: string) {
  const fields = await p.profiles.get('vpn', profile);
  const password = await p.secret('password', { profile });
  const ovpn = await p.secret('ovpn', { profile });
  return { server: fields?.server, password, ovpn };
}
```

A setting of each project is read the same way, by its ranks: this machine’s value for the project, else its `.escouade.yml`’s, else the manifest’s `default`.

`sdk/doc-examples/ts/src/profiles.ts` (excerpt)

```ts
// A setting of each project: this machine's value for it, else its `.escouade.yml`'s, else the default.
export async function routes(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.routes;
}
```

In Rust, the same calls:

`sdk/doc-examples/rust/src/profiles.rs` (excerpt)

```rust
/// The profile the project chose (a `profile` setting of `"scope": "project"`): its id, none while it chose none.
pub async fn profile_of(cx: &Cx, project: &str) -> Result<Option<String>, Refusal> {
    let settings = cx
        .request("settings/get", json!({ "project": project }))
        .await?;
    Ok(settings["profile"].as_str().map(str::to_string))
}

/// A secret field of that profile: a configuration on several lines comes whole.
pub async fn secret_of(cx: &Cx, profile: &str, key: &str) -> Result<Option<String>, Refusal> {
    let v = cx
        .request("secrets/get", json!({ "key": key, "profile": profile }))
        .await?;
    Ok(v.as_str().map(str::to_string))
}
```

The kit 0.1.4 names them `Cx::profile_of`, `Cx::secret_of`, `Cx::profiles`, `Cx::profile` and `Cx::settings` ([the Rust kit](plugins/rust#profiles-and-the-settings-of-a-project)).

One instance serves all the projects of the plugin: it is its code that uses the profile of atlas-web only for atlas-web, and the store’s review reads it. The journal says `was given its secret "password" of profile "Client A"`, never the value; of a secret on several lines, it masks each line of more than 8 characters.

## What `.escouade.yml` can and cannot do

A repository cloned from anyone must not choose the network your agents go through. What its `.escouade.yml` gives the plugin, under `plugins.<id>`:

```yaml
plugins:
  dev.exemple.tunnel:
    routes: 10.20.0.0/16
    profile: client-a
```

- **A simple setting of `project` scope** (`routes`): yes. This machine’s value comes first; “Write into .escouade.yml”, on the project’s Plugins page, writes that key alone.
- **A profile**: never chosen. `profile: client-a` is only a proposal by a name: the project’s page says it in a band (“.escouade.yml proposes the profile “client-a” for Tunnel.”) and the user chooses.
- **A secret, a value imported from a file, a `"repo": false` setting**: never. A secret key of the file is ignored.
- **Nothing, for a project where the plugin is not on** (turned off for that project, or a private project where it was not turned on): only the defaults, and `null` for a profile.

## The programs of the tunnel

The plugin writes the configuration in its temporary folder, then runs the tunnel: `{path:temp}` is that file, `{pid}` the first program.

`sdk/doc-examples/ts/src/profiles.ts` (excerpt)

```ts
// The configuration in its temporary folder, then the tunnel: `{path:temp}` is that file, `{pid}` the first program.
export async function tunnel(p: Plugin, ovpn: string) {
  await p.fs.write({ root: 'temp', path: 'tunnel.ovpn', text: ovpn });
  const vpn = await p.runTyped({ command: 'unshare', args: ['-Urn', 'openvpn', '--config', 'tunnel.ovpn', '--script-security', '1'] });
  const net = await p.runTyped({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', String(vpn.pid), 'tap0'] });
  return [vpn, net];
}
```

- **`{path:temp}`**: the plugin gives `tunnel.ovpn`, the host resolves it under its temporary folder, checks it is a file that stays there, and gives the program its absolute path.
- **`{pid}`**: the pid of a living program this instance started; `proc/spawn` gives it with `proc` (`p.runTyped` and `Cx::spawn_typed` require it).
- **`--script-security 1` after `--config`**: OpenVPN takes the last value of an option given twice, which forbids the scripts the configuration would ask for. Check it on your version of OpenVPN: the review reads it.
- **Eight programs at once**: two a tunnel, four tunnels. The ninth is refused until one has ended: “8 programs at once at most for this plugin (limits.processes)”.

## What is refused

| What the plugin asks | What Escouade says |
| --- | --- |
| A secret of a profile without `profile` (`secrets/get { "key": "password" }`) | `-32602`: a field of a profile is asked with its profile, never another secret in its place |
| A profile it does not have, or an id that is none | `-32602` |
| A field that is no secret of that kind | `-32602` |
| The secrets of a kind without `profile:<kind>` in its permissions | `-32001` |
| A word that does not have the form of its joker: an option (`-x`), `..`, an absolute path, a pid of another process | `-32003`, the faulty word and the reason in the journal |
| One program more than `limits.processes` | `-32003`, the limit said |

In the manifest, `escouade-plugin validate` says the faulty field and why ([the jokers](plugins/capabilities#the-jokers)).

## Test it

- **The bench** (`escouade-plugin test .`) plays one profile by kind, chosen by the bench’s project, secrets on several lines, and runs the programs for real. Its scenarios “Profiles”, “Multi-line secret”, “Command jokers” and “Program limit” say what was refused ([testing a plugin](plugins/testing#the-conformance-bench)).
- **The fake Escouade** of the SDK, given the manifest, serves the profiles, the settings of each project, the jokers and the limit as the app does ([the tests](plugins/sdk#the-tests)).
