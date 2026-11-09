# Dépannage

Les messages ci-dessous sont ceux du plugin et d'Escouade, tels qu'ils s'affichent : sous le formulaire de connexion, sur la ligne du compte dans Réglages › Services connectés, sur la carte d'un agent ou dans l'onglet de sa PR. Quand Bitbucket donne une raison, elle suit le message.

## À la connexion

**« Indique l'e-mail. »**
Bitbucket se connecte avec l'e-mail du compte Atlassian du jeton. Remplis-le.

**« Bitbucket : donne un jeton. »**
Le champ « Jeton » est vide. Bitbucket n'a pas de CLI dont Escouade prendrait le jeton : crée-en un avec « Créer un jeton ».

**« Bitbucket refuse ces identifiants (401) »**
Le jeton est faux, expiré ou révoqué, ou l'e-mail n'est pas celui de son compte Atlassian, ou c'est un jeton sans droits (scopes). Crée un jeton d'API avec les droits de Bitbucket que le formulaire rappelle.

**« Bitbucket injoignable : … »**
L'appel n'est pas arrivé : pas de réseau, un proxy ou un pare-feu qui bloque `api.bitbucket.org`, ou plus de 15 secondes sans réponse. Vérifie ta connexion et Réglages › Avancé › Réseau.

## Sur la ligne du compte

**« Jeton expiré ou révoqué · @ada »**, **« Injoignable · @ada »**
La dernière vérification a été refusée, ou n'a pas joint Bitbucket. Remplace le jeton, ou « Vérifier » pour réessayer.

**« Trousseau du système illisible : relance Escouade ou reconnecte le compte. »**
Au démarrage, le trousseau n'a pas rendu le jeton. Aucun appel ne part tant que c'est le cas.

**« Le plugin Bitbucket est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies »**
Tu as coupé le plugin. Rallume-le dans la vue Plugins.

**« Le plugin Bitbucket s'est arrêté (…) : ses PR ne sont plus suivies jusqu'à sa relance »**
Le plugin s'est arrêté seul, la raison entre parenthèses. Sa page de la vue Plugins dit son état ; son onglet « Journal » le relance.

## Sur une PR

**« Hébergeur arrêté : PR plus suivies »**
Sur la carte d'un agent : le plugin est coupé ou arrêté. La carte garde la PR telle qu'elle a été lue en dernier ; la première lecture qui marche efface le message.

**« Bitbucket : le merge a échoué (…) »**
Bitbucket a mergé en arrière-plan et sa tâche a échoué, son état entre parenthèses. Regarde la PR dans Bitbucket (conflit, règle de branche).

**« Bitbucket : le merge n'est pas fini »**
Bitbucket merge encore en arrière-plan après une trentaine de secondes. Le merge peut aboutir seul : la PR le dira à la prochaine lecture.

**« Bitbucket : PR #42 introuvable »**
La PR n'existe pas dans ce dépôt, ou ton jeton ne la voit pas.

**« Bitbucket : commentaire inconnu (…) »**
La réponse ou la résolution vise un commentaire que Bitbucket ne connaît plus (supprimé).

**« Cet hébergeur ne sait pas faire : … »**
Ce qui est demandé n'existe pas chez Bitbucket dans ce plugin (l'auto-merge, la relance des jobs). Escouade le sait d'avance et ne le propose pas en général ; ce message le dit s'il arrive.

**« Bitbucket refuse l'accès (403) »**
Ton jeton n'a pas le droit demandé (un droit manquant parmi ceux du formulaire), ou ton compte n'a pas ce droit sur le dépôt (merger, approuver).

**« Bitbucket limite les requêtes (429) : réessaie dans un moment »**
Le budget de l'heure est épuisé : le plugin a déjà attendu et redemandé une fois. Attends un peu.

**« Bitbucket : introuvable (404) »**, **« Bitbucket : erreur 500 »** (ou une autre erreur du service), **« Bitbucket : réponse illisible »**, **« Bitbucket : trop de redirections »**
Le dépôt ou l'objet n'existe plus ou n'est pas visible, Bitbucket a répondu par une erreur, autre chose que du JSON (souvent un proxy), ou plus de dix redirections. Réessaie ; si ça dure, regarde Réglages › Avancé › Réseau.

**« Bitbucket : aucun jeton pour bitbucket.org. Reconnecte le compte dans Réglages › Services connectés. »**
Escouade n'a pas pu prêter de jeton au plugin. Reconnecte le compte.

**« Le plugin dev.escouade.bitbucket ne répond pas : … »**
Le plugin est coupé, arrêté ou trop lent. Rien n'est perdu : les PR se relisent quand il répond.
