---
number: 27
id: "store-un-serveur-mcp-avec-oauth-2-1-pour"
title: "Store : un serveur MCP avec OAuth 2.1 pour gérer ses plugins depuis un agent"
date: "2026-10-31"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-317"
pages: ["store/serveur-mcp"]
tickets: ["ESC-317"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/serveur-mcp","added":23,"removed":0}]
accepted_at: "2026-10-31T23:29:07+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le chef d'escouade doit soumettre et gérer lui-même les plugins de première main (dev.escouade.*, plugins/<id>/ du dépôt de l'app) sans passer par le navigateur de l'utilisateur à chaque geste (demande du 2026-10-31). Jusqu'ici, tout ce que fait un auteur passait par la session du site ; seul le script de revue avait un jeton (administrateur).

## Décision

1. Un serveur MCP distant /mcp sur le store (Streamable HTTP, sans état, réponses JSON), derrière OAuth 2.1 selon la spec MCP : le store est serveur d'autorisation et ressource ; RFC 9728, RFC 8414, enregistrement dynamique RFC 7591 (clients publics seulement), PKCE S256 obligatoire, indicateur de ressource RFC 8707, iss RFC 9207, révocation RFC 7009.
2. Connexion par la connexion GitHub du site, puis une page de consentement fr/en (selon ui_locales ou le navigateur). Jetons d'accès d'une heure, de rafraîchissement de 30 jours et tournants (un réemploi révoque l'autorisation), gardés hachés. « Applications connectées » dans l'espace de l'auteur pour les voir et les déconnecter.
3. Deux portées : author (tout compte) et review (STORE_ADMINS seuls, relue à chaque appel).
4. Les outils appellent les fonctions des routes du site (mêmes contrôles, limites, refus). change_source vérifie d'abord que le manifeste de la nouvelle source porte l'id du plugin.
5. Jamais de signature, révocation ou clé par le MCP ; aucun secret ni adresse interne rendu ; le texte d'un plugin est rendu entre <third_party_data> et </third_party_data> ; chaque appel est journalisé « par un client MCP ».
6. Une page publique /agents (fr/en) sur le store, et une page plugins/store-mcp dans la doc du site.

## Conséquences

Migration 0010 (oauth_clients, oauth_requests, oauth_grants, oauth_tokens). Nouvelles dépendances épinglées : @modelcontextprotocol/sdk 1.32.2 et zod 4.6.7. Aucune variable nouvelle dans Coolify. Le chef soumet les plugins de première main par le MCP (docs/STORE.md §9.2) ; la signature reste sur le poste. Le faux GitHub des e2e gagne un dépôt de l'app (STORE_FIRST_PARTY_REPO = escouade-chef/escouade-app).
