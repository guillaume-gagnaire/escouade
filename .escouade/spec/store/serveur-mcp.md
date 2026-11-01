---
title: "Le serveur MCP du store : gérer ses plugins depuis un agent"
subject: "store"
sources: ["docs/STORE.md","store/server/utils/oauth.ts","store/server/utils/mcp.ts"]
tickets: ["ESC-317"]
decisions: [27]
compared: null
---

# Le serveur MCP du store : gérer ses plugins depuis un agent

Détail et procédure : `docs/STORE.md` §2.5 et §9.2 (il fait foi).

Un auteur (et d'abord le chef d'escouade, pour les plugins `dev.escouade.*`) gère ses plugins depuis un agent : `https://store.escouade.dev/mcp`, un serveur MCP distant en Streamable HTTP, ajouté à Claude Code par `claude mcp add --transport http escouade-store https://store.escouade.dev/mcp`. Sans état : chaque `POST` est un échange complet, réponse en JSON ; `GET` et `DELETE` répondent 405.

## L'autorisation (OAuth 2.1, selon la spec MCP)
- Le store est le serveur d'autorisation et la ressource. Métadonnées de la ressource (RFC 9728, `/.well-known/oauth-protected-resource[/mcp]`, et `WWW-Authenticate` avec `resource_metadata` sur un 401) et du serveur (RFC 8414, `/.well-known/oauth-authorization-server`).
- Enregistrement dynamique (RFC 7591) de clients publics seulement ; adresses de retour en `https` ou en `http` sur la boucle locale ; nom du client nettoyé et montré comme le nom qu'il se donne.
- PKCE S256 obligatoire ; indicateur de ressource (RFC 8707) : un jeton ne vaut que pour `<store>/mcp`.
- Connexion par la connexion GitHub du site, puis page de consentement `/oauth/consent` en français ou en anglais (selon `ui_locales` ou le navigateur) : le client, le compte, ce qu'il pourra faire, ce qu'il ne fera jamais. Le retour porte `iss` (RFC 9207). La CSP de cette page n'ajoute à `form-action` que l'origine de retour du client.
- Jetons d'accès d'une heure, de rafraîchissement de 30 jours et tournants ; un code ou un jeton de rafraîchissement présenté deux fois révoque toute l'autorisation. La base ne garde que leurs empreintes. Révocation par le client (RFC 7009) ou par l'auteur.
- Portées : `author` pour tout compte ; `review` aux seuls comptes de `STORE_ADMINS`, demandée et montrée à part, relue à chaque appel. Un non-administrateur qui la demande reçoit `author` seule.
- « Applications connectées » dans l'espace de l'auteur : chaque client, ses portées, son dernier usage, ses dernières actions, « Déconnecter ». Un compte supprimé perd toutes ses autorisations.

## Les outils
- `author` : dépôts, dossiers, étiquettes ; soumettre et suivre les dix contrôles, relancer ; publier (catégories, « Demander la revue » oui par défaut, publication seule des nouvelles étiquettes) ; lister ses plugins et versions avec leur état ; retirer une version, mettre de côté une version en attente, retirer de la liste ; changer de source (le manifeste de la nouvelle source doit porter l'id du plugin).
- `review` : lire la file, une version et son rapport ; déposer un rapport.
- Chacun appelle les fonctions des routes du site : mêmes contrôles, limites et refus.

## Ce qu'il ne fait jamais
- Ni signature, ni révocation, ni clé : elles restent sur le poste.
- Ni jeton GitHub, ni secret, ni adresse interne rendus ; aucune écriture dans un dépôt ; aucune action pour un autre compte.
- Le texte venu d'un plugin est rendu entre `<third_party_data>` et `</third_party_data>`, annoncé comme une donnée, jamais une consigne.
- Chaque appel d'outil est journalisé (`mcp.<outil>`, « par un client MCP », le client).

## La documentation
- Une page publique courte du store, `/agents` et `/en/agents`, liée du pied de page et d'« Applications connectées ».
- La page « Le store depuis un agent » de la documentation du site (`plugins/store-mcp`, fr/en).
