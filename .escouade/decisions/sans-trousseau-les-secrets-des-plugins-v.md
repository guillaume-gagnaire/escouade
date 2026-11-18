---
number: null
id: "sans-trousseau-les-secrets-des-plugins-v"
title: "Sans trousseau, les secrets des plugins vont dans un fichier lisible du seul propriétaire, sur choix de l'utilisateur"
date: "2026-11-16"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-499"
pages: ["plugins/vue-d-ensemble","fenetre/reglages","introduction/donnees-et-coupures"]
tickets: ["ESC-499"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/vue-d-ensemble","added":10,"removed":0},{"page":"fenetre/reglages","added":5,"removed":0},{"page":"introduction/donnees-et-coupures","added":1,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Issue #37 : sous WSL ou Linux sans Secret Service, « Garder » un secret de plugin ne gardait rien. L'erreur, en anglais brut, apparaissait en bas de l'onglet, loin du champ, et `secrets/get` rendait `null`. Tout plugin qui déclare `contributes.secrets` était inutilisable là où il n'y a pas de trousseau. Le jeton MCP et les comptes des intégrations avaient déjà leur repli en fichier, pris tout seul ; les secrets des plugins n'en avaient aucun.

## Décision

1. Un secret que le trousseau n'a pas gardé (absent, refusé, ou qui ne le rend pas tel quel) est une erreur dite à côté du champ, avec sa cause, en français et en anglais.
2. La ligne offre alors « Garder dans un fichier lisible de toi seul » : `plugin-secrets.json` du dossier de données, en clair, droits du propriétaire seul dès la création, écrit par le chemin atomique de l'état, jamais dans les copies de l'état ni dans un journal. Il ne sert qu'aux secrets des plugins.
3. Le choix vaut pour le poste et c'est le fichier lui-même : jamais pris tout seul, jamais dans un projet ni dans `.escouade.yml`. Réglages › Plugins le montre et le retire ; le retirer efface ses secrets, après confirmation.
4. Le fichier est lu avant le trousseau, donc le plugin lit son secret par `secrets/get` sans changement du protocole : `null` reste la réponse pour un secret absent. Rien n'est ajouté pour distinguer « jamais saisi » de « refusé », puisque c'est l'utilisateur qui peut y remédier.
5. Un trousseau qui arrive ensuite ne déplace rien tout seul : « Déplacer vers le trousseau » est offert.

## Conséquences

Sur un poste sans trousseau, les plugins à secrets marchent, au prix d'un secret en clair sur le disque, choisi en connaissance de cause. Les autres secrets de la fenêtre gardent leurs règles : clé de voix et clé d'API d'un compte refusées et dites ; jetons des intégrations et des hébergeurs dans leur fichier, avec un avertissement. Une stratégie de poste géré pour interdire ce repli est à faire sur release/2.3 ; la clé proposée est `secrets.fileFallback: deny`. Le trait `SecretStore` gagne `present()` (vrai par défaut) et l'erreur typée `NoKeychain`. Les e2e jouent un poste sans trousseau par `ESCOUADE_TEST_KEYCHAIN=absent`, lu dans un bac à sable seulement.
