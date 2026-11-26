---
title: "Fetch automatique"
subject: "git-fichiers"
sources: ["Ticket ESC-561 (demandes de l'utilisateur du 2026-11-20 : « ajoute la possibilité d'activer le git fetch automatique », « pouvoir configurer le délai »)","~/.escouade/app.log de l'utilisateur (échecs du fetch de fond, 2026-09-29 au 2026-11-13)","git-for-windows/git#5199 (ssh d'OpenSSH 9.9p1 qui cale, corrigé dans 2.47.0(2))","src-tauri/src/core.rs (fetch_all, fetch_root, fetch_plans)","src-tauri/src/git.rs (fetch_background, windows_ssh_command, fetch_failure_kind)","crates/escouade-model/src/model.rs (GitFetchSettings, ProjectFetch, FetchPlan, FetchFailure)","src/components/settings/FetchSettings.svelte","src/components/settings/GitPage.svelte","src/components/branches/FetchLine.svelte","website/content/docs/fr/branches.md","website/content/docs/fr/settings-projects.md"]
tickets: ["ESC-561","ESC-597"]
decisions: [293,338]
compared: null
---

# Fetch automatique

## Ce que c'est

En arrière-plan, Escouade récupère (`git fetch`) chaque dépôt de ses projets qui a un dépôt distant, pour que les commits à tirer se voient d'eux-mêmes (barre d'état, sélecteur de branche). Il se règle pour tous les projets et pour chacun, se voit dans le panneau Git, et ne demande jamais d'identifiants. Demandé le 2026-11-20 (ESC-561) : il existait déjà, sans réglage ni rien qui se voie, et échouait chez l'utilisateur sur le dépôt d'Escouade.

## Comportement

### Quand il tourne

- 15 s après le démarrage, puis Escouade regarde toutes les 15 s quels dépôts sont dus : un dépôt l'est quand son intervalle est passé depuis son dernier fetch de fond, ou qu'il n'en a jamais eu. Les réglages sont relus à chaque fois : un changement vaut sans redémarrer.
- Un dépôt à la fois, sans jamais demander d'identifiants (ni fenêtre de Git Credential Manager, ni programme askpass, ni phrase de passe ssh).
- Un dépôt dont l'utilisateur fait déjà un fetch, un pull ou un push est sauté.
- Un dépôt partagé par plusieurs projets est récupéré dès que l'un d'eux le veut, au plus court de leurs intervalles et au plus long de leurs délais.
- Il récupère le remote de la branche suivie (sinon `origin`, sinon l'unique remote, sinon tous), avec `--quiet --prune --no-auto-maintenance`, comme avant.

### Les réglages

**Réglages › Tous les projets**, groupe « Git » (entre « Nouveaux projets » et « Projets fermés ») :

- « Récupérer automatiquement (git fetch) » : allumé par défaut.
- « Intervalle » : entre deux fetch d'un dépôt. Une valeur libre en minutes, de 1 min à 24 h, ou une valeur proposée (2, 5, 15, 30 min, 1 h, en pastilles) ; 5 min par défaut.
- « Délai d'attente » : au-delà, le fetch de fond est arrêté (lui et ce qu'il a lancé) et repris à l'intervalle suivant. Une valeur libre en secondes, de 30 s à 30 min, ou proposée (1, 2, 5, 10, 30 min) ; **5 min par défaut** (voir *Sur un gros dépôt*). Le délai d'un geste (« Fetch », « Récupérer », « Pousser ») ne change pas : 3 min.

**Réglages › Projets › *projet* › Git et worktrees**, après « Commit » :

