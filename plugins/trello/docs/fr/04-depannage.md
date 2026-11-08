# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, dans la fenêtre d'import, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand Trello donne une raison, elle suit le message.

## À la connexion

**« Indique la clé d'API. »**, **« Indique le jeton. »**
Un champ est vide. Remplis-le ; rien n'est parti chez Trello.

**« Trello refuse ces identifiants (401) »**
La clé ou le jeton est faux, ou le jeton a été révoqué, ou il a été créé pour une autre clé. Recrée le jeton avec « Obtenir un jeton pour cette clé », après avoir collé la bonne clé.

**« Trello injoignable : … »**
L'appel n'est pas arrivé : pas de réseau, un proxy ou un pare-feu qui bloque `api.trello.com`, ou plus de 15 secondes sans réponse. Vérifie ta connexion et Réglages › Avancé › Réseau.

## À l'import et à la synchronisation

**« Trello refuse l'accès (403) »**
Ton compte n'a plus accès au tableau ou à la carte. Vérifie chez Trello que tu en es toujours membre, ou lie un autre tableau.

**« Trello : introuvable (404) »**
Le tableau, la liste ou la carte n'existe plus (supprimé, ou déplacé hors de ta portée). Pour un tableau, choisis-en un autre dans Réglages › Projets › Tickets importés ; pour une liste, change la correspondance des statuts.

**« Trello limite les requêtes (429) : réessaie dans un moment »**
Trello limite le nombre d'appels par clé et par jeton. Attends un peu : une synchronisation qui échoue ainsi est réessayée seule.

**« Trello : erreur 500 »** (ou une autre erreur du service)
Trello a répondu par une erreur. Une synchronisation est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.

**« Trello : réponse illisible »**
Trello a répondu autre chose que du JSON, souvent une page d'erreur d'un proxy. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« Trello : aucun jeton pour … Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin : le compte n'en a plus. Déconnecte puis reconnecte Trello.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu la clé et le jeton. Aucun appel ne part tant que c'est le cas. Relance Escouade, ou reconnecte le compte.

## Quand le plugin ne répond pas

**« Le plugin dev.escouade.trello ne répond pas : … »**
Le plugin est coupé (vue Plugins, pour l'app ou pour ce projet), arrêté après des plantages, ou trop lent. Rien n'est perdu : la synchronisation repart quand il répond. Regarde sa page dans la vue Plugins : son état, et son onglet « Journal ».

**« Réponse illisible du plugin dev.escouade.trello : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

## Rien ne bouge chez Trello

- La colonne n'a pas de liste dans la correspondance des statuts (« inchangé ») : la carte reste où elle est.
- « Mettre à jour le statut externe » est éteint dans le groupe « Synchronisation ».
- La carte du ticket porte un ⚠ : clique dessus pour lire la raison et « Resynchroniser ».
