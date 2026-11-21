---
title: "Les profils d'un plugin et ses réglages d'un projet"
subject: "plugins"
sources: ["docs/PLUGINS-CONTRACT-STUDY.md (release/2.3, ESC-501) §1.1 à §1.3, §2, §3, §6 à §8 et §10 (choix n° 1 à 4 tranchés le 2026-11-17)","design/plugins-contract-2-3/captures (release/2.3) : b-plugin, b-profil, b-projet","issues GitHub #38 et #39 (un auteur de plugin tiers)","crates/escouade-plugins/src/profiles.rs","crates/escouade-plugins/src/runtime.rs","src-tauri/src/plugin_profiles.rs","src-tauri/src/plugin_secrets.rs","src-tauri/src/projectfile.rs","src/lib/plugin-profiles.ts","src/components/plugins/PluginProfiles.svelte","src/components/plugins/ProfileModal.svelte","src/components/settings/ProjectPluginsPage.svelte","schemas/plugin-protocol-1.json","crates/escouade-plugin-kit/src/serve.rs","sdk/plugin/src/index.ts","sdk/plugin/src/testing.ts","crates/escouade-plugins/src/bench/fake.rs","website/content/docs/fr/plugins/project-credentials.md"]
tickets: ["ESC-530","ESC-531"]
decisions: [255]
compared: null
---

# Les profils d'un plugin et ses réglages d'un projet

Ce qu'un plugin garde **pour un projet** : un **profil** (un serveur, ses identifiants, sa configuration) que l'utilisateur crée autant de fois qu'il veut et que **chaque projet choisit**, et des **réglages de portée `project`** que chaque projet règle pour lui. Issues #38 et #39 d'un auteur de plugin tiers ; étude ESC-501, direction B (les profils) choisie le 2026-11-17 avec les choix n° 2 à 4 ; servi par ESC-530. La déclaration dans le manifeste est sur *Le contrat du manifeste*.

## Les rangs d'un réglage

`settings/get` sans projet rend les réglages de l'utilisateur, comme avant. Avec un `project` :

