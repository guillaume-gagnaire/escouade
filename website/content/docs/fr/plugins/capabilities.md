# Les capacités de l’hôte

Un plugin en WASM n’atteint rien par lui-même : ni le réseau, ni le disque, ni un programme. Cette page dit ce qu’il peut demander à Escouade, ce que chaque capacité permet, ce qu’Escouade refuse et ce que la fenêtre d’installation en dit.

## Le principe

Une capacité suit toujours le même chemin, pour `run.js` comme pour `run.wasm` :

- **Déclarée** au manifeste, dans `permissions` ou `limits`. Ce qui n’est pas déclaré répond l’erreur `-32003`, et le journal le garde.
- **Appliquée par Escouade** : le module n’a ni socket, ni descripteur de fichier, ni process. Chaque appel passe par l’hôte, qui le juge.
- **Dite** dans la fenêtre d’installation, ligne par ligne, avant que rien ne tourne. L’utilisateur peut retirer chaque ligne.
- **Gardée au journal** : la méthode et l’hôte ou la racine, jamais le chemin, la requête ou les en-têtes. Une commande y va avec ses arguments, secrets masqués.

Un plugin qui demande tout, pour montrer la forme :

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
  "permissions": {
    "events": ["ticket.column"],
    "network": ["api.example.dev", "{account}"],
    "secrets": ["token"],
    "files": [
      { "root": "plugin", "access": "read" },
      { "root": "temp", "access": "write" },
      { "root": "agent", "access": "write" },
      { "root": "agentHome", "access": "read" }
    ],
    "commands": ["git status --short", "git log *"]
  },
  "limits": { "fetchSeconds": 60, "fetchBytes": 8388608, "fileBytes": 1048576 },
```

| Capacité | Au manifeste | La fenêtre d’installation dit |
| --- | --- | --- |
| Un hôte | `permissions.network` | « Se connecte à », puis l’hôte |
| Les instances de ses comptes | `{account}` | « Se connecte aux instances que tu lui donnes » |
| Le serveur local d’un harness | `local:shared` | « Parle au serveur local du harness, lancé par Escouade » |
| Lire ou écrire un dossier | `permissions.files` | « Lit les fichiers de tes agents », « Écrit dans les dossiers de tes agents »… |
| Lancer un programme | `permissions.commands` | « Lance cette commande sur ton ordinateur », puis la commande |
| Des limites plus hautes | `limits` | Rien de plus |
| Des minuteurs | rien, c’est un droit de base | Rien |

## Le réseau

Tout ce qui sort passe par `net/fetch`, `net/stream` ou `ws/open` : voir [les appels](plugins/calls#le-reseau). Un plugin en JS garde le `fetch` standard, un plugin en Rust le client du kit.

### Un hôte nommé

`"network": ["api.example.dev"]`.

**Ce qu’elle permet.** Joindre cet hôte et ses sous-domaines, en https. `*.example.dev` prend les sous-domaines sans l’hôte lui-même.

**Ce qu’Escouade refuse.**

- http, et tout autre hôte ;
- une adresse IP écrite dans l’URL, que `permissions.network` ne nomme pas ;
- une adresse de la machine ou du réseau local, même pour un hôte déclaré : bouclage, privée, de lien local, non spécifiée, de multidiffusion ou de diffusion, en IPv4 comme en IPv6. Escouade juge les adresses que la connexion emploie, après la résolution du nom ;
- les redirections : le plugin reçoit la réponse 3xx et décide. En Rust, le client du kit les suit pour lui, et chaque saut est jugé comme un appel.

Un appel refusé répond `-32003`. Un appel qui échoue en route répond `-32004`.

**Réglages › Avancé › Réseau.** Le proxy, ses exclusions et les certificats non vérifiés sont suivis, et un changement atteint l’appel suivant, sans relancer le plugin. Par un proxy, seul le nom est jugé, et le journal le dit.

**La fenêtre** dit « Se connecte à », puis chaque hôte, avec « appliqué par Escouade ».

### Les instances de ses comptes : `{account}`

`"network": ["{account}"]`, pour un service auto-hébergé : GitHub Enterprise Server, GitLab, Gitea, Jira.

**Ce qu’elle permet.** Joindre l’hôte que l’utilisateur a saisi pour un compte que ce plugin sert, sur le port saisi, et lui seul. Pour cet hôte, http et le réseau local sont permis : c’est l’utilisateur qui l’a nommé.

**Ce qu’Escouade refuse.** Ses sous-domaines et tout autre hôte. La liste est relue à chaque appel : un compte retiré ferme son hôte.

**La fenêtre** dit « Se connecte aux instances que tu lui donnes ». `{account}` n’est pas un hôte : le store l’accepte tel quel.

Au banc, `escouade-plugin test <dossier> --account <adresse>` tient lieu de compte.

### Le serveur local d’un harness : `local:shared`

`"network": ["local:shared"]`, pour un harness dont Escouade lance le serveur, comme `opencode serve`.

**Ce qu’elle permet.** http vers `127.0.0.1:<port>`, seulement sur un port que le cœur a lu dans la sortie d’un process partagé qu’il a lancé pour ce plugin, tant que ce process vit. Ce trafic ne passe jamais par le proxy.

**Ce qu’Escouade refuse.** Tout autre port local. Le plugin ne nomme jamais un port que le cœur n’a pas lu, et `local:shared` ne sert ni un WebSocket ni un autre plugin.

**La fenêtre** dit « Parle au serveur local du harness, lancé par Escouade ».

### Les limites : `limits`

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
  "limits": { "fetchSeconds": 60, "fetchBytes": 8388608, "fileBytes": 1048576 },
```

