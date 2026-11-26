---
number: 322
id: "hors-du-lancement-du-projet-un-agent-mon"
title: "Hors du lancement du projet, un agent montre sans demander ; lire et agir demandent l'accord de l'utilisateur"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":13,"removed":1}]
accepted_at: "2026-11-26T00:48:40+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-663, précisé par l'utilisateur le 2026-11-26 : un agent ouvre soit sa page (ESC-661), soit un vrai onglet de navigateur, qu'il place où il veut et qui peut sortir du lancement du projet. Exemple : l'artefact claude.ai qu'il édite. Cet onglet garde le stockage du projet, où l'utilisateur peut être connecté. Il ne doit pas devenir une session que l'agent manœuvre ou lit au nom de l'utilisateur. La question a été posée dans « À toi » (a-42).

## Décision

Le choix A de l'utilisateur (2026-11-26). Sur une origine qui n'est pas un lancement du projet, trois paliers.

**Montrer, sans demander** :
- ouvrir sur une adresse permise, placer (`full`, `beside`, `below`) ;
- changer de largeur, défiler, recharger, Retour et Avancer dans ce que l'agent a ouvert ;
- mettre en évidence (l'agent ne sait que « trouvé » ou non), fermer.

**Lire et agir, avec l'accord de l'utilisateur**, une fois par origine et par navigateur, donné dans une bande native et retiré par le menu de la ligne :
- `read_browser`, `capture_browser`, `browser_console` ;
- `click`, `type`, `select`, `press`, `tab`, `navigate` ailleurs dans l'origine.

**Toujours interdit**, même avec l'accord : taper un mot de passe, envoyer un fichier, télécharger, les cookies, le stockage et les en-têtes, taper un secret d'Escouade.

Les artefacts que l'agent a publiés s'ouvrent sans demander, au palier « montrer ». Leur adresse est prise dans le résultat de son outil `Artifact`. Toute autre origine est demandée (choix C3). L'utilisateur se connecte lui-même dans l'onglet, jamais l'agent.

Pourquoi : une page connectée montre plus que l'artefact (compte, autres conversations), et l'agent sait déjà ce qu'il a changé. Il peut montrer sans voir, et demande une fois s'il veut vérifier.

## Conséquences

Les outils de lecture du lot 1 (AB-2 à AB-4 : lire le HTML, capturer, la console) demandent cet accord hors du lancement, et apportent la bande d'accord et son retrait. AB-7 y ajoute les gestes d'action. AB-10 couvre l'option `artifact` d'`open_browser` et l'essai de claude.ai. Les e2e AB-5 et AB-12 vérifient le palier. Sur le lancement du projet, rien ne change : l'agent y lit et agit sans demander, sauf le mot de passe (choix C2).
