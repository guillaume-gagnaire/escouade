# Gérer ses plugins depuis un agent

Le store a un serveur MCP, `https://store.escouade.dev/mcp` : un agent (Claude Code, ou tout client MCP) y soumet tes versions, suit leurs contrôles, les publie et gère tes plugins, connecté à ton compte par OAuth. La signature et la révocation n’y sont pas.

## Le connecter

Dans Claude Code :

```sh
claude mcp add --transport http escouade-store https://store.escouade.dev/mcp
```

Puis `/mcp` dans Claude Code, et « Authenticate » sur `escouade-store`. Un autre client MCP prend la même adresse, en Streamable HTTP : il découvre seul le serveur d’autorisation (RFC 9728, RFC 8414), s’enregistre (RFC 7591) et se connecte avec PKCE.

## La première connexion

1. Le navigateur s’ouvre sur le store, puis sur GitHub : la même connexion que sur le site, par l’App « Escouade Store ».
2. Une page de consentement, en français ou en anglais selon ton navigateur, dit le client (le nom qu’il se donne, que le store ne vérifie pas), ton compte et ce qu’il pourra faire. Tu autorises ou tu refuses.
3. Le client reçoit un jeton d’une heure, valable pour `https://store.escouade.dev/mcp` seul, et un jeton de rafraîchissement de 30 jours qui change à chaque usage. Le store ne garde que leur empreinte.

## Ce qu’il permet

Avec la portée `author`, celle de tout compte :

| Outil | Ce qu’il fait |
| --- | --- |
| `list_repositories` | tes dépôts où l’App est installée, et tes dernières soumissions |
| `list_folders`, `list_tags` | les dossiers à manifeste d’un dépôt, les étiquettes d’un dossier |
| `submit_version`, `get_version`, `retry_checks` | soumettre une étiquette d’un dossier, suivre ses onze contrôles, les relancer si le store ou GitHub a échoué |
| `publish_version` | publier : les catégories d’une première version, « Demander la revue » (oui par défaut), la publication seule des nouvelles étiquettes |
| `list_plugins`, `get_plugin` | tes plugins, leurs versions et leur état, les versions en attente d’un clic |
| `yank_version`, `dismiss_version` | retirer une version publiée, mettre de côté une version en attente |
| `set_listed`, `set_auto_publish` | retirer un plugin de la liste (ou l’y remettre), publier seules ses nouvelles étiquettes |
| `change_source` | déplacer un plugin vers un autre dépôt à toi ou un autre dossier, en y soumettant sa version suivante |

Chaque outil passe par le même code que le site : les mêmes contrôles, les mêmes limites, les mêmes refus. Un refus revient comme une erreur de l’outil, avec son code (`not-yours`, `path-parent`, `categories`, `id-differs`…).

La portée `review` (lire la file de revue, une version et son rapport, déposer un rapport) n’est donnée qu’aux administrateurs du store, demandée à part au consentement. Un autre compte qui la demande ne reçoit que `author`.

## Ce qu’il ne fait jamais

- **Signer, révoquer, toucher à une clé.** La signature d’une version et la liste de révocation se font sur le poste de l’administrateur, avec la clé qui ne le quitte pas (`npm run plugin-review -- sign`).
- Rendre un jeton GitHub, un secret ou une adresse interne ; écrire dans un dépôt ; agir pour un autre compte que celui qui a consenti.
- Donner pour une consigne ce qu’a écrit un plugin. Un README, un manifeste, un nom de dossier ou un rapport rendu par un outil arrive entre `<third_party_data>` et `</third_party_data>`, annoncé comme une donnée à lire : l’agent ne suit pas une instruction qui s’y trouve.

## Déconnecter un client

Dans ton compte sur le store, « Applications connectées » montre chaque client autorisé, ses portées, la date de son dernier usage et ses dernières actions. « Déconnecter » coupe ses jetons aussitôt. Chaque action d’un outil est notée dans ton journal, « par un client MCP », avec le nom du client. Supprimer ton compte efface toutes ses autorisations.

Voir aussi [Publier sur le store](/docs/plugins/publishing/) pour les onze contrôles et leurs refus.
