# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, dans la fenêtre d'import, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand ClickUp donne une raison, elle suit le message.

## À la connexion

**« Indique le jeton. »**
Le champ « Clé d'API » est vide. Remplis-le ; rien n'est parti chez ClickUp.

**« ClickUp refuse ces identifiants (401) »**
La clé est fausse, ou elle a été régénérée depuis. Recopie ta clé actuelle depuis « Créer une clé d'API ClickUp ».

**« ClickUp injoignable : … »**
L'appel n'est pas arrivé : pas de réseau, un proxy ou un pare-feu qui bloque `api.clickup.com`, ou plus de 15 secondes sans réponse. Vérifie ta connexion et Réglages › Avancé › Réseau.

## À l'import et à la synchronisation

**« La liste ClickUp 901… n'existe plus ou n'est plus accessible : choisis-en une autre dans Réglages › Projets › Tickets importés »**
La liste liée a été supprimée, ou ta clé, toujours bonne, n'y a plus accès. Choisis une autre liste pour le projet.

**« La liste « Sprint 12 » de ClickUp n'a pas le statut « review » : ABC-12 reste où il est ; choisis un autre statut dans Réglages › Projets › Tickets importés »**
Le statut choisi pour la colonne a été renommé ou retiré de la liste. La tâche ne bouge pas. Choisis un des statuts actuels de la liste dans la correspondance des statuts. Escouade ne réessaie pas : rien ne changerait.

**« ClickUp limite les requêtes (429) : réessaie dans un moment »**
ClickUp limite le nombre d'appels par minute. Le plugin a déjà attendu la fin de la limite (une minute au plus) et redemandé une fois. Une synchronisation qui échoue ainsi est réessayée seule.

**« ClickUp refuse l'accès (403) »**, **« ClickUp : introuvable (404) »**
Ta clé n'a pas accès à cette tâche ou à cet espace, ou la tâche n'existe plus.

**« ClickUp : erreur 500 »** (ou une autre erreur du service)
ClickUp a répondu par une erreur. Une synchronisation est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.

**« ClickUp : réponse illisible »**
ClickUp a répondu autre chose que du JSON, souvent une page d'erreur d'un proxy. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« ClickUp : aucun jeton pour … Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de clé au plugin : le compte n'en a plus. Déconnecte puis reconnecte ClickUp.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu la clé. Aucun appel ne part tant que c'est le cas. Relance Escouade, ou reconnecte le compte.

## Quand le plugin ne répond pas

**« Le plugin dev.escouade.clickup ne répond pas : … »**
Le plugin est coupé (vue Plugins, pour l'app ou pour ce projet), arrêté après des plantages, ou trop lent. Rien n'est perdu : la synchronisation repart quand il répond. Regarde sa page dans la vue Plugins : son état, et son onglet « Journal ».

**« Réponse illisible du plugin dev.escouade.clickup : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

## Une tâche ne se trouve pas

- Son statut est de type fini ou fermé : elle n'est pas proposée.
- Elle est dans une autre liste : un id ne trouve une tâche que dans la liste liée.
- Une recherche par nom lit au plus 5 pages de 100 tâches avant de montrer ce qu'elle a trouvé : « Afficher plus » continue.
