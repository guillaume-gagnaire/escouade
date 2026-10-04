# Intégrations externes (Jira, Trello, GitHub Issues) — design

## But

Importer dans le Kanban d'un projet des tickets venus d'un système de tickets externe, et garder leur état synchronisé : quand un ticket bouge chez nous, son pendant externe change de statut et reçoit un commentaire, selon des règles réglables par projet. Pour commencer : Jira (Cloud), Trello et GitHub Issues.

Référence visuelle : bouton « ⤓ Importer » de l'en-tête du Kanban, modale « Importer des tickets » et onglet « Intégrations » des réglages de `design/Claude Code Manager.dc.html` (projet claude.ai/design 9047fbff…, relu le 2026-10-04). L'onglet se branche dans la modale de réglages unique d'ESC-4 (`SETTINGS_TABS`, `settingsForm`).

Demandes du ticket ESC-5 : connexion à chaque service dans les réglages ; modale d'import ; synchro des états (« en cours » quand le ticket est en cours ou à tester chez nous, « à tester » quand il est terminé chez nous) ; automatisation réglable (quel état externe à l'arrivée dans chaque colonne, commentaire…). Ajout de l'utilisateur en cours de route : l'import automatique du design, avec l'étiquette surveillée réglable.

## Comptes (réglages de l'app)

| Service | Champs | Vérification | Libellé du compte |
|---|---|---|---|
| Jira | site (`https://x.atlassian.net` ; http seulement vers cette machine), e-mail, jeton d'API | `GET /rest/api/3/myself` (Basic e-mail:jeton) | « e-mail · x.atlassian.net » |
| Trello | clé d'API, jeton (« Obtenir un jeton » ouvre la page d'autorisation de Trello pour la clé saisie) | `GET /1/members/me` | « @username » |
| GitHub | jeton (vide : celui de `gh auth token`) | `GET /user` | « @login » (« · via gh ») |

- « Connecter… » déplie le formulaire dans la ligne du service ; la connexion est vérifiée puis enregistrée tout de suite (hors du brouillon « Enregistrer » de la modale) ; une erreur s'affiche sous le formulaire (« Jira refuse ces identifiants (401) »). « Déconnecter » oublie le compte.
- Les secrets vivent dans `~/.escouade/integrations.json`, à part de `settings.json`, et ne repassent jamais par la fenêtre : elle ne reçoit que « connecté · libellé ».
- Les appels passent par le proxy des réglages (comme les quotas). Les adresses des API Trello et GitHub se remplacent par `ESCOUADE_TRELLO_API` / `ESCOUADE_GITHUB_API` (tests, démo).

## Sources liées (par projet)

Onglet « Intégrations », sélecteur de projet comme « Projets » et « Kanban » :

- **Sources liées à <projet>** : pour chaque compte connecté, le conteneur à importer — projet Jira, tableau Trello, dépôt GitHub (le dépôt `origin` du projet en tête, « (dépôt du projet) ») — ou « Aucun ». Liste dans un `<select>` (un compte Jira a vite des dizaines de projets).
- **Correspondance des statuts** : une ligne par colonne du Kanban (À faire, En cours, À tester, Terminé), une colonne par source liée : l'état externe à donner au ticket quand il arrive dans la colonne, ou « — inchangé » ; une dernière colonne « Commenter » (interrupteur par ligne). États proposés : statuts du projet Jira, listes du tableau Trello, pour GitHub « Ouverte », « Fermée » et une étiquette du dépôt (« ◆ in-progress »).
- Choisir un conteneur pré-remplit la correspondance : En cours et À tester → l'état qui ressemble à « en cours » (in progress, doing, en cours, wip…), Terminé → celui qui ressemble à « à tester » (à tester, to test, qa, review, recette…), À faire inchangé ; commenter à l'arrivée « À tester » et « Terminé ». GitHub : les étiquettes du dépôt qui leur ressemblent (jamais « Ouverte » ni « Fermée »).
- Liens, correspondances et commentaires font partie du projet (`Project.integrations`) et passent par « Enregistrer » avec le reste.

## Synchronisation et import automatique (réglages de l'app)

- **Synchronisation** : « Mettre à jour le statut externe » (défaut : oui), « Publier un résumé à chaque boucle » (défaut : non), « Extraire les critères d'acceptation » (défaut : oui).
- **Import automatique** : « Importer les tickets étiquetés » (défaut : non), « Étiquette » (défaut `claude-ready` : label Jira, étiquette Trello, label GitHub), « Vérifier toutes les » 5 / 15 / 60 min (défaut 15). Chaque passage importe dans « À faire » de chaque projet les tickets ouverts de ses sources liées qui portent l'étiquette et n'y sont pas déjà, avec les boucles max par défaut (5) ; un toast « 2 tickets importés depuis Jira dans <projet> ».

## Modale « Importer des tickets »

Bouton « ⤓ Importer » de l'en-tête du Kanban.