- **Un réglage de portée `app`** (d'office) : la valeur de l'utilisateur (page du plugin), puis par-dessus celle du `.escouade.yml` du projet (`plugins.<id>.<clé>`), comme avant.
- **Un réglage simple de portée `project`** (`string`, `number`, `boolean`, `select`), du plus fort au plus faible :
  1. la **valeur de cette machine pour ce projet** (rang 4), gardée avec la liste des plugins installés (`projectSettings`, par projet) ;
  2. celle du **`.escouade.yml`** du projet (rang 3), sauf si l'auteur l'a écartée (`"repo": false`) ou que le réglage a `import` ;
  3. le **`default`** du manifeste (rang 1).
- **Un réglage `type: "profile"` de portée `project`** : l'identifiant du profil choisi pour ce projet ; `null` pour « Aucun profil ». Un projet qui n'a rien choisi prend le profil d'un réglage `profile` de portée `app` du même genre (« Profil par défaut », sur la page du plugin), sinon `null`. Un profil supprimé vaut `null`. **Jamais la valeur du fichier.**
- Le plugin n'a ni le fichier ni la valeur de cette machine pour un projet où il n'est **pas actif** (coupé pour ce projet, ou un projet privé où il n'a pas été allumé) : seulement le défaut, et `null` pour un profil.
- Une valeur de la page du plugin gardée avant qu'un réglage devienne de portée `project` n'est pas celle d'un projet : elle est ignorée.
- Les rangs 2 (recommandé par l'équipe), 5 (imposé par l'équipe) et 6 (poste géré) viennent avec le serveur d'équipe de la 2.3 (ESC-532).

## Les profils, sur la page du plugin

L'onglet « Réglages » de la page du plugin commence par un groupe par genre de profil (`contributes.profiles`) :

- Son titre, le nombre de profils, « Ajouter un profil… » à droite ; sa description (celle du genre, sinon « Un profil réunit des réglages et des secrets, que tu crées autant de fois que tu veux. Chaque projet choisit le sien dans ses réglages. »).
- Une rangée par profil, dans l'ordre où ils ont été créés : son nom (`nameField`), dessous les valeurs de ses trois premiers autres champs simples (« vpn.client-a.example · UDP · 1194 »), les projets qui l'ont choisi (leur couleur et leur nom ; « Aucun projet »), « 2 secrets gardés » (« Aucun secret gardé »), « Modifier ». Sans profil : « Aucun profil encore : « Ajouter un profil… » en crée un. »
- Les réglages de portée `project` ne sont **pas** sur la page du plugin ; un réglage `profile` de portée `app` y est un menu des profils, « Aucun profil » en tête.

### La fenêtre d'un profil

- Titre « Nouveau profil · Profils VPN », ou « Profil « Client A » ». Ses champs simples d'abord, sous le titre du genre (le champ du nom dit « Son nom dans la liste et dans les réglages des projets. »), puis « Secrets », avec la note « Gardés dans le trousseau du système sous un nom propre à ce profil, jamais dans .escouade.yml ; le plugin les lit pour les projets qui le choisissent. ». Les champs ont les formes d'ESC-526 : plusieurs lignes numérotées, « Importer un fichier… » (le contenu, jamais le chemin), 64 Kio au plus.
- **Un secret gardé ne se relit pas** : « Gardé dans le trousseau » (ou « dans un fichier lisible de toi seul », ou « Gardé · 84 lignes · 5,1 Ko » sur plusieurs lignes), « Oublier », « Remplacer… » (« Remplacer par un fichier… » quand il importe). Un secret tapé est gardé à « Enregistrer ».
- **« Enregistrer »** : grisé tant que le profil n'a pas de nom. Un nouveau profil reçoit son identifiant d'Escouade, puis ses secrets sont gardés sous cet identifiant. Un secret refusé (trousseau, taille) le dit sous son champ, et la fenêtre reste ouverte.
- **« Supprimer le profil »** (un profil existant), en bas à gauche ; à côté « Utilisé par atlas-web » (« Utilisé par aucun projet »). Il demande sur place : « Supprimer « Client A » ? Ses secrets sont effacés. atlas-web n'aura plus de profil. » avec « Annuler » et « Supprimer ». Supprimé : ses secrets sont effacés du trousseau (ou du fichier), les projets qui le prenaient passent à « Aucun profil » (`null`, pas au profil par défaut), un « Profil par défaut » qui le nommait est oublié.

## La page Plugins d'un projet

Réglages › Projets › *projet* › **Plugins**, entre « Tickets importés » et « Équipe » (voir *Réglages*).

- Elle n'est dans la barre que si un plugin **installé, en état de tourner et actif dans ce projet** déclare un réglage de portée `project` (règle du 2026-11-11 : aucune offre « Non installé »). Un plugin coupé pour ce projet n'y est pas ; un projet privé, seulement pour un plugin qui y a été allumé. Choisir dans la barre un projet qui ne l'a pas ouvre sa page « Projet ».
- Titre « Plugins », description « Ce que les plugins actifs dans ce projet te demandent pour lui. Les autres réglages d'un plugin sont dans sa page. ».
- Un groupe par plugin : sa tuile, son nom, son identifiant, « Page du plugin » (ferme la fenêtre et ouvre sa page dans la vue Plugins, sur « Réglages »). Dedans, ses réglages de portée `project` dans l'ordre du manifeste.
- **Un profil** : un menu « Par défaut : Client A » (s'il y a un profil par défaut), « Aucun profil », puis chaque profil (« Client A · vpn.client-a.example ») ; sous le titre, « Gardé sur cette machine, jamais dans .escouade.yml. ». Sans profil pour ce projet : « Aucun profil : Tunnel n'en a pas pour ce projet. ». Sans aucun profil créé : « Aucun profil encore : crée-le sur la page du plugin. ».
- **Un réglage simple** porte les marques des autres réglages d'un projet (*.escouade.yml*, « Dans la fenêtre ») :
  - « Défini par .escouade.yml » quand la valeur vient du fichier ;
  - réglé ici alors que le fichier en donne une : « Réglage local de cette machine : il passe devant .escouade.yml. », avec « Écrire dans .escouade.yml » et « Reprendre la valeur du fichier » ;
  - réglé ici sans valeur dans le fichier : « Écrire dans .escouade.yml » seul ;
  - un réglage `"repo": false` ou qui a `import` : « Gardé sur cette machine, jamais dans .escouade.yml. », sans « Écrire dans .escouade.yml ».
  - Vider un champ texte ou nombre reprend la valeur du fichier, ou le défaut.
- **« Écrire dans .escouade.yml »** écrit **cette clé seule**, `plugins.<id>.<clé>`, par-dessus le fichier en gardant ses commentaires, son ordre et ses clés inconnues (un nouveau fichier sinon), puis oublie la valeur de cette machine : le projet suit de nouveau le fichier. Jamais un secret, un profil, une valeur importée ni un réglage `"repo": false` : l'app le refuse. « Écrit dans .escouade.yml. »
- **Un fichier qui propose un profil** (`plugins.<id>.<réglage profile>: <nom>`) : une bande ambre en tête de la page, « .escouade.yml propose le profil « client-a » pour Tunnel. Un dépôt ne choisit pas tes identifiants : c'est toi qui choisis, ici, sur cette machine. ». Un profil de ce nom (casse et espaces des bouts ignorés) : « Prendre « Client A » » ; aucun : « Aucun de tes profils ne porte ce nom : crée-le sur la page du plugin, ou choisis-en un. ». La bande disparaît quand le profil de ce nom est celui du projet.
- Dans l'onglet « Projets » de la page d'un plugin qui a des réglages de portée `project`, chaque projet où il est actif a un lien « Réglages du projet » qui ouvre cette page.
- Tout tient à 1024 × 728, en français et en anglais.

## Le protocole (version 1, par ajouts)

- `initialize` annonce `capabilities.profiles: true` ; le crate `escouade-plugin-protocol` le lit dans `greeting().profiles`.
- `settings/get { project }` : les rangs ci-dessus ; un réglage `profile` rend l'identifiant du profil, ou `null`.
- `profiles/list { kind }` : `[{ id, name }]`, dans l'ordre de création ; un genre non déclaré : `-32602`.
- `profiles/get { kind, profile }` : les champs simples du profil (jamais un secret), `null` s'il n'en a pas de cet identifiant.
- `secrets/get { key, profile }` : le champ secret `key` du profil. Refusé `-32602` pour un identifiant qui n'en est pas un ou qu'il n'a pas, et pour une clé qui n'est pas un champ secret de son genre ; `-32001` sans `profile:<genre>` dans ses permissions (retirée par l'utilisateur comprise).
- `secrets/get { key }` répond comme avant. **Une clé qui n'est qu'un champ secret d'un profil, demandée sans `profile`, est refusée (`-32602`)** : une confusion ne rend jamais un autre secret.
- Le kit (`escouade-plugin-kit` 0.1.4, avec le protocole 0.1.4 ; 0.1.3 n'a jamais été publié) : `settings(project)`, `profile_of(project, setting)`, `secret_of(profile, key)`, `profiles(kind)`, `profile(kind, id)`. Le SDK (`@escouade-dev/plugin` 0.5.0) : `p.settings(project)`, `p.profiles.of`, `p.profiles.list`, `p.profiles.get`, `p.secret(key, { profile })` (ESC-531).
- **Une app qui ne sert pas les profils** ignorerait `profile` et rendrait un autre secret : le kit (`Cx::require`) et le SDK (`p.require`) vérifient `capabilities.profiles` reçu à `initialize` et refusent l'appel avant de l'envoyer (`-32601`), en disant la version qui les sert (`ESCOUADE_MIN`, 2.2.5). Le plugin dit `escouade.min` ; `escouade-plugin validate` avertit (⚠) un `min` plus ancien.

## La sécurité (étude §2.3)

- **Le nom au trousseau** d'un secret de profil est `plugin:<id>:profile:<genre>:<profil>:<clé>`. `<profil>` est un identifiant **tiré par Escouade** à la création (16 chiffres hexadécimaux), jamais un texte du plugin ni du dépôt ; un `profile` demandé qui n'a pas cette forme est refusé avant toute lecture. Sans trousseau, le fichier choisi d'ESC-499, sous le même nom ; un secret long passe en morceaux (ESC-527).
- **Le plugin seul** lit ses secrets : sous son préfixe, d'un profil qu'il a, de son genre et avec sa permission.
- **Un projet** ne donne son choix de profil (et ses valeurs) qu'où le plugin est **actif**. Une instance sert tous les projets : que le plugin n'utilise le secret de atlas-web que pour atlas-web, c'est son code, que la revue lit.
- **`.escouade.yml`** ne porte jamais un secret (une clé secrète du fichier est ignorée) et **ne choisit jamais un profil** : sa valeur pour un réglage `profile` n'est qu'une proposition par un nom, dite dans la bande.
- **Le journal** du plugin dit `was given its secret "password" of profile "Client A"`, jamais la valeur (masquée, ligne par ligne sur plusieurs lignes).
- **Les noms gardés** : Escouade tient, avec la liste des plugins installés, **la liste des noms** qu'il a gardés au trousseau pour un plugin (`secretNames` : les noms, jamais les valeurs), puisque le trousseau ne liste pas par préfixe. Désinstaller le plugin (sans garder ses données) efface ses secrets déclarés et chaque nom de cette liste qui est sous son propre préfixe `plugin:<id>:`. Supprimer un profil efface ses noms.

## Compatibilité

- **Un plugin d'aujourd'hui marche pareil** : sans `scope`, un réglage est de portée `app` ; `secrets/get { key }` répond comme avant ; `settings/get` sans projet ne change pas.
- Les profils et un réglage de portée `project` ne coupent plus un plugin. **Un secret de portée `project`** (direction A de l'étude, non retenue) reste lu et coupe toujours le plugin (*Le contrat du manifeste*).
- L'export de tout le projet (« Exporter vers .escouade.yml ») n'écrit toujours pas `plugins.*`.

## Pour l'auteur (ESC-531)

- **Le faux Escouade du SDK**, donné le manifeste, sert les profils (`profiles`), les rangs d'un réglage de projet (`projectValues`, `projectSettings`, `off`) et les refus d'un secret de profil comme l'app.
- **Le banc de conformité** sert un profil du banc par genre, choisi par le projet du banc (`bench-project`), et juge le scénario « Profils » : un refus d'Escouade (`secrets/get`, `profiles/*`, `settings/get`) ou un secret de profil écrit sur la sortie d'erreur ou dans un `log` le fait échouer.
- **La documentation du site** (fr et en) : la page `plugins/project-credentials` (« Des identifiants par projet ») suit un tunnel par projet de bout en bout ; `plugins/manifest`, `plugins/calls`, `plugins/api`, `plugins/sdk`, `plugins/rust`, `plugins/testing`, `plugins/versioning` et `escouade-yml` disent les profils, la portée projet, ce que `.escouade.yml` peut et ne peut pas.

## Limites

- Les réglages recommandés ou imposés par une équipe (rangs 2 et 5) et par un poste géré (rang 6) arrivent avec la 2.3 (TS-13, ESC-532).
- Un profil n'a pas de groupes à lui : ses champs simples, puis ses secrets.