| Clé | Ce qu’elle dit | Sans elle | Plafond de l’hôte |
| --- | --- | --- | --- |
| `fetchSeconds` | Le temps d’un `net/fetch` | 10 s | 120 s |
| `fetchBytes` | La taille d’une réponse | 1 Mio | 32 Mio |
| `fileBytes` | Ce que rend un `fs/read` | 256 Kio | 4 Mio |

Une valeur au-delà du plafond est ramenée à lui par l’hôte (`fileBytes` est refusé par le manifeste). La fenêtre ne dit rien de plus : une limite ne donne accès à rien.

### Les flux et le WebSocket

Ils ne se déclarent pas : ils suivent `network`.

- **`net/stream`** applique les règles de `net/fetch` à l’ouverture, puis un crédit : l’hôte ne lit plus la socket au-delà de 256 Kio non consommés, donc un plugin lent ralentit le serveur et non la mémoire de l’app. 8 flux à la fois. 120 s sans octet ferment un flux, sauf s’il est déclaré `"sse": true`.
- **`ws/open`** est en `wss` seulement, vers les mêmes hôtes, jamais `local:shared`. 4 sockets, 1 Mio par message. `ws/send` est refusé si ce qui attend dépasserait 256 Kio. Ses en-têtes ne vont pas au journal.

## Les fichiers : `files`

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
    "files": [
      { "root": "plugin", "access": "read" },
      { "root": "temp", "access": "write" },
      { "root": "agent", "access": "write" },
      { "root": "agentHome", "access": "read" }
    ],
