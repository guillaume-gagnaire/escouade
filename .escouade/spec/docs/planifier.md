---
title: "Planifier"
subject: "Docs"
sources: ["website/content/docs/fr/docs-space.md","docs/FACTORY.md","src-tauri/src/docsplan.rs","src-tauri/src/core.rs","src/lib/i18n/fr/docs.ts"]
tickets: ["ESC-262","ESC-291"]
decisions: []
compared: null
---

## Ce que c'est

Quand la spec a changé, « Planifier » demande à un agent planificateur les tickets qui mettent ces changements en œuvre. Escouade les crée au Kanban, dans « À faire », sans les lancer, chacun lié aux pages et aux décisions qu'il implémente.

## Comportement

### Où

- Sur une discussion dont les changements ont été versés (voir *Discussions*).
- En tête de la spécification, tant que des changements n'ont pas de tickets : « N changements sans tickets · Planifier ».

### Ce qui est à planifier

Depuis la dernière planification : les pages versées par une discussion, les pages dont le texte a été écrit à la main dans la fenêtre, et les décisions proposées par une discussion ou depuis la fenêtre. Une page seulement créée, sans texte, n'en est pas. Escouade retient ces changements par projet, dans son dossier de données.

### Le planificateur

- Il lit ces pages et ces décisions, le code (les fichiers que l'index du code rapproche des changements, 12 au plus, et le dossier du projet) et les tickets du Kanban, pour n'en créer aucun deux fois ni un pour ce qui est déjà fait.
- C'est un agent court qui lit sans rien modifier, sur le harness de « Modèle des textes rédigés » (voir *Langues*), 10 minutes au plus. Une planification à la fois par projet. « Le planificateur lit les changements… » pendant qu'il travaille.

### Les tickets

- 12 au plus par planification, créés dans « À faire », chacun avec sa description, ses critères d'acceptation, les tickets qu'il suit (« Après ») et les pages et décisions qu'il implémente, que sa description cite pour son agent. Chaque page reçoit la clé du ticket dans ses `tickets`.
- Un ticket dont le Kanban a déjà le titre, dans n'importe quelle colonne, n'est pas recréé, pas plus qu'un titre répété : « Déjà au Kanban : … ». Rien de nouveau : « Aucun ticket nouveau : le Kanban couvre déjà ces changements. »
- Ils ne sont pas lancés : le pilote auto et les règles du Kanban s'appliquent comme à tout ticket (voir *Pilote auto*).

### Le compte rendu

- Ce qu'il a créé s'affiche là où il a été demandé, sous la discussion ou en tête de la spécification : « N tickets créés … », dépliables (« Voir les tickets »), chacun ouvert au Kanban d'un clic. Son état : « À lancer », « N lancés sur M », « Tous lancés », « Annulé » ou « Retirés du Kanban ».
- « Annuler » retire les tickets pas encore lancés ; ceux déjà lancés restent. Les changements sont de nouveau à planifier. « Fermer » ferme le compte rendu ; les tickets restent au Kanban.
- La carte d'un sujet montre les clés de ses tickets ; la carte d'un ticket au Kanban, les pages qu'il implémente (« Spec : … »), qui s'ouvrent d'un clic dans l'onglet Docs.

## Limites

- Une décision acceptée ne reçoit pas la clé du ticket qui l'implémente : elle ne se réécrit plus. Le lien reste du côté du ticket.
- Le planificateur ne lance aucun ticket et ne modifie aucun fichier.
