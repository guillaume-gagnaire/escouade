---
title: "Le .escouade.yml d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §6.1, §6.2, §6.5, §6.6, §15.2 (C12-B), §16 (TS-13)","crates/escouade-team/src/settings.rs","src-tauri/src/team_settings.rs, src-tauri/src/teams_settings.rs"]
tickets: ["ESC-471","ESC-480"]
decisions: [183,186]
compared: null
---

# Le .escouade.yml d'équipe

Les réglages qu'une équipe recommande et impose à ses membres. Un seul fichier par équipe, **signé** par ses administrateurs, **chiffré** par la clé d'équipe, **jamais dans un dépôt** : il vit sur le serveur d'équipe, que ce dernier ne lit pas.

## Le format

```yaml
version: 1
defaults:              # rang 2 : recommandé ; le dépôt et le poste le changent
  board:
    merge_strategy: squash
enforced:              # rang 5 : imposé
  harnesses.allowed: [claude, codex]          # une borne
  permissions:
    modes: [plan, default, acceptEdits]
  board.on_approve: pr
  telemetry.enabled: false
  proxy.exclusions: ["*.acme.fr"]             # sensible : attend l'accord de chaque poste
proposed:              # ce qui s'exécute : proposé, relu et accepté sur chaque poste
  worktrees:
    setup: [npm ci]
reasons:
  harnesses.allowed: "Seuls les fournisseurs sous contrat DPA."
quorum: true           # facultatif : par défaut, allumé dès qu'il y a deux administrateurs
signers: []            # facultatif : les membres qui signent ; vide, tout administrateur
```

- Les clés sont celles de la **liste unique des stratégies** (`permissions.modes`, `harnesses.allowed`, `telemetry.enabled`, `proxy.url`…) et celles du `.escouade.yml` du dépôt (`board.on_approve`, `git.agent_branch_prefix`, `worktrees.setup`, `launch`…), plus `plugins.install`. On les écrit pointées (`permissions.modes:`) ou en groupes (`permissions:` puis `modes:`).
- Une clé inconnue de cette version d'Escouade est gardée (ce qui n'entre jamais y est cherché), dite, et ne s'applique pas.
- **Un fichier par équipe.** Un projet d'équipe n'a pas encore le sien par-dessus : il viendra avec les groupes des projets gardés (TS-5) et leur clé, signé de la même façon.

## Signé, chaîné, jamais en recul

- Chaque **version** porte son numéro, l'empreinte de la précédente, le nom de son auteur (« Claire Martin », tel qu'il le donne) et l'heure. Elle est scellée sous la clé d'équipe de son époque et **signée par la feuille MLS de l'appareil** d'un administrateur. Elle va dans un flux du serveur à part, qui n'y voit que des octets.
- Une **cosignature** nomme l'empreinte d'une version : l'accord d'un second administrateur.
- Un appareil refuse une version :
  - **non signée**, ou forgée par le serveur (aucune feuille de l'équipe ne l'a signée) ;
  - **signée par qui n'est pas administrateur** à l'époque où elle a été scellée (quand l'appareil a vécu cette époque), ou qui ne l'est plus aujourd'hui (**un administrateur retiré depuis**), ou qui n'est pas dans `signers` ;
  - **rejouée** (une version déjà vue, servie de nouveau) ;
  - **qui recule** (un numéro déjà pris, une version qui cite une version plus récente) ;
  - qui ne suit pas la version en vigueur, ou dont le numéro est pris par une autre passée avant.
- L'appareil garde la chaîne qu'il a choisie et ne la défait jamais. Les administrateurs encore là **cosignent d'eux-mêmes** les versions de la chaîne dont le signataire est parti : un appareil qui arrive après lit toute la chaîne.
- **Un serveur menteur** : l'appareil relit la dernière entrée lue avant de lire plus loin. Disparue (le serveur sert une vieille version), c'est un retour en arrière ; changée, une entrée réécrite ; une numérotation qui recule, une anomalie. La lecture s'arrête, l'écran le dit, « Réessayer » relit. Ce qui s'appliquait reste en vigueur.
- Limite : un serveur qui **cache** une version à un appareil qui ne l'a jamais vue ne se voit pas de cet appareil seul.

## Le quorum (C12-B)

- Une version qui change une **clé sensible**, la **liste des signataires** ou le **quorum lui-même** ne vaut qu'avec **deux administrateurs distincts** : deux appareils d'un même administrateur comptent pour un. En attendant, elle est **proposée** : rien n'en s'applique, l'historique dit « En attente d'une seconde signature (1/2) », un autre administrateur la cosigne.
- Le reste (restreindre, une valeur, une commande proposée) passe avec une signature.
- Le quorum est une **option de l'équipe** (`quorum`), **allumée d'office dès qu'il y a deux administrateurs**. Le quorum en vigueur est celui de la version en vigueur.
- **Un seul administrateur** : le quorum ne s'applique pas, une signature suffit pour tout, et l'écran dit « Un seul administrateur : le quorum ne s'applique pas. Nomme un second administrateur pour que deux signatures soient demandées. » Une équipe qui a écrit `quorum: true` avec un seul administrateur ne fait plus passer de clé sensible tant qu'un second n'est pas nommé.

## Les sortes, tenues par le cœur

- **Restreindre** (couper une fonction, réduire une liste permise) : appliqué tout de suite.
- **Valeur** (une branche cible, un préfixe) : appliqué et montré.
- **Sensible** (`proxy.url`, `proxy.exclusions`, `proxy.authorities`) : **en attente de l'accord de chaque membre**, sur son poste, avec ce que cela permet, dit en clair (« Ce certificat permet de lire le trafic TLS d'Escouade »). L'accord porte sur l'empreinte de la valeur : une valeur changée redemande l'accord. Il se retire (« Retirer mon accord »).
- **Exécutable** (commandes, hooks, `launch`, tests, installation d'un plugin, serveur MCP, variables d'environnement des agents) : **jamais imposé**. Seulement dans `proposed`, relu et accepté sur chaque poste à son empreinte ; un changement redemande l'accord. Accepté, il s'applique comme une valeur recommandée (rang 2) aux projets de l'équipe.

