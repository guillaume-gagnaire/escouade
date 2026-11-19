---
title: "Le contrat du manifeste : réglages, secrets, profils, jokers et limite"
subject: "plugins"
sources: ["docs/PLUGINS-CONTRACT-STUDY.md (release/2.3, ESC-501) §1.5, §2 à §6, §8 et §10","design/plugins-contract-2-3/captures/install.png (release/2.3)","issues GitHub #38, #39, #40, #42 (un auteur de plugin tiers)","crates/escouade-plugin-cli/src/manifest.rs","crates/escouade-plugins/src/procs.rs","crates/escouade-plugins/src/files.rs","crates/escouade-plugins/src/lib.rs","crates/escouade-plugins/src/plugins_jokers_tests.rs","schemas/plugin-manifest.mjs","schemas/plugin-1.json","schemas/plugin-protocol-1.json","src/lib/plugins.ts","src/components/plugins/AskList.svelte","tests/fixtures/plugins/manifests.json","store/server/utils/checks.ts","docs/PLUGINS-CONTRACT-STUDY.md (release/2.3, ESC-501) §2 à §6 et §10","src/components/plugins/PluginSettings.svelte","src/lib/plugin-fields.ts","src-tauri/src/plugin_secrets.rs","design/plugins-contract-2-3/captures (release/2.3) : a-plugin, a-projet, b-profil"]
tickets: ["ESC-525","ESC-528","ESC-529","ESC-526"]
decisions: ["le-contrat-du-manifeste-est-lu-avant-d-e","les-jokers-de-permissions-commands-sont","la-limite-de-programmes-d-un-plugin-est","la-page-d-un-plugin-sert-ses-groupes-ses"]
compared: null
---

# Le contrat du manifeste : réglages, secrets, profils, jokers et limite

Ce que le manifeste d'un plugin (`escouade-plugin.json`) peut dire de ses réglages, de ses secrets, de ses profils, des programmes qu'il lance et de leur nombre. Conçu par l'étude ESC-501 (`docs/PLUGINS-CONTRACT-STUDY.md`), d'après les issues #38, #39, #40 et #42 ; choix tranchés le 2026-11-17 : les profils (direction B), la liste des jokers et rien d'autre (choix n° 7 : pas de chemin dans le projet ni dans le dossier de l'agent), la limite de programmes déclarée et plafonnée à 16 (choix n° 8, pas un compte par projet), et tout dans la ligne 2.2.

**Ce qui est fait** :
- **ESC-525 (T1)** : le contrat est **lu, validé et refusé** par les deux lecteurs, celui de l'app (`crates/escouade-plugin-cli/src/manifest.rs`) et celui du store (`schemas/plugin-manifest.mjs`, contrôle n° 1), qui passent le même corpus (`tests/fixtures/plugins/manifests.json`) et rendent le même verdict cas par cas. Le schéma `plugin-1.json` le décrit, par ajouts.
- **ESC-526 (T2)** : la page du plugin sert les groupes, la description d'un secret, les champs sur plusieurs lignes et l'import d'un fichier (dernière section).
- **ESC-527 (T3)** : un secret long se garde en morceaux au trousseau.
- **ESC-528 (T5)** : les **jokers sont servis** : l'hôte les vérifie à chaque `proc/spawn`, la fenêtre d'installation et l'onglet Permissions les montrent en mots.
- **ESC-529 (T6)** : la **limite de programmes est servie** : `limits.processes` fixe le nombre de programmes à la fois, dit à l'installation au-delà de 4.
- Restent à servir : les profils et les secrets d'un projet (T4).

## Compatibilité

- **Un manifeste d'aujourd'hui se lit pareil.** Chaque ajout est facultatif ; absent, il vaut le comportement d'avant : `scope: app`, pas de groupe, une ligne, 4 programmes à la fois, pas de joker.
- **Jamais servi à moitié.** Un plugin qui déclare un point que l'app lit mais ne sert pas encore est **chargé coupé, la raison dite**, et la fenêtre d'installation le dit avant l'installation. Deux points sont encore concernés :
  - des profils (`contributes.profiles`) ;
  - un secret de portée `project`.

  La raison : « Ce plugin demande ce que cette version d'Escouade ne sert pas encore : … Il reste coupé plutôt que servi à moitié ; une version plus récente d'Escouade le servira. » Chaque ticket qui sert un point le retire de cette liste. Les jokers en sont sortis avec ESC-528, `limits.processes` au-delà de 4 avec ESC-529.
