---
title: "Stats"
subject: "Fenêtre"
sources: ["docs/SPEC.md","tests/fixtures/copilot/full-turn.jsonl","tests/fixtures/engines/copilot-1.0.98.session-events.schema.json","tests/fixtures/engines/codex-0.163.0.schema.json"]
tickets: ["ESC-30","ESC-662","ESC-691"]
decisions: [343,"en-2-2-x-aussi-les-sous-agents-comptent"]
compared: null
---

## Ce que c'est

L'écran des statistiques d'usage : tokens, coûts et prompts des agents lancés par Escouade, sur une période, avec leur répartition par projet, modèle, agent, ticket, compte et harness. Il s'ouvre par « Stats », en bas de la barre des projets.

## Comportement

### Périmètre

- Seulement les agents lancés par l'app.
- Fermer un projet ne touche pas à ses statistiques. Tant que l'app ne le connaît plus, il y figure comme « Projet fermé », ses agents comme « Agent supprimé ». Rouvert dans son dossier, il retrouve son nom, ses agents et ses chiffres (voir *Fermer un projet*).

### Plages

- Jour (14 jours), Semaine (12 semaines), Mois (12 mois). Toutes les vues suivent la plage choisie.

### Vues

- Indicateurs : Tokens (avec l'évolution par rapport à la période précédente), Coût global (depuis le premier agent), Coût moyen par prompt, Prompts.
- Histogramme empilé en 3 séries : entrée, cache, sortie.
- Répartition par projet et par modèle.
- « Par agent » : les agents de la période, archivés compris, du plus cher au moins cher, avec nom, projet, tokens et coût ; « Agent supprimé » quand l'app ne le connaît plus.
- « Par ticket » : clé, titre, nombre de boucles, coût cumulé de ses agents (archivés compris) sur la période.
- Ces deux listes montrent 20 lignes, puis « Tout voir ».

### Comptes

- Chaque tour et chaque prompt porte le compte de l'agent à ce moment (voir *Comptes*). Un agent repris sur un autre compte compte chaque tour pour le compte de ce tour.
- Avec plusieurs comptes réglés, ou des tours de plusieurs comptes (même d'un compte supprimé depuis, « Compte supprimé (id) »), un sélecteur « Compte » filtre toutes les vues : plages, indicateurs (le « Coût global » depuis le premier tour du compte, « Prompts » et « Coût moyen / prompt » de ce compte), histogramme, « Par projet », « Par modèle », « Par agent », « Par ticket ».
- Il propose « Tous les comptes » (défaut), puis les comptes dans l'ordre des réglages. Le choix n'est pas retenu d'une visite à l'autre (l'écran s'ouvre sur les totaux de la barre de statut) ; il survit au changement de plage. Un compte qui ne peut plus être choisi ramène à « Tous les comptes ».
- La carte « Par compte », entre « Par projet / Par modèle » et « Par agent », met toujours tous les comptes côte à côte, du plus cher au moins cher : tokens, coût, tours (une réponse de Claude compte pour un tour), agents distincts sur la période. La ligne du compte choisi est marquée.

### Harness

- Chaque tour et chaque prompt porte le harness qui l'a fait.
- Avec plusieurs harness, un sélecteur « Harness » (« Tous les harness », puis chacun) filtre toutes les vues comme celui des comptes, et se combine avec lui : le sélecteur « Compte » ne propose alors que les comptes de ce harness, et « Par compte » ne liste qu'eux.
- La carte « Par harness », avant « Par compte », met les harness côte à côte : tokens, coût, tours, agents (ceux du compte choisi, s'il y en a un). La ligne du harness choisi est marquée.
- Un coût que le harness ne donne pas exactement s'écrit « ≈ 0,40 $ » avec « estimé » (infobulle : d'après les tokens et le tarif public de ses modèles). « — » pour un harness qui ne donne aucun coût. Les totaux sont « estimé » dès qu'un tel harness y compte.

### Graphique

- Le graphique garde son infobulle riche (titre, séries), au style des infobulles de l'app (voir *Fenêtre*).

## Limites

- Les agents lancés hors d'Escouade ne comptent pas.
- Les statistiques n'enregistrent que les coûts exacts que donne le harness, ou son estimation quand il n'en donne pas (voir *Barre de statut*).

### Sous-agents (ESC-662, ESC-691)

- Un sous-agent que lance un agent (le `Task` de Claude Code, le `task` de Copilot CLI, un sous-agent de Codex, d'OpenCode, de Cursor CLI…) tourne dans le process de son agent, donc sur son compte et son harness. Ce qu'il consomme compte dans le tour de son agent, **une fois** : dans les totaux de l'agent (en-tête, carte) et dans les stats par projet, harness, compte et modèle. Le total qu'un harness donne d'un sous-agent à sa fin (les `totalTokens` de Copilot CLI) s'affiche sur sa carte et ne s'y ajoute jamais.
- Par modèle, il compte pour le modèle sur lequel il a tourné, quand le harness le dit : Claude Code (son `modelUsage`), Copilot CLI (le modèle de chaque appel), Codex (le modèle que nomme le départ du sous-agent), OpenCode (le modèle de chaque message, fournisseur compris). Sinon, pour le modèle de l'agent (Cursor CLI, Mistral Vibe, Gemini CLI, Antigravity CLI, qui donnent l'usage du tour entier).
- Un tour ainsi réparti sur plusieurs modèles reste un seul tour dans « Par compte » et « Par harness ».
- Chez Codex, le coût estimé d'un sous-agent suit le tarif public de son modèle.
- Chez Copilot CLI, l'usage d'un sous-agent dont le plugin n'a pas vu le départ (plugin redémarré pendant qu'il tournait) compte aussi : il n'apparaît sous aucun `Task`, mais il est consommé sur ce compte.
- Le quota lu d'un compte (crédits du mois de Copilot CLI, fenêtres de Claude Code et de Codex) bouge avec ce que consomment les sous-agents de ses agents, puisqu'ils tournent sur lui.
- Ne compte pas : ce qu'un agent envoie hors de son process, comme le `/delegate` de Copilot CLI, que seul l'utilisateur tape dans son interface (l'agent de GitHub dans le cloud, sur le compte GitHub de la connexion).
