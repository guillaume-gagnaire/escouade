---
number: 103
id: "plugins-publish-publier-avant-la-revue-e"
title: "plugins:publish : publier avant la revue, et un jeton d'administration limité à trois outils sur /mcp"
date: "2026-11-06"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-353"
pages: ["store/revue-et-signature","store/serveur-mcp","plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-353"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":10,"removed":0},{"page":"store/serveur-mcp","added":4,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":7,"removed":0}]
accepted_at: "2026-11-07T02:47:55+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-353 écrit `npm run plugins:publish`. Le ticket et `docs/PLUGINS.md` §8.6.2 mettaient la publication en dernier, après la signature, et disaient de soumettre « par la même API que le MCP du store », qui ne prend qu'un jeton OAuth obtenu par consentement dans un navigateur. Deux faits du code empêchent de le faire tel quel : `plugin-review` lit la version au catalogue, et `acceptSignature` ne prend qu'une version à l'état `published`. Une commande en terminal n'a pas de navigateur, et la règle du jeton d'administration disait « ni publier ni supprimer ». La liste `BUILTINS` devait aussi se vider, alors qu'elle portait les catégories et les tuiles.

## Décision

1. La commande publie **avant** la revue, en version non vérifiée et en demandant la revue, comme pour un tiers (`docs/STORE.md` §4.3). La signature reste la dernière étape et donne le sceau. La chaîne devient : étiquette, soumission, contrôles, publication, revue, rapport, signature.
2. `/mcp` prend aussi un jeton `esc_admin_`, limité à `submit_version`, `get_version` et `publish_version`, avec les mêmes fonctions, limites et journal qu'un autre client. Il n'ouvre ni retrait, ni mise hors liste, ni changement de source, ni la file.
3. Rien n'est gardé par la commande : elle lit son avancée dans le catalogue et le dossier des rapports. Un rapport présent n'est jamais récrit.
4. `--changed` prend tout dossier dont la version n'est pas signée au store, pas seulement ceux sans étiquette : sinon un plugin arrêté à la lecture ne serait jamais repris. Une version `0.0.0` est laissée de côté. Une version signée ou retirée n'est pas touchée, une version publiée non signée va à sa revue sans nouvelle soumission.
5. La marque « Escouade » et la tuile viennent du préfixe `dev.escouade.*`. Les catégories d'une première version viennent de `store.categories` du manifeste, comme pour un tiers : 21 modules et Goose les déclarent. Cursor CLI, Gemini CLI et Mistral embarquent leur manifeste dans leur module, où cette clé changerait l'empreinte du tableau de `docs/PLUGINS.md` §8.29 : leurs catégories restent dans la commande (`EMBEDDED_CATEGORIES`) jusqu'à leur prochaine version.
6. Le verdict se lit dans la dernière section « Constats de la revue » : un titre cité d'un plugin plus haut ne décide plus de rien.

## Conséquences

Une version de la première main est visible au store, « En revue », entre sa publication et sa signature ; elle ne s'installe pas, l'app refusant un `dev.escouade.*` non signé. Un verdict autre que « Vérifier » la laisse publiée non vérifiée, comme pour un tiers : à retirer à la main s'il le faut. Un jeton d'administration volé peut désormais soumettre et publier sur les dépôts de son compte, sans sceau et sans rien pouvoir retirer. Le store doit être déployé avant le premier lancement. Le store ne montre plus aucun plugin de la première main tant que le chef ne les a pas publiés. Un dossier changé sans nouvelle version n'est pas republié. Plus tard, un outil OAuth dans la commande pourrait remplacer le point 2.