- `escouade-plugin validate` dit ces points par un avertissement (⚠), sans échouer : un plugin écrit pour l'Escouade suivante se valide.
- Ce qui se lit sans couper le plugin : `groups`, `description`, `multiline`, `lines`, `import` (servis par la page du plugin depuis ESC-526), `scope: project` sur un réglage simple, `repo`. `repo: false` est déjà tenu : la valeur de `.escouade.yml` n'est jamais prise pour ce réglage, ni pour un réglage qui a `import`.

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

### Au manifeste

- `limits.processes` : un nombre entier de **1 à 16** (code `bounds` au-delà, par les deux lecteurs). Absent, il vaut **4**, comme avant.
- `limits.reason` : un texte, ou un texte par langue ; **exigé au-delà de 4** (code `required`). Il dit pourquoi le plugin lance autant de programmes ; la revue du store le juge.

### Ce que l'hôte tient

- Le nombre de programmes vivants d'une instance (`proc/spawn`, `procs::Procs`) ne dépasse jamais la limite du plugin : `limits.processes`, ou 4 sans lui. **L'hôte ne donne jamais plus de 16**, quoi que dise le manifeste (`procs::at_once` ramène tout nombre au-delà à 16 et 0 à 1), même si un manifeste refusé par le lecteur lui parvenait.
- La limite est par instance, et une instance par plugin : elle vaut pour le plugin, tous projets confondus.
- **Une place est rendue** dès que l'hôte voit la fin d'un programme (fini ou tué par `proc/kill`) ; le suivant peut partir.
- **Le refus dit la limite du plugin**, dans la réponse (`-32003`) et au journal : `proc/spawn: refused, 8 programs at once at most for this plugin (limits.processes)` (« 1 program » au singulier). Le message commence toujours par « N programs at once at most », comme avant.
- **Ce que l'hôte garde par programme**, borné : trois tuyaux, quatre tâches (l'entrée, la sortie, l'erreur, la fin), un crédit de 256 Kio non lus par le module partagé entre la sortie et l'erreur, au plus 1 Mio en attente d'entrée, deux tampons de lecture de 16 Kio, un Job object. Soit environ 1,3 Mio de l'app au pire par programme, une vingtaine de Mio à 16, plus ce que prennent les programmes eux-mêmes ; rien ne grandit sans borne.

### Ce que voit l'utilisateur

- **À 4 ou moins** (ou sans `limits.processes`) : rien de plus qu'avant, nulle part.
- **Au-delà de 4**, et seulement pour un plugin qui a un programme et des `permissions.commands` : la fenêtre d'installation et l'onglet Permissions ont une ligne en gras, juste après les commandes : « Jusqu'à N programmes à la fois » (« Up to N programs at once »), et dessous « 4 sans le demander. L'auteur dit pourquoi : « raison ». » (« 4 without asking. The author says why: “reason”. »), la raison dans la langue de l'interface. Sans raison lisible : « 4 sans le demander. » seul.
- C'est une **permission** comme les autres, le mot `processes:<N>` des permissions du plugin (`plugins::asks`) :
  - **une mise à jour qui l'augmente redemande l'accord** (au-delà de 4, ou au-delà de ce qui était accordé : 8 → 12), la ligne marquée « nouveau » ; **une baisse ne demande rien** (12 → 8, ou retour à 4). « Tout mettre à jour » ne passe donc pas une hausse sans demander ;
  - **elle se retire** dans l'onglet Permissions : le plugin repart avec les 4 qu'il a sans le demander. Un retrait vaut pour la limite retirée : une mise à jour qui l'augmente et que l'utilisateur accepte rend la nouvelle limite ;
  - un plugin « En développement » suit la même règle que pour ses autres permissions.

### Les apps qui ne la servent pas

