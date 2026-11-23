---
number: 309
id: "la-2-2-6-a-la-2-2-8-reversees-dans-la-2"
title: "La 2.2.6 à la 2.2.8 reversées dans la 2.3 : l'ouverture bornée jugée par l'hôte, « Mode hors ligne » dans la liste des stratégies"
date: "2026-11-23"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-651"
pages: ["a-distance/ce-qui-change-de-machine","fenetre/reglages-imposes","kanban/tickets-partages"]
tickets: ["ESC-651"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/ce-qui-change-de-machine","added":6,"removed":0},{"page":"fenetre/reglages-imposes","added":7,"removed":0},{"page":"kanban/tickets-partages","added":4,"removed":0}]
accepted_at: "2026-11-24T00:58:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

release/2.3 reçoit par une vraie fusion (ESC-651, second parent v2.2.8, 0ab871e39) ce que la 2.2.6, la 2.2.7 et la 2.2.8 ont publié : les correctifs de l'audit de la 2.2.0 et ceux d'OshOEz. Plusieurs gardes de main touchent ce que la 2.3 a réécrit : l'ouverture par le système (SEC-12) passe sur la 2.3 par un hôte (le poste ou un serveur qui publie à son client), les stratégies du poste ont sur la 2.3 une liste unique de clés (TS-12), les tickets partagés y sont figés et versés à une équipe (TS-7), et la politique de contenu de la fenêtre y couvre la vue VS Code.

## Décision

1. Le cœur nomme les dossiers permis (ceux des projets, des worktrees, des agents, les données ; les téléchargements d'Escouade pour une copie reçue d'un serveur) et `Host::open` les reçoit : l'hôte du poste juge le chemin (`opening::allow`) puis l'ouvre ; celui d'un serveur le juge sur sa machine avant d'en parler au client. Rien n'ouvre un chemin sans ce jugement.
2. `network.offline` entre dans la liste unique des stratégies (restreindre, allumé seulement) : le mode lit toujours la stratégie du système, et les verrous de la 2.3 ramènent aussi le réglage, ce qui permet à une équipe de l'imposer.
3. La politique de contenu stricte de main (`style-src 'self'` et le nonce des `<style>`, `style-src-attr 'unsafe-inline'`) vaut aussi pour la 2.3 et sa vue VS Code, avec ce que la 2.3 avait ajouté (workers, tunnel local, médias).
4. Figer et verser les tickets partagés suivent les bornes d'ESC-653 : rien d'écrit à travers un lien, un fichier trop gros jamais réécrit, un dossier de plus de 2 000 fichiers ni figé ni versé.
5. Une remarque de relecture publiée seule garde sa valeur sur la 2.3 (`published`), et une valeur inconnue se lit comme ouverte (la lecture indulgente d'ESC-586).

## Conséquences

Les deux lignes gardent leurs garanties ensemble ; un client sur un serveur ne reçoit jamais un chemin hors des dossiers du serveur. Les fichiers générés des stratégies (ADMX, profils, schéma, page du site) gagnent une ligne. Les commandes de main (`harness_agree`, `harness_launch_ask`, `harness_launches`, `inbox_view_reopen`, `offline_state`) sont servies par l'API distante. Les 19 décisions de main reçoivent les numéros 0290 à 0308 et sont reportées sur main à part.
