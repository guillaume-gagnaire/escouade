# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, dans la fenêtre d'import, ou au clic sur le ⚠ d'une carte dont la synchronisation a échoué. Quand Linear donne une raison, elle suit le message.

Linear répond par ses propres codes d'erreur ; le plugin les lit comme les erreurs HTTP qu'ils valent : une clé refusée comme un 401, un accès refusé comme un 403, une limite comme un 429, une autre requête refusée comme un 400.

## À la connexion

**« Indique le jeton. »**
Le champ « Clé d'API » est vide. Remplis-le ; rien n'est parti chez Linear.

**« Linear refuse ces identifiants (401) »**
La clé est fausse ou a été révoquée. Crée une nouvelle clé avec « Créer une clé d'API Linear ».

**« Linear injoignable : … »**
L'appel n'est pas arrivé : pas de réseau, un proxy ou un pare-feu qui bloque `api.linear.app`, ou plus de 15 secondes sans réponse. Vérifie ta connexion et Réglages › Avancé › Réseau.

## À l'import et à la synchronisation

**« Linear refuse de passer ENG-42 à « In Review » »**
Linear n'a pas accepté le changement d'état. Vérifie que l'état existe encore dans l'équipe, et choisis-en un autre dans la correspondance des statuts. Escouade ne réessaie pas : rien ne changerait.

**« Linear refuse le commentaire sur ENG-42 »**
Linear n'a pas accepté le commentaire. Vérifie que l'issue existe encore et que ta clé peut y écrire.

**« Linear refuse l'accès (403) »**
Ta clé n'a pas le droit de lire ou d'écrire ce qui est demandé, ou tu n'es plus dans l'équipe.

**« Linear limite les requêtes (429) : réessaie dans un moment »**
Linear limite le nombre d'appels par clé. Une synchronisation qui échoue ainsi est réessayée seule.

**« Linear : erreur 400 »**
Linear a refusé la requête telle quelle. Escouade la met de côté sans réessayer ; « Resynchroniser » la renvoie quand tu as corrigé ce que dit Linear.

**« Linear : erreur 500 »** (ou une autre erreur du service)
Linear a répondu par une erreur. Une synchronisation est réessayée seule, après 1, 5, 15 puis 60 minutes, pendant 24 heures ; « Resynchroniser » la renvoie tout de suite.

**« Linear : réponse illisible »**
Linear a répondu sans données, ou autre chose que du JSON, souvent une page d'erreur d'un proxy. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« Linear : aucun jeton pour … Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de clé au plugin : le compte n'en a plus. Déconnecte puis reconnecte Linear.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu la clé. Aucun appel ne part tant que c'est le cas. Relance Escouade, ou reconnecte le compte.

## Quand le plugin ne répond pas

**« Le plugin dev.escouade.linear ne répond pas : … »**
Le plugin est coupé (vue Plugins, pour l'app ou pour ce projet), arrêté après des plantages, ou trop lent. Rien n'est perdu : la synchronisation repart quand il répond. Regarde sa page dans la vue Plugins : son état, et son onglet « Journal ».

**« Réponse illisible du plugin dev.escouade.linear : … »**
Le plugin a répondu quelque chose qu'Escouade ne comprend pas. Relance-le depuis son onglet « Journal » ; si ça revient, signale-le.

## Une issue ne se trouve pas

- Elle est terminée ou annulée : elle n'est pas proposée.
- Elle est dans une autre équipe : un numéro ne trouve une issue que dans l'équipe liée, même si tu tapes la clé d'une autre (`OPS-42` cherche l'issue 42 de l'équipe liée).