- Sans source liée : « Aucune source liée à ce projet » + « Lier une source » (réglages, onglet Intégrations du projet).
- Sinon : une pastille par source liée (lettre colorée, nom, conteneur, nombre de tickets cochés), « ⚙ Gérer les sources » ; recherche (envoyée au service, 300 ms après la frappe) ; filtres en pastilles propres au service — Jira : Assignés à moi, Sprint actif, À faire ; Trello : Mes cartes + une par liste ; GitHub : Assignées à moi + une par étiquette ; « Tout sélectionner », « n résultats », compte · conteneur.
- Ligne : case, clé externe (`ATL-1287`, `#142`, `#42`), titre, type (Story, Bug, Tâche ; Carte ; Issue), priorité · assigné · statut, « ✓ 3 critères détectés » (infobulle : les critères) ; « Déjà dans le Kanban » grisé et non cochable.
- Pied : « n tickets sélectionnés », boucles max 3 / 5 / 8 (défaut du Kanban : 5), Annuler, « Importer n tickets ». Les tickets arrivent à la fin de « À faire » ; toast « 3 tickets importés depuis Jira et Trello ».
- Une erreur du service (identifiants expirés, JQL refusée…) s'affiche à la place de la liste, avec le message du service.

## Ticket importé

- Clé interne `<PRÉ>-<n>` inchangée (branche `ticket/<clé>`, unicité) ; le ticket garde sa référence externe : service, id d'API, clé affichée, conteneur, adresse.
- Titre = titre externe ; description = texte externe (Jira : document ADF converti en markdown) + « Ticket Jira ATL-1287 : <adresse> » ; critères = ceux extraits (si l'extraction est activée et en trouve), sinon les critères par défaut.
- Extraction des critères : les éléments de liste sous un titre ou une ligne « Critères d'acceptation » / « Acceptance criteria » / « Definition of done » ; à défaut les cases à cocher (`- [ ]`, `taskList` ADF) ; Trello : la checklist nommée ainsi, sinon toutes les checklists.
- Carte du Kanban : pastille du service (J, T, GH) et clé externe avant le titre, lien vers le ticket externe ; ⚠ et infobulle quand la dernière synchro a échoué.
- Un ticket ne s'importe qu'une fois par projet (même service et même id). Le projet se souvient de tout ce qui y a été importé : l'import automatique ne ramène jamais un ticket supprimé depuis ; l'import à la main le peut.

## Synchro des états

À chaque changement de colonne d'un ticket importé (départ, passage « À tester », validation, renvoi, retour « À faire »), dans l'ordre, un ticket à la fois :

1. si « Mettre à jour le statut externe » et que la source liée du même conteneur donne un état pour la colonne : Jira — la transition dont la cible est cet état (rien si le ticket y est déjà ; erreur « aucune transition vers X ») ; Trello — la carte va dans la liste ; GitHub — « Fermée » ferme l'issue, « Ouverte » la rouvre, une étiquette rouvre si besoin, pose l'étiquette et retire les autres étiquettes de la correspondance ;
2. si la colonne est cochée « Commenter » : un commentaire — En cours : « Escouade : <clé> pris par un agent (branche ticket/<clé>). » ; À tester : « … prêt à tester » + critères ✓/○ + ce qui a été fait ; Terminé : « … terminé — <issue de la validation> » + lien ; À faire : « … revenu à faire ».

« Publier un résumé à chaque boucle » : à chaque nouvelle boucle d'un ticket en cours, « Escouade : <clé>, boucle n/max — k/N critères atteints » + critères.

Une synchro ratée est journalisée et notée sur le ticket (`external.error`, montrée par ⚠ sur sa carte) ; la suivante qui réussit l'efface. Elle ne bloque jamais le ticket.

## Architecture

- `src-tauri/src/integrations/` : `mod.rs` (types, stockage des comptes, extraction des critères, choix des états par défaut, synchro), `jira.rs`, `trello.rs`, `github.rs` (un client par service sur `reqwest`, adresse de base injectable), `fake.rs` (petit serveur HTTP des tests).
- Modèle : `Ticket.external: Option<ExternalRef>`, `Project.integrations: ProjectIntegrations { links, comments }`, `Settings.integrations: IntegrationSettings`.
- Commandes : `integration_accounts`, `integration_connect`, `integration_disconnect`, `integration_containers`, `integration_statuses`, `integration_issues`, `integration_import`. La fenêtre reçoit les comptes dans son état initial et dans les réponses de connexion.
- Accroche : `Core::edit_ticket` compare la colonne et la boucle avant / après et met la synchro dans une file qu'un seul worker vide dans l'ordre.
- Fenêtre : `IntegrationsTab.svelte` (onglet des réglages), `ImportModal.svelte`, pastille externe dans `TicketCard.svelte`, `lib/integrations.ts` (service → lettre, couleur, libellés).

## Tests

- Rust : chaque client contre le faux serveur (requêtes envoyées, réponses lues, erreurs) ; conversion ADF ; extraction des critères ; états par défaut ; import (dédoublonnage, critères, description) ; synchro de bout en bout sur le harnais du Kanban (un ticket importé qui part, passe à tester et est validé déplace et commente son pendant sur le faux serveur) ; import automatique.
- Vitest : onglet Intégrations (connexion, liaison, correspondance, enregistrement), modale d'import (sources, recherche, filtres, sélection, import), carte d'un ticket importé.
- e2e Playwright sur l'app réelle avec le faux serveur des trois services : connexion, liaison, import, synchro.
