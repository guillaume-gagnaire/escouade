# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, dans la fenêtre d'import, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand Jira donne une raison, elle suit le message.

## À la connexion

**« Indique l'adresse du site. »**, **« Indique l'e-mail. »**, **« Indique le jeton. »**
Un champ est vide. Remplis-le ; rien n'est parti chez Jira.

**« Le site Jira doit être en https (http seulement sur cette machine). »**
L'adresse du site commence par `http://`. Tape-la sans schéma (`atlas.atlassian.net`) ou en `https://`.

**« Jira refuse ces identifiants (401) »**
Le jeton est faux ou révoqué, ou l'e-mail n'est pas celui du compte Atlassian du jeton. Vérifie l'e-mail, et crée un nouveau jeton avec « Créer un jeton d'API Atlassian ».

**« Jira injoignable : … »**
L'appel n'est pas arrivé : un site mal tapé, pas de réseau, un proxy ou un pare-feu qui bloque le site, ou plus de 15 secondes sans réponse. Vérifie l'adresse du site, ta connexion et Réglages › Avancé › Réseau.

## À l'import et à la synchronisation

**« Jira : aucune transition de « En cours » vers « Done » pour ATL-1294 »**
Le workflow du ticket ne mène pas de son statut actuel au statut choisi pour la colonne. Escouade ne force rien et ne réessaie pas. Choisis dans la correspondance des statuts un statut que le workflow atteint depuis là, ou ajoute la transition dans Jira, puis « Resynchroniser ».

**« Jira refuse l'accès (403) »**
Ton compte n'a pas le droit de faire ce qui est demandé dans ce projet (lire, faire passer une transition, commenter). Vérifie ses permissions dans Jira.

**« Jira : introuvable (404) »**
Le projet ou le ticket n'existe plus, ou ton compte ne le voit plus. Pour un projet, choisis-en un autre dans Réglages › Projets › Tickets importés.

**« Jira limite les requêtes (429) : réessaie dans un moment »**
Jira limite le nombre d'appels. Une synchronisation qui échoue ainsi est réessayée seule.

**« Jira : erreur 500 »** (ou une autre erreur du service)
Jira a répondu par une erreur. Une synchronisation est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.

**« Jira : réponse illisible »**
Jira a répondu autre chose que du JSON, souvent une page d'erreur d'un proxy ou d'une page de connexion. Vérifie l'adresse du site et Réglages › Avancé › Réseau.

**« Jira : ce compte n'a pas de site. Reconnecte-le dans Réglages › Services connectés. »**
Le compte gardé n'a pas d'adresse de site. Déconnecte puis reconnecte Jira avec son site.

**« Jira : aucun jeton pour jira. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin : le compte n'en a plus. Déconnecte puis reconnecte Jira.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas. Relance Escouade, ou reconnecte le compte.

## Quand le plugin ne répond pas

**« Le plugin dev.escouade.jira ne répond pas : … »**
Le plugin est coupé (vue Plugins, pour l'app ou pour ce projet), arrêté après des plantages, ou trop lent. Rien n'est perdu : la synchronisation repart quand il répond. Regarde sa page dans la vue Plugins : son état, et son onglet « Journal ».

**« Réponse illisible du plugin dev.escouade.jira : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

## Un ticket ne se trouve pas

- Il est dans la catégorie « Done » : il n'est pas proposé.
- Une puce le cache : « Sprint actif » et « À faire » se cumulent avec la recherche.
- Il est dans un autre projet : la clé d'un autre projet ne trouve rien dans le projet lié.
