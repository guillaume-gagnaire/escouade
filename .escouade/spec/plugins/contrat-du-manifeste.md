---
title: "Le contrat du manifeste : réglages, secrets, profils, jokers et limite"
subject: "plugins"
sources: ["docs/PLUGINS-CONTRACT-STUDY.md (release/2.3, ESC-501) §1.5, §2 à §6, §8 et §10","design/plugins-contract-2-3/captures/install.png (release/2.3)","issues GitHub #38, #39, #40, #42 (un auteur de plugin tiers)","crates/escouade-plugin-cli/src/manifest.rs","crates/escouade-plugins/src/procs.rs","crates/escouade-plugins/src/files.rs","crates/escouade-plugins/src/plugins_jokers_tests.rs","schemas/plugin-manifest.mjs","schemas/plugin-1.json","schemas/plugin-protocol-1.json","src/lib/plugins.ts","src/components/plugins/AskList.svelte","tests/fixtures/plugins/manifests.json","store/server/utils/checks.ts"]
tickets: ["ESC-525","ESC-528"]
decisions: ["le-contrat-du-manifeste-est-lu-avant-d-e","les-jokers-de-permissions-commands-sont"]
compared: null
---

# Le contrat du manifeste : réglages, secrets, profils, jokers et limite

Ce que le manifeste d'un plugin (`escouade-plugin.json`) peut dire de ses réglages, de ses secrets, de ses profils, des programmes qu'il lance et de leur nombre. Conçu par l'étude ESC-501 (`docs/PLUGINS-CONTRACT-STUDY.md`), d'après les issues #38, #39, #40 et #42 ; choix tranchés le 2026-11-17 : les profils (direction B), la liste des jokers et rien d'autre (choix n° 7 : pas de chemin dans le projet ni dans le dossier de l'agent), et tout dans la ligne 2.2.

**Ce qui est fait** :
- **ESC-525 (T1)** : le contrat est **lu, validé et refusé** par les deux lecteurs, celui de l'app (`crates/escouade-plugin-cli/src/manifest.rs`) et celui du store (`schemas/plugin-manifest.mjs`, contrôle n° 1), qui passent le même corpus (`tests/fixtures/plugins/manifests.json`) et rendent le même verdict cas par cas. Le schéma `plugin-1.json` le décrit, par ajouts.
- **ESC-528 (T5)** : les **jokers sont servis** : l'hôte les vérifie à chaque `proc/spawn`, la fenêtre d'installation et l'onglet Permissions les montrent en mots.
- Restent à servir : la page du plugin (T2), les profils et les secrets d'un projet (T4), la limite (T6).

## Compatibilité

- **Un manifeste d'aujourd'hui se lit pareil.** Chaque ajout est facultatif ; absent, il vaut le comportement d'avant : `scope: app`, pas de groupe, une ligne, 4 programmes à la fois, pas de joker.
- **Jamais servi à moitié.** Un plugin qui déclare un point que l'app lit mais ne sert pas encore est **chargé coupé, la raison dite**, et la fenêtre d'installation le dit avant l'installation. Trois points sont concernés :
  - des profils (`contributes.profiles`) ;
  - un secret de portée `project` ;
  - `limits.processes` au-delà de 4.

  La raison : « Ce plugin demande ce que cette version d'Escouade ne sert pas encore : … Il reste coupé plutôt que servi à moitié ; une version plus récente d'Escouade le servira. » Chaque ticket qui sert un point le retire de cette liste. Les jokers en sont sortis avec ESC-528.
- `escouade-plugin validate` dit ces points par un avertissement (⚠), sans échouer : un plugin écrit pour l'Escouade suivante se valide.
- Ce qui se lit et se montre comme avant, sans couper le plugin : `groups`, `description`, `multiline`, `lines`, `import`, `scope: project` sur un réglage simple, `repo`, `limits.processes` jusqu'à 4. `repo: false` est déjà tenu : la valeur de `.escouade.yml` n'est jamais prise pour ce réglage, ni pour un réglage qui a `import`.

## Réglages et secrets

