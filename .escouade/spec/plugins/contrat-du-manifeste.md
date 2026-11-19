---
title: "Le contrat du manifeste : réglages, secrets, profils, jokers et limite"
subject: "plugins"
sources: ["docs/PLUGINS-CONTRACT-STUDY.md (release/2.3, ESC-501) §2 à §6 et §10","issues GitHub #38, #39, #40, #42 (un auteur de plugin tiers)","crates/escouade-plugin-cli/src/manifest.rs","schemas/plugin-manifest.mjs","schemas/plugin-1.json","tests/fixtures/plugins/manifests.json","store/server/utils/checks.ts"]
tickets: ["ESC-525"]
decisions: ["le-contrat-du-manifeste-est-lu-avant-d-e"]
compared: null
---

# Le contrat du manifeste : réglages, secrets, profils, jokers et limite

Ce que le manifeste d'un plugin (`escouade-plugin.json`) peut dire de ses réglages, de ses secrets, de ses profils, des programmes qu'il lance et de leur nombre. Conçu par l'étude ESC-501 (`docs/PLUGINS-CONTRACT-STUDY.md`), d'après les issues #38, #39, #40 et #42 ; choix tranchés le 2026-11-17 : les profils (direction B), et tout dans la ligne 2.2.

**Ce qui est fait (ESC-525, T1)** : le contrat est **lu, validé et refusé** par les deux lecteurs, celui de l'app (`crates/escouade-plugin-cli/src/manifest.rs`) et celui du store (`schemas/plugin-manifest.mjs`, contrôle n° 1), qui passent le même corpus (`tests/fixtures/plugins/manifests.json`) et rendent le même verdict cas par cas. Le schéma `plugin-1.json` le décrit, par ajouts. **Rien n'est encore servi dans la fenêtre** : la page du plugin, les profils, les secrets d'un projet, les jokers à l'exécution et la limite viennent avec les tickets suivants (T2 à T6).

## Compatibilité

- **Un manifeste d'aujourd'hui se lit pareil.** Chaque ajout est facultatif ; absent, il vaut le comportement d'avant : `scope: app`, pas de groupe, une ligne, 4 programmes à la fois, pas de joker.
- **Jamais servi à moitié.** Un plugin qui déclare un point que l'app lit mais ne sert pas encore est **chargé coupé, la raison dite**, et la fenêtre d'installation le dit avant l'installation. Quatre points sont concernés :
  - des profils (`contributes.profiles`) ;
  - un secret de portée `project` ;
  - un joker dans `permissions.commands` ;
  - `limits.processes` au-delà de 4.

  La raison : « Ce plugin demande ce que cette version d'Escouade ne sert pas encore : … Il reste coupé plutôt que servi à moitié ; une version plus récente d'Escouade le servira. » Chaque ticket qui sert un point le retire de cette liste.
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
- La correspondance à l'exécution (ce que chaque joker admet, le chemin résolu, le pid d'un programme du plugin) est le ticket des jokers (T5) ; d'ici là, un plugin qui a un joker est coupé.

## La limite de programmes

`limits.processes`, de 1 à 16 (code `bounds`), et `limits.reason`, un texte, exigé au-delà de 4. L'hôte garde 4 programmes à la fois tant que le ticket de la limite (T6) n'est pas fusionné : au-delà, le plugin est coupé.
