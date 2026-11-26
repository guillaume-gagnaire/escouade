---
number: 334
id: "la-confiance-lsp-ne-couvre-pas-ce-qu-ecr"
title: "La confiance LSP ne couvre pas ce qu'écrit un agent ; les extensions jugées et servies par le cœur ; routes des forges et Open VSX gardées"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-648"
pages: ["editeur/serveurs-de-langage","editeur/extensions","serveur-equipe/store-prive","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-648","ESC-627"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/serveurs-de-langage","added":10,"removed":0},{"page":"editeur/extensions","added":10,"removed":5},{"page":"serveur-equipe/store-prive","added":2,"removed":0},{"page":"git-fichiers/hebergeurs-de-code","added":5,"removed":0}]
accepted_at: "2026-11-26T11:42:03+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-627, SR4-06 à SR4-09) relève : la confiance d'un projet pour ses serveurs de langage valait pour le worktree qu'écrit son agent (un node_modules/.bin ou un build.rs posé par l'agent s'exécutait à l'ouverture de l'éditeur) ; une extension coupée ou refusée restait servie (vsx_file, escouade-ext:) ; la liste imposée par l'équipe et le refus privé dépendaient de la liste de projets envoyée par la page ; « .. » dans un nom de branche changeait la route de l'API GitHub ; les redirections d'Open VSX étaient suivies vers tout hôte ; sa clé de signature était lue chez lui ; la compilation d'un plugin d'équipe tournait sur la machine de l'administrateur sans qu'il le sache.

## Décision

1. Dans le worktree d'un agent, un programme de ce worktree et un serveur qui exécute du code du projet attendent la confiance de ce worktree (par agent et dossier, le temps de l'app, lsp_trust_worktree), jamais celle du projet ; le refus dit le chemin du programme. 2. Une extension coupée ou refusée par la liste permise n'est plus servie, ni par le protocole ni par vsx_files/vsx_file. 3. Les couches imposées de la liste permise sont prises du poste et de tous ses projets d'équipe, jamais de la liste de la page ; une demande qui ne nomme aucun projet du poste est refusée comme privée si le poste en a un. 4. Les plugins des cinq forges refusent une route dont un segment est « . » ou « .. » (ou leurs formes %2e), rien n'est envoyé. 5. Le client d'Open VSX ne suit que les redirections vers son origine ou son stockage (openvsx.eclipsecontent.org, https), 5 au plus. 6. La clé d'Open VSX est épinglée dans l'app (lue le 2026-11-26, vérifiée sur une vraie signature) ; la clé servie n'est lue que du faux registre de développement. 7. Signer un plugin d'équipe demande à l'administrateur d'accepter que son poste le compile (case à cocher, et le cœur refuse sans) : pas de bac à sable local, un plugin privé n'allant pas au store public.

## Conséquences

Ouvrir le worktree d'un agent dans l'éditeur redemande la confiance pour rust-analyzer, typescript-language-server, gopls et tout programme du worktree, à chaque lancement de l'app. Un membre d'une équipe qui impose extensions.allowed y est tenu dans toutes ses vues. Si Open VSX change de clé ou d'hôte de stockage, les installations sont refusées jusqu'à une mise à jour d'Escouade. Les plugins GitLab 2.2.5, Gitea 2.2.4, Bitbucket 2.2.5, Azure DevOps 2.2.6 et GitHub 2.3.0 sont à signer et publier au store. Le plugin GitHub de main garde le point dans ses routes, en lecture seule (checks_at), sans delete_branch : non corrigé sur main.