- « Récupérer automatiquement (git fetch) » : « Comme le réglage général » (par défaut), « Toujours » (même réglage général coupé) ou « Jamais ». Dessous, le réglage général rappelé (« Réglage général : toutes les 5 min, délai de 5 min. », ou « Réglage général : coupé. ») et « Réglage général… », qui ouvre Tous les projets.
- « Intervalle propre au projet » et « Délai d'attente propre au projet » (absents avec « Jamais ») : vides, ceux du réglage général, montrés dans le champ ; mêmes bornes et valeurs proposées.
- Les trois se partagent dans `.escouade.yml` : `git.auto_fetch` (`default`, `always`, `never`), `git.fetch_every` (minutes ; `0`, le général) et `git.fetch_timeout` (secondes ; `0`, le général). Une valeur hors des bornes du fichier est ramenée à la borne, avec un avertissement du fichier (voir *.escouade.yml*). C'est ainsi qu'une équipe les impose à un projet ; ils sont dans le registre des réglages comme les autres, pour les rangs « recommandé » et « imposé » d'une équipe ou d'un poste géré quand ils viendront.

**Les bornes** : un nombre tapé hors des bornes est ramené à la plus proche quand le champ est quitté ou validé (Entrée), et la fenêtre le dit sous le champ (« 5000 est hors des bornes (1 min à 24 h) : ramené à 24 h. »). Un champ vidé ou qui n'est pas un nombre reprend sa valeur (pour un projet, vide veut dire « le général »). Le cœur les ramène aussi aux bornes à l'enregistrement et à l'usage.

### Dans le panneau Git

Au pied du sélecteur de branche, sous « Récupérer », « Pousser » et « Fetch », une ligne discrète (texte atténué, jamais une alerte) :

- « Récupéré il y a 3 min » (« Récupéré à l'instant », « Jamais récupéré »), l'heure exacte au survol (« Dernier fetch : … »). C'est le dernier fetch du dépôt, de fond ou à la main.
- Si le dernier fetch de fond a échoué depuis sa dernière réussite, « Fetch automatique : » et sa raison, les mots de git au survol :
  - il demande des identifiants : « Fais « Fetch » une fois : il reprendra ensuite. » ;
  - le délai est dépassé : « pas fini en 5 min. Le délai d'attente se règle dans les réglages. », avec « Régler », qui ouvre le réglage qui le fixe (celui du projet s'il a le sien, sinon Tous les projets) ;
  - le réseau ou le dépôt distant est injoignable (DNS, connexion, proxy, TLS, coupure) ;
  - sinon « git a refusé (au survol) ».
- Avec un Git for Windows dont le ssh cale (voir plus bas) : « Ton Git for Windows 2.47.0 a un ssh qui cale sur les gros fetch : le fetch automatique passe par celui de Windows. Mets Git à jour (2.47.0(2) ou plus). »
- Un projet de plusieurs dépôts montre le premier échec de ses dépôts.

### Des identifiants demandés

Un fetch de fond qui demande des identifiants (clé ssh refusée ou pas chargée, `Cannot prompt because user interactivity has been disabled`, `terminal prompts disabled`, `Host key verification failed`…) le dit une fois, puis **ne réessaie plus** : le dépôt attend le prochain fetch, pull ou push réussi de l'utilisateur (depuis le dossier du projet ou le worktree d'un agent), qui efface l'échec et relance le fetch de fond à son intervalle. Avant ESC-561, un tel dépôt était retenté toutes les 5 min (546 fois en trois jours chez l'utilisateur).

### La configuration de git de l'utilisateur