```

**Ce qu’elle permet.** `fs/read`, `fs/write`, `fs/stat`, `fs/list` et `fs/canon`, sous une racine de cette liste fermée, qu’Escouade résout :

| Racine | Ce que c’est | La fenêtre dit |
| --- | --- | --- |
| `plugin` | Son dossier, en lecture seule : ce que le store a signé ne change pas. | « Lit son propre dossier » |
| `temp` | Un dossier à lui, partagé par ses agents. | « Garde des fichiers dans un dossier à lui » |
| `agent` | Le dossier de travail de l’agent. | « Lit les fichiers de tes agents », ou « Écrit dans les dossiers de tes agents » en orange |
| `agentRepo` | Son dépôt git, pour un worktree. | Les mêmes |
| `agentHome` | Le dossier de son compte, là où un harness garde ses identifiants. | « Lit le dossier de compte de tes agents » |

`access` vaut `read` ou `write`. Un plugin nomme une racine et un chemin relatif, ou un chemin de l’hôte sous une racine de cet agent : c’est ce que lit un harness. Seuls les plugins de harness reçoivent un agent, par `agent/attach`, avec les seules racines qu’ils déclarent ; un autre plugin a `plugin` et `temp`.

**Ce qu’Escouade refuse.**

- un chemin libre, un `~`, un `..`, un flux NTFS (`a.txt:b`), un lien qui ne mène nulle part ;
- ce qui sort d’une racine une fois les liens suivis. Le refus vient avant de dire si le fichier existe : il ne renseigne sur rien ;
- `plugin` en écriture, à la lecture du manifeste ;
- le dossier d’un projet privé dont l’agent est coupé, quel que soit le code du plugin ;
- une lecture de plus de 256 Kio, ou de `limits.fileBytes`.

Chaque écriture et chaque refus vont au journal.

**La revue** demande à quoi sert `agentHome` : ce dossier contient les identifiants d’un harness.

## Les programmes : `commands`

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
    "commands": ["git status --short", "git log *"]
```

**Ce qu’elle permet.** `proc/spawn` lance une de ces lignes. Le premier mot est un nom de programme, qu’Escouade cherche sur le PATH.

**Ce qu’Escouade refuse.**

- un chemin à la place du nom ;
- ce qui n’est pas une entrée, mot pour mot ; la suite n’est permise que si l’entrée finit par `*` ;
- un shell : sous Windows, ni `.cmd` ni `.bat`, donc aucun outil installé par npm ;
- `dir` autre qu’une racine (`agent`, `project`), et un projet privé où le plugin n’est pas activé. Sans `dir`, le programme tourne dans `temp` ;
- une variable `ESCOUADE_*`, `PATH`, `LD_*`, `DYLD_*` ou `NODE_OPTIONS` dans `env` : elle ferait tourner autre chose.

Chaque programme meurt avec l’instance du plugin ou à `proc/kill`, avec ce qu’il a lancé. Quatre à la fois. Sa sortie n’est plus lue au-delà de 256 Kio qui n’ont pas atteint le module.

**La fenêtre** dit « Lance cette commande sur ton ordinateur », puis la commande, en rouge, une ligne par commande.

> [!WARNING]
> Le module est isolé, le programme qu’il lance ne l’est pas : il a les droits de l’utilisateur. Une entrée en `*` sur un programme généraliste (`git *`, `docker *`) rouvre tout, par `git -c core.sshCommand=…` par exemple. C’est la capacité la plus forte : demande des lignes exactes, et attends-toi à justifier un `*` à la revue.

`permissions.commands` n’est pas `contributes.commands`, qui lance une ligne de shell sur un fait, après l’accord de l’utilisateur ([les événements](plugins/events#une-commande-sur-un-fait)).

## Les minuteurs

`timer/set`, `timer/cancel` et `timer/fired` ne se déclarent pas : c’est un droit de base, comme `log`. Le plancher est 10 ms, ce qui est dessous y est ramené, et 64 minuteurs à la fois. Ils s’arrêtent avec l’instance. `setTimeout` en JS et `sleep` du kit en Rust passent par eux.

## Ensuite

- [Les appels](plugins/calls) : chaque méthode, avec un exemple en TS et en Rust.
- [Les permissions](plugins/permissions) : tout ce qu’un plugin peut demander, vu de l’utilisateur.
- [La sécurité des plugins](plugins/security) : ce que la revue en fait.
