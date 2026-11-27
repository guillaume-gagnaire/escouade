---
number: null
id: "les-plugins-peuvent-se-mettre-a-jour-tou"
title: "Les plugins peuvent se mettre à jour tout seuls, réglage éteint d'office, sous la règle de la fenêtre commune"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-700"
pages: ["store/store-dans-l-app"]
tickets: ["ESC-700"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":11,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Jusqu'ici une mise à jour de plugin était proposée, jamais installée seule (PLG-4, ESC-436). Le 2026-11-27 l'utilisateur a demandé « une configuration pour activer les mises à jour auto pour les plugins ». Installer sans geste ouvre un risque : une version que personne n'a relue, ou qui demande plus (une permission, un hôte, une nouvelle ligne de harness), partirait sans l'accord de l'utilisateur ; une installation au milieu d'un tour couperait un agent.

## Décision

Un réglage « Mettre à jour tout seuls » (Jamais, Les plugins d'Escouade, Tous les plugins vérifiés), éteint d'office, à côté de « N'installer que des versions vérifiées » et dans Réglages › Plugins › Sécurité. Ne part seul que ce que la fenêtre commune de « Tout mettre à jour » confirmerait (ESC-438) : une version du store, vérifiée avec les clés de l'app après relecture de la liste de révocation, qui ne demande rien de plus (ni permission, ni contribution, ni ligne de harness changée). Tout le reste reste proposé. Après chaque recherche (démarrage, 6 heures, « Chercher maintenant »), jamais quand le store n'a pas répondu. Le plugin d'un harness en plein tour attend la fin du tour. Une erreur laisse la version installée, est dite sur la page Mises à jour et n'est pas réessayée avant le prochain démarrage. La page Mises à jour et l'onglet Versions disent ce qui est parti tout seul ; la statistique `update` suit l'accord aux statistiques.

## Conséquences

La règle « proposé, jamais installé seul » ne vaut plus que réglage éteint. Le cœur garde, tant que l'app tourne, les versions essayées seules et les échecs ; le plugin installé garde sa dernière mise à jour toute seule (`autoUpdated` de ses choix), effacée par un geste. Le choix est fait par le cœur (`escouade_plugins::auto`), non par la fenêtre : il doit rester aligné sur `asksMore` de la fenêtre. Une version de l'amorce et tout plugin hors du store restent au geste. Les rangs imposés d'une équipe (2.3) pourront couper ce réglage.