## Ce qui n'entre jamais, même signé

Une liste écrite dans l'app (`escouade-team::settings::never`), que l'équipe ne change pas. Une version qui en contient une ligne est **refusée entière, à la lecture** (et l'écran de l'administrateur refuse de la signer, en disant pourquoi) :

- `bypassPermissions`, nommé n'importe où, ou permis (`permissions.allowBypass: true`) ;
- une commande qui tournerait sans accord : une clé qui s'exécute hors de `proposed` (`worktrees.setup`, `launch`, `hooks.*`, `agents.env.*`…), ou une proposition marquée acceptée d'avance (`trusted`, `approved`, `autoRun`…) ;
- un plugin installé sans le geste du membre (`plugins.install`, `plugins.required` hors de `proposed`) ;
- un `dev.escouade.*` signé par l'équipe (un espace de noms `dev.escouade…`, ou un `dev.escouade.*` du store de l'équipe à installer) ;
- un projet rendu public par le fichier (`private: false`, `projects.forcePrivate: false`, `….public: true`).

## Où il s'applique

- `enforced` alimente le **rang 5** du verrou, `defaults` le **rang 2** (page « Réglages imposés »). Une borne se croise avec celle du poste géré (il reste ce que les deux permettent), une valeur du poste géré gagne.
- **Une stratégie d'équipe ne vaut que pour les projets de l'équipe** : les modes, les harness, les modèles, l'effort, la clôture et le Remote Control d'un projet, ainsi que les clés du `.escouade.yml` d'un projet (rang 2 sous le fichier du dépôt et le poste, rang 5 par-dessus, refusé quand on le change). Un projet perso n'en reçoit rien.
- Les clés **de l'app** (le réseau, la télémétrie, la voix, les mises à jour, les plugins, les clients MCP extérieurs) n'ont pas de projet : tant que les espaces n'existent pas (TS-4), elles valent pour l'app pendant que le poste est dans l'équipe.
- **Projets de l'équipe sur ce poste** : un projet dont un dépôt est dans la liste des projets de l'équipe se rattache dans « Projets de l'équipe » (page « Les projets de l'équipe »), reconnu à l'adresse de son dépôt ; l'écran des réglages de l'équipe le dit. Pour les autres projets, une case par projet. La rattache par le dépôt (`team:` du `.escouade.yml`) viendra avec TS-7.

## Ce que voit le membre

L'écran est dans la fenêtre des équipes, bouton « Réglages de l'équipe » de chaque équipe (Réglages › Équipe est TS-4).

- **La bande**, sur toute la largeur de la fenêtre : « L'équipe Acme a changé 2 réglages · Claire Martin, il y a 5 min · Voir », tant que le membre n'a pas vu la version en vigueur ; et « Acme demande ton accord pour 1 réglage » tant qu'une clé sensible ou une commande proposée attend. « Voir » ouvre l'écran et marque la version vue.
- **La version en vigueur** : son numéro, qui l'a signée et quand, ses signatures, ce que l'auteur dit du changement, la règle du quorum.
- **En attente de ton accord** : chaque clé sensible avec sa valeur et ce qu'elle permet ; chaque commande proposée avec son texte et son empreinte ; « Accepter sur ce poste », « J'ai relu, accepter ».
- **Ce que l'équipe impose**, avec la raison écrite par l'administrateur ; **ce qu'elle recommande** ; **les commandes acceptées sur ce poste**.
- **L'historique** : chaque version, son auteur, son état (En vigueur, Remplacée, En attente d'une seconde signature, Refusée : la raison), et son changement (le diff, clé par clé, avec l'ancienne et la nouvelle valeur). Le nombre d'entrées refusées à la lecture.
- Dans les réglages d'un projet de l'équipe, un choix interdit est grisé avec qui l'impose : « Auto · non permis par l'équipe Acme ».

## Ce que fait l'administrateur

- **Proposer une version** : le fichier en YAML, prérempli de la version en vigueur ; « Relire le changement » montre le diff, ce qui ne s'appliquera pas, et si la version attendra une seconde signature ; « Signer et publier » la signe par la clé de cet appareil, sous le nom qu'il donne (gardé pour la fois suivante).
- **Cosigner** la version proposée d'un autre administrateur, depuis l'historique.
- L'éditeur confortable, clé par clé, viendra avec TS-4.