- Une app d'avant ESC-525 ignore `limits.processes` et garde 4 ; une app d'ESC-525 à ESC-528 charge coupé un plugin qui demande plus de 4. Le plugin dit `escouade.min`.
- Les plugins de la première main n'ont pas `limits.processes` : `isola` garde ses 4 programmes à la fois, et rien ne change pour eux.

## La page d'un plugin : groupes, secrets décrits, plusieurs lignes, import (ESC-526, T2)

L'onglet « Réglages » de la page d'un plugin sert `groups`, `description` d'un secret, `multiline`, `lines` et `import` (issue #40, étude §4.1 et §4.3, choix n° 6 : l'import du contenu, jamais un chemin).

- **Les groupes d'abord**, dans l'ordre de la liste : un titre, sa description dessous, ses rangées séparées par un filet, sans boîte. Réglages et secrets y sont **mêlés dans l'ordre de `fields`**. Un groupe `folded` est replié d'office, comme « Avancé » ailleurs dans les réglages : une rangée calme « › Avancé · 3 réglages », un bouton qui s'ouvre et se ferme au clavier (`aria-expanded`). Ce que l'utilisateur ouvre reste ouvert tant que la page est là.
- **Un champ hors de tout groupe vient après**, sous « Réglages » puis « Secrets », comme avant. **Un plugin sans groupes est rendu comme avant**, au pixel près. Quand tous les secrets sont dans des groupes, la note du trousseau se lit sous les groupes.
- **La description d'un secret** se lit sous son titre, avant son état.
- **Un champ sur plusieurs lignes** (`multiline`, un réglage `string` ou un secret) : une zone de texte à chasse fixe, ses lignes numérotées dans une gouttière qui défile avec elle, `lines` lignes visibles (3 à 20, 6 d'office), sans retour à la ligne automatique. **64 Kio au plus** : au-delà, « Plus de 64 Kio : rien n'a été pris, rien n'est coupé. », et rien n'est gardé ni tronqué. L'app refuse aussi un secret de plus de 64 Kio de son côté (la limite d'ESC-527).
- **Un secret sur plusieurs lignes ne se relit pas.** Gardé, sa rangée dit « Gardé · 84 lignes · 5,1 Ko » (« Gardé dans un fichier · … » s'il est dans le fichier lisible de l'utilisateur seul) et propose « Oublier » et « Remplacer… ». L'app ne rend à la fenêtre que son nombre de lignes et d'octets, jamais son texte. « Remplacer… » ouvre une zone vide, « Annuler » y revient ; pendant la saisie, le texte se voit.
- **« Importer un fichier… »** (un champ qui a `import`) ouvre la boîte de dialogue du système, filtrée sur ces extensions. Le fichier choisi est lu **une fois**, dans la fenêtre :
  - son nom doit finir par une des extensions (le filtre de la boîte n'est qu'une aide) ;
  - 64 Kio au plus ;
  - du texte en UTF-8, sans caractère nul.

  Sinon un message dit pourquoi et rien n'est pris. Son texte remplit le champ : un secret attend « Garder », un réglage est gardé aussitôt. **Le chemin n'est jamais connu de l'app** : ni dans l'état, ni dans les réglages, ni dans le journal du plugin ou de l'app. Son nom non plus. Le plugin ne gagne **aucun droit** sur les fichiers.
- **Une configuration de plus de 1 280 caractères** (6 Ko avec ses certificats) se garde aussi sous Windows : le trousseau la reçoit en morceaux (ESC-527). Un trousseau qui refuse quand même fait dire la raison à « Garder » (le message d'ESC-499), et le fichier lisible de l'utilisateur seul est proposé.
- Tout tient à 1024 × 728, en français et en anglais. L'e2e `plugin-groups.spec.ts` le vérifie avec un plugin de test.
- **Le journal du plugin** masque un secret entier, comme avant. Pour un secret sur plusieurs lignes, il masque aussi chacune de ses lignes de plus de 8 caractères (étude §4.4, un seul masquage partagé avec ESC-527). Un plugin qui écrit la configuration ligne par ligne n'en montre donc rien de lisible ; une ligne courte comme `client` reste, sinon elle masquerait tout.