Le fetch de fond passe par la configuration de git de l'utilisateur telle qu'elle est (proxy, certificats, `insteadOf`, `core.sshCommand`, helpers d'identifiants), sans rien y ajouter ni y écrire : seulement `-c core.quotepath=false` et les variables qui empêchent toute demande d'identifiants. Le proxy des réglages d'Escouade (Réglages › Avancé › Réseau) ne lui est pas donné : derrière Zscaler, git suit le proxy que l'utilisateur a configuré pour git.

### Sur un gros dépôt

Mesures du 2026-11-20 sur le dépôt d'Escouade (800 Mo de packs, 264 étiquettes, 8 branches et 24 PR sur origin), avec un clone en retard de 8 jours (9 700 objets, 120 Mo à recevoir) :

- **Avant** : avec le ssh de Git for Windows 2.47.0.windows.1 (OpenSSH 9.9p1), le transfert cale après quelques Ko (16 Ko en 600 s, puis GitHub coupe ; aucun octet de plus en 150 s, quatre essais sur quatre, branche seule et `--no-tags` compris) ; ssh et `index-pack` au repos, sans CPU. Le délai de fond de 60 s l'arrêtait et il repartait de zéro à chaque intervalle : 15 échecs de suite le 2026-11-13 chez l'utilisateur, des `tmp_pack` de 16 Ko laissés dans `.git/objects/pack`. C'est le bug git-for-windows/git#5199, corrigé dans 2.47.0(2).
- Le même fetch en HTTPS : 6,6 s (5,4 s sans étiquettes, 4,9 s la branche seule) ; avec le ssh de Windows (`OpenSSH_for_Windows_9.5p2`) : 5 à 10 s. Un fetch sans rien de nouveau : 1,5 s. Les étiquettes et les autres branches ne coûtent presque rien : elles ne sont pas la cause, et le fetch de fond les garde.
- **Après** : par le fetch de fond d'Escouade, 12,3 s, par le ssh de Windows, sans dépassement.

D'où :

- **Le ssh de Windows, seulement pour ce Git** : quand `git --version` dit `git version 2.47.0.windows.1`, qu'un remote récupéré parle ssh (`ssh://`, `git+ssh://`, `[user@]hôte:chemin`), que l'utilisateur n'a choisi aucun ssh (`GIT_SSH`, `GIT_SSH_COMMAND`, `core.sshCommand`) et que Windows a le sien (`%SystemRoot%\System32\OpenSSH\ssh.exe`), le fetch de fond passe par lui, `-o BatchMode=yes` (rien n'est demandé), par `GIT_SSH_COMMAND` pour ce seul processus ; rien n'est écrit dans la configuration. Le panneau dit de mettre Git à jour. Choix de l'utilisateur du 2026-11-20 (« Contourner, et guider »).
- **Un délai de fond de 5 min** au lieu de 60 s : 120 Mo passent même à 0,5 Mo/s, et un fetch de fond ne gêne rien (un dépôt à la fois, sauté pendant un geste de l'utilisateur).

## Limites

- Le contournement ne vaut que pour le fetch de fond : un « Fetch » ou un « Récupérer » à la main passe par le ssh de Git, et cale de même avec Git for Windows 2.47.0(1) (le panneau dit de le mettre à jour).
- Avec le ssh de Windows, une clé protégée par une phrase de passe chargée dans l'agent de Git (et non dans le service ssh-agent de Windows) est refusée : le fetch de fond le dit comme des identifiants demandés et attend un geste.
- L'état du fetch de fond (dernier échec, attente d'un geste) n'est gardé que pendant que l'app tourne : il repart de zéro au redémarrage.
- Les rangs « recommandé » et « imposé » d'une équipe ou d'un poste géré viennent avec la 2.3 (ESC-532) ; d'ici là, c'est `.escouade.yml` qui partage le réglage d'un projet.

### Couper le réglage pendant un passage

Le réglage est relu juste avant que git parte : un passage préparé avant que le fetch automatique soit coupé (pour tous les projets, ou « Jamais » pour le projet) ne lance rien. Seul un `git fetch` déjà lancé à ce moment va jusqu'au bout.

### Sur un serveur avec la porte du réseau privé (RP-1, ESC-597)

Sur un `escouade-server` lancé avec `--gate` (voir *Atteindre un réseau privé*), le fetch de fond, comme les autres `git` du cœur qui joignent un dépôt distant (`fetch`, `pull`, `push`), reçoit les variables de la porte avec un jeton de son projet : `HTTPS_PROXY`, `ALL_PROXY` vers la porte et `GIT_SSH_COMMAND` par `escouade-server via %h %p` (la commande ssh de l'utilisateur gardée dedans). Une variable que le fetch avait déjà (le ssh de Windows) reste. Rien n'est écrit dans la configuration par le fetch ; c'est le démarrage d'un agent qui écrit `core.sshCommand` dans le dépôt. Le poste de bureau ne change pas.