- `scope` : `app` (d'office) ou `project`, sur un réglage et sur un secret.
- `repo` : `false` interdit la valeur venue de `.escouade.yml`. Jamais `true` pour un secret, un réglage `profile` ou un réglage qui a `import` (code `repo`).
- `description` : un texte, ou un texte par langue, sur un secret aussi.
- `multiline: true` : un drapeau, pas un type, sur un réglage `string` ou sur un secret. `lines` (3 à 20, 6 d'office) ne va qu'avec lui.
- `import` : une liste d'extensions (`.ovpn`, `.tar.gz`), sur un réglage `string` ou sur un secret. Jamais un chemin (code `extension`).
- Ailleurs, `multiline`, `lines`, `import` et `of` sont refusés (code `misplaced`).

## Groupes (`contributes.groups`)

`id`, `title`, `description`, `folded`, `fields` : les clés de ses réglages et de ses secrets, mêlées, dans l'ordre où les montrer. Sont refusés :
- une clé qui n'existe pas (code `field`) ;
- une clé dans deux groupes (code `duplicate`) ;
- une liste vide.

## Profils (`contributes.profiles`)

- Un genre : `id`, `title`, `nameField`, `fields`.
- Un champ : `key`, `title`, `description`, et un `type` (`string`, `number`, `boolean`, `select`) ou `secret: true`, sans `type`. Il accepte `multiline`, `lines` et `import`, jamais `scope` ni `of`.
- `nameField` nomme un champ `string` d'une ligne, non secret (code `field`).
- Un réglage `type: "profile"` dit son genre par `of`, qui doit être un genre déclaré (code `profile`).
- `permissions.secrets` peut demander `profile:<genre>`, pour un genre déclaré qui a un champ secret (code `profile`). Le contrôle n° 9 du store le tient comme `account:<genre>`. `profile:*` n'existe pas.
- Les secrets d'un profil ne sont pas des clés de la page du plugin.

## Les jokers de `permissions.commands`

### La grammaire, au manifeste

- Une entrée se découpe en mots sur les espaces, comme l'hôte la découpe. Les deux lecteurs suivent le White_Space d'Unicode : U+0085 sépare deux mots, U+FEFF non.
- Le premier mot est un nom de programme : jamais un chemin, ni un joker, ni une accolade.
- Un joker est un mot **entier**, parmi `{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}` et `{path:plugin}`.
- Un mot littéral ne contient aucune accolade ; il se compare octet pour octet.
- `*` reste seul et en dernier.
- Refusé avec le code `command` :
  - un joker inconnu (`{float}`, `{INT}`, `{path:..}`) ;
  - `{path:project}`, `{path:agent}` ;
  - un joker dans un mot (`--pid={int}`, `{path:temp}/x`) ;
  - un joker en premier mot ;
  - une accolade dans un mot littéral.

### Ce que l'hôte admet, à chaque `proc/spawn`

Le plugin demande une commande et ses arguments ; l'hôte la compare à chaque entrée (`procs::allowed`) :

- Un mot littéral est égal octet pour octet ; un joker admet **un seul mot**, de sa forme :

| Joker | Admet | Ce que dit la fenêtre (fr / en) |
| --- | --- | --- |
| `{int}` | `0` ou un nombre sans zéro en tête, 10 chiffres au plus, 4 294 967 295 au plus ; ni signe, ni hexadécimal, ni espace | « un nombre » / « a number » |
| `{port}` | un `{int}` de 1 à 65 535 | « un port » / « a port » |
| `{name}` | 1 à 64 lettres ASCII, chiffres, `_`, `.`, `-`, commençant par une lettre, un chiffre ou `_` ; jamais un nom de périphérique de Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM0` à `COM9`, `LPT0` à `LPT9`, quelle que soit la casse ou l'extension) | « un nom » / « a name » |
| `{pid}` | un `{int}` qui est le pid d'un programme **vivant** lancé par `proc/spawn` de **cette** instance du plugin | « un programme qu'il a lancé » / « a program it started » |
| `{path:temp}` | un chemin **relatif** d'un fichier de son dossier temporaire | « un fichier de son dossier temporaire » / « a file of its temporary folder » |
| `{path:plugin}` | de même, sous le dossier du plugin, sans partie cachée (comme `files/read`) | « un fichier de son dossier » / « a file of its folder » |

- **Jamais une option** : un joker ne correspond ni à `--` ni à un mot qui commence par `-`. `--` peut être un mot littéral d'une entrée.
- **L'entrée garde sa longueur** : un mot de plus ou de moins est refusé ; seul un `*` final laisse suivre n'importe quels mots, comme avant (ses jokers de tête vérifiés).
- **Un chemin est remplacé par l'hôte** : ses parties sont `[A-Za-z0-9_.-]+` entre des `/`, aucune `.` ni `..`, aucune qui commence par `-` ni qui nomme un périphérique de Windows. Donc jamais un chemin absolu, `~`, une lettre de lecteur, `\\?\`, un chemin UNC. L'hôte le joint à la racine, le résout comme `fs/*` (liens et jonctions suivis), vérifie qu'il reste sous la racine et que c'est un fichier existant, puis passe au programme **le chemin absolu résolu** (sans `\\?\` sous Windows) : le programme lit ce qui a été vérifié, et le mot ne commence jamais par `-`.
- **Le pid** : `proc/spawn` rend désormais `{ proc, pid }`. Le pid est retiré de la liste dès que l'hôte voit la fin du programme, avant d'attendre ce qu'il a laissé tourner ; il est vérifié de nouveau au moment du lancement, sous le même verrou que le lancement lui-même. Tant qu'il est dans la liste, l'hôte tient encore le programme : sous Windows son handle (le système ne redonne pas un pid tenu), sous Unix son zombie non récolté. Le pid d'un autre process, d'une autre instance ou d'un programme fini est refusé.
- **Un refus est dit au journal**, avec le mot fautif et la raison, pris de l'entrée qui allait le plus loin : `proc/spawn: refused, git var -x is not in permissions.commands: word 3 ("-x") is no {name}: an option`.
- **Un lancement est dit au journal avec les mots réels**, chemins résolus compris, secrets masqués comme avant : `proc/spawn: ran slirp4netns --configure --mtu=65520 41823 tap0 in temp`.
- **La course entre la vérification et la lecture** d'un chemin : le chemin passé est déjà résolu (aucun lien dedans) ; pour qu'il mène ailleurs au moment où le programme l'ouvre, il faudrait remplacer entre-temps une de ses parties par un lien. Le module ne crée aucun lien (`fs/write` écrit des fichiers) : seul un programme que l'utilisateur a accordé et qui crée des liens dans ce dossier le pourrait. C'est la borne ; l'hôte ne garde pas le fichier ouvert pour le programme.
- Une entrée que cet hôte ne comprend pas (un joker inconnu, un joker dans un mot, un joker en premier mot) ne correspond à rien, pas même à son texte.

### Les apps qui ne les servent pas

- `initialize` annonce `capabilities.typedCommands: true` (le banc de conformité aussi) ; le crate `escouade-plugin-protocol` le lit dans `greeting().typed_commands`.
- Une app qui ne le sert pas (2.2.4 et avant, sans la coupure d'ESC-525) compare un joker comme un mot littéral : l'entrée **ne correspond à rien** de ce qu'un plugin demande, elle n'est jamais élargie. Le plugin dit `escouade.min`.

### Ce que voit l'utilisateur

- La fenêtre d'installation et l'onglet Permissions du plugin montrent chaque commande à jokers mot à mot, chaque joker en **pastille de mots** (le tableau ci-dessus), puis, une fois, sous la dernière commande qui en a une : « Chaque pastille est un seul mot, que l'hôte vérifie : jamais une option, jamais un chemin hors de ce qu'elle dit. » (« Each pill is one word, which the host checks: never an option, never a path outside what it says. »). Le joker tel qu'écrit reste au survol de sa pastille.
- Une entrée ajoutée ou élargie par une mise à jour est un mot nouveau des permissions : l'accord est redemandé, comme pour toute permission.
- `isola`, le seul plugin de la première main qui a `permissions.commands`, n'a aucun joker : il marche comme avant.

## La limite de programmes

`limits.processes`, de 1 à 16 (code `bounds`), et `limits.reason`, un texte, exigé au-delà de 4. L'hôte garde 4 programmes à la fois tant que le ticket de la limite (T6) n'est pas fusionné : au-delà, le plugin est coupé.
