# Des identifiants par projet

Un tunnel VPN par projet, de bout en bout : le manifeste, ce que voit l’utilisateur, ce que lit le plugin, ce que `.escouade.yml` peut et ne peut pas, les programmes qu’il lance et ce qui est refusé.

## Ce qu’on veut

Un plugin ouvre, pour chaque projet, un tunnel vers le réseau de son client. Deux projets d’un même client partagent un serveur et des identifiants ; un troisième en a d’autres. Trois outils d’Escouade 2.2.5 le permettent :

- **un profil** (`contributes.profiles`) : un serveur, un identifiant, un mot de passe, une configuration OpenVPN, que l’utilisateur crée autant de fois qu’il veut sur la page du plugin ;
- **un réglage de chaque projet** (`"scope": "project"`) : le profil que prend ce projet, et ses routes ;
- **des jokers** dans `permissions.commands` et **une limite de programmes** (`limits.processes`) : les programmes du tunnel, chaque mot qui varie vérifié par l’hôte.

Un secret n’a pas de portée de projet : un secret de `"scope": "project"` se lit, mais le plugin reste coupé. Les identifiants d’un projet passent par un profil, que plusieurs projets peuvent partager.

## Le manifeste

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

- `profile:vpn` dans `permissions.secrets` demande les secrets des profils du genre `vpn` ; sans lui, `secrets/get` les refuse (`-32001`).
- `nameField` nomme le champ qui donne son nom au profil, dans les listes.
- `multiline` et `import` font de la configuration une zone de plusieurs lignes, qu’on colle ou qu’on importe d’un fichier lu une fois.
- `escouade.min` dit `2.2.5` : une Escouade plus ancienne ne sert ni les profils, ni les jokers, ni la limite ([les versions](plugins/versioning#le-contrat-de-la-2-2-5)).

## Ce que voit l’utilisateur

- **À l’installation**, la fenêtre montre chaque commande, chaque joker en pastille de mots (« un fichier de son dossier temporaire », « un programme qu’il a lancé »), puis « Jusqu’à 8 programmes à la fois », « 4 sans le demander » et la raison de l’auteur.
- **Sur la page du plugin**, onglet « Réglages » : « Profil VPN », « Ajouter un profil… ». Chaque profil s’ouvre dans une fenêtre, ses champs simples puis ses secrets ; la configuration se colle ou s’importe (« Importer un fichier… »). Gardé, un secret ne se relit pas : « Gardé · 84 lignes · 5,1 Ko », « Remplacer… », « Oublier ».
- **Dans Réglages › Projets › *projet* › Plugins**, chaque projet choisit son profil et règle ses routes. Supprimer un profil efface ses secrets et les projets qui le prenaient passent à « Aucun profil ».

## Ce que lit le plugin

Le profil d’un projet est un réglage comme un autre : `settings/get` de ce projet rend son identifiant, tiré par Escouade, ou `null` tant qu’il n’en a choisi aucun. Le plugin lit ensuite les champs simples du profil (`profiles/get`) et ses secrets (`secrets/get` avec `profile`) :

`sdk/doc-examples/ts/src/profiles.ts` (extrait)

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

Un réglage de chaque projet se lit de même, par ses rangs : la valeur de cette machine pour le projet, sinon celle de son `.escouade.yml`, sinon le `default` du manifeste.

`sdk/doc-examples/ts/src/profiles.ts` (extrait)

```ts
// A setting of each project: this machine's value for it, else its `.escouade.yml`'s, else the default.
export async function routes(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.routes;
}
```

En Rust, les mêmes appels :

`sdk/doc-examples/rust/src/profiles.rs` (extrait)

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

Le kit 0.1.4 les nomme `Cx::profile_of`, `Cx::secret_of`, `Cx::profiles`, `Cx::profile` et `Cx::settings` ([le kit Rust](plugins/rust#les-profils-et-les-reglages-d-un-projet)).

Une instance sert tous les projets du plugin : c’est son code qui n’utilise le profil de atlas-web que pour atlas-web, et la revue du store le lit. Le journal dit `was given its secret "password" of profile "Client A"`, jamais la valeur ; d’un secret sur plusieurs lignes, il masque chaque ligne de plus de 8 caractères.

## Ce que `.escouade.yml` peut et ne peut pas

Un dépôt cloné de n’importe qui ne doit pas choisir le réseau par lequel passent tes agents. Ce que son `.escouade.yml` donne au plugin, sous `plugins.<id>` :

```yaml
plugins:
  dev.exemple.tunnel:
    routes: 10.20.0.0/16
    profile: client-a
```

- **Un réglage simple de portée `project`** (`routes`) : oui. La valeur de cette machine passe devant ; « Écrire dans .escouade.yml », sur la page Plugins du projet, écrit cette clé seule.
- **Un profil** : jamais choisi. `profile: client-a` n’est qu’une proposition par un nom : la page du projet la dit dans une bande (« .escouade.yml propose le profil « client-a » pour Tunnel. ») et c’est l’utilisateur qui choisit.
- **Un secret, une valeur importée d’un fichier, un réglage `"repo": false`** : jamais. Une clé secrète du fichier est ignorée.
- **Rien, pour un projet où le plugin n’est pas actif** (coupé pour ce projet, ou un projet privé où il n’a pas été allumé) : seulement les défauts, et `null` pour un profil.

## Les programmes du tunnel

Le plugin écrit la configuration dans son dossier temporaire, puis lance le tunnel : `{path:temp}` est ce fichier, `{pid}` le premier programme.

`sdk/doc-examples/ts/src/profiles.ts` (extrait)

```ts
// The configuration in its temporary folder, then the tunnel: `{path:temp}` is that file, `{pid}` the first program.
export async function tunnel(p: Plugin, ovpn: string) {
  await p.fs.write({ root: 'temp', path: 'tunnel.ovpn', text: ovpn });
  const vpn = await p.runTyped({ command: 'unshare', args: ['-Urn', 'openvpn', '--config', 'tunnel.ovpn', '--script-security', '1'] });
  const net = await p.runTyped({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', String(vpn.pid), 'tap0'] });
  return [vpn, net];
}
```

- **`{path:temp}`** : le plugin donne `tunnel.ovpn`, l’hôte le résout sous son dossier temporaire, vérifie que c’est un fichier qui y reste, et passe au programme son chemin absolu.
- **`{pid}`** : le pid d’un programme vivant que cette instance a lancé ; `proc/spawn` le rend avec `proc` (`p.runTyped` et `Cx::spawn_typed` l’exigent).
- **`--script-security 1` après `--config`** : OpenVPN prend la dernière valeur d’une option donnée deux fois, ce qui interdit les scripts que demanderait la configuration. À vérifier sur ta version d’OpenVPN : la revue le lit.
- **Huit programmes à la fois** : deux par tunnel, quatre tunnels. Le neuvième est refusé tant qu’aucun n’a fini : « 8 programs at once at most for this plugin (limits.processes) ».

## Ce qui est refusé

| Ce que le plugin demande | Ce qu’en dit Escouade |
| --- | --- |
| Un secret d’un profil sans `profile` (`secrets/get { "key": "password" }`) | `-32602` : un champ de profil se demande avec son profil, jamais un autre secret à sa place |
| Un profil qu’il n’a pas, ou un identifiant qui n’en est pas un | `-32602` |
| Un champ qui n’est pas un secret de ce genre | `-32602` |
| Les secrets d’un genre sans `profile:<genre>` dans ses permissions | `-32001` |
| Un mot qui n’a pas la forme de son joker : une option (`-x`), `..`, un chemin absolu, un pid d’un autre process | `-32003`, le mot fautif et la raison au journal |
| Un programme de plus que `limits.processes` | `-32003`, la limite dite |

Au manifeste, `escouade-plugin validate` dit le champ fautif et pourquoi ([les jokers](plugins/capabilities#les-jokers)).

## Le tester

- **Le banc** (`escouade-plugin test .`) joue un profil par genre, choisi par le projet du banc, des secrets sur plusieurs lignes, et lance les programmes pour de vrai. Ses scénarios « Profils », « Secret de plusieurs lignes », « Jokers des commandes » et « Limite de programmes » disent ce qui a été refusé ([tester un plugin](plugins/testing#le-banc-de-conformite)).
- **Le faux Escouade** du SDK, donné le manifeste, sert les profils, les réglages de chaque projet, les jokers et la limite comme l’app ([les tests](plugins/sdk#les-tests)).
