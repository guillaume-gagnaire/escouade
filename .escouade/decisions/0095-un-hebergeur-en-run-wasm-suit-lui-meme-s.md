---
number: 95
id: "un-hebergeur-en-run-wasm-suit-lui-meme-s"
title: "Un hébergeur en run.wasm suit lui-même ses redirections, et lit 8 Mio au plus"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-345"
pages: ["plugins/tous-les-plugins-en-wasm","git-fichiers/hebergeurs-de-code","plugins/livres-avec-escouade"]
tickets: ["ESC-345"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":12,"removed":0},{"page":"git-fichiers/hebergeurs-de-code","added":5,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":0}]
accepted_at: "2026-11-06T08:03:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Porter GitHub, GitLab, Bitbucket, Gitea et Azure DevOps en WASM (ESC-345) retire reqwest : l'hôte rend les 3xx au module pour juger chaque adresse (`net/fetch`), et la façade du kit ne les suit pas, alors que dépôts renommés, instances passées en https et journaux de GitHub Actions reposaient dessus. Un process n'avait pas non plus de limite de taille, et un module a 64 Mio de mémoire. Le kit publié sur crates.io ne peut changer sans une publication. À la relecture : reqwest renvoyait le corps d'un 307 ou d'un 308 vers un autre hôte, et `*.blob.core.windows.net`, désormais joignable, est ouvert à tout compte Azure.

## Décision

Chaque hébergeur porte son `src/redirect.rs`, qui fait ce que faisait reqwest, à une différence près : dix sauts, un GET sans corps après 301, 302 et 303, le même appel après 307 et 308, ses identifiants gardés sur le même hôte et jamais de https vers http. Un corps ne quitte pas le service : si un saut n'est plus sur cet hôte, ou passe de https à http, sauts d'avant compris, et que l'appel a encore son corps, quel que soit le 3xx, `follow` s'arrête et rend ce 3xx comme dernière réponse, sans rien envoyer à l'autre hôte ; `success` en fait une erreur qui dit le statut. Un appel sans corps, ou dont un 301, 302 ou 303 sur le service a retiré le corps, suit toujours, sans les identifiants. GitHub déclare le stockage de ses journaux (`*.actions.githubusercontent.com`, `*.blob.core.windows.net`), joint sans jeton. Une Enterprise Server dont le stockage est ailleurs ne lit pas ce journal : un hôte d'instance ne vient que du compte. Chaque hébergeur déclare `fetchSeconds: 15`, le délai de son process, et `fetchBytes: 8388608`. Azure DevOps remplace `futures-util` par un `join_all` local : ses fichiers font dépasser à `rustc-canon` la longueur d'une ligne de commande sous Windows.

## Conséquences

Mêmes réponses et mêmes requêtes que le natif sur tout `forge/*`, à deux détails près : plus de `Referer` ajouté en suivant, et un saut compte dans le budget de Bitbucket. Une écriture redirigée hors du service, que le natif renvoyait, finit en erreur avec son statut ; la parité ne le voit pas, son écriture redirigée reste sur le même hôte. Un journal de plus de 8 Mio, que le process lisait, se dit trop long. `*.blob.core.windows.net` s'ajoute aux droits de GitHub dans la fenêtre d'installation. Cinq copies d'un même fichier : à remonter dans le kit à sa prochaine publication, qui demande un `plugin-lock.sh` par plugin.
