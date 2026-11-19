# Plusieurs serveurs : les espaces

Un même Escouade peut piloter plusieurs serveurs, un pour le travail et un perso. Chaque machine est un **espace** : ce poste, et chaque serveur auquel tu t’es [connecté](connect-server), avec ses projets, ses agents et ses réglages.

Rien ne passe d’un espace à l’autre. Tant que tu n’as qu’un espace, la fenêtre ne change pas.

## Les espaces dans la barre des projets

- **L’espace ouvert** est en tête de ses projets : sa plaque (un écran pour ce poste, une lettre pour un serveur) et son nom.
- **Chaque autre espace** est une pile sous le « + » : deux de ses projets en cartes derrière sa plaque, et une pastille avec le nombre de demandes qui y attendent. Un clic y mène. Son nom passe en ambre quand sa connexion est coupée.
- Au-delà de trois espaces, ou dans une fenêtre de 1024 pixels de large, les piles se tassent en simples plaques : la barre ne défile pas pour elles.
- Le clic droit sur un espace ouvre son menu : « Aller sur… », « Ouvrir dans une fenêtre », « Réessayer » s’il est coupé, les réglages de l’espace, « Retirer… de ce poste ».

Chaque serveur reste connecté, même quand tu es sur un autre espace : son tunnel SSH et sa connexion reprennent seuls après une coupure, sans toucher aux autres.

## « Partout » : tout ce qui attend, d’un coup d’œil

Dès deux espaces, la vue d’ensemble (`Ctrl+Maj+A`) s’ouvre sur **« Partout »** : ce qui attend dans chaque espace, rangé sous sa machine (son nom, son adresse, le temps d’un aller-retour), l’espace ouvert d’abord. Ses onglets filtrent par espace.

- **Tu y réponds sans changer d’espace** : une autorisation lisible en entier s’accorde par « Autoriser sur Travail », et part au serveur Travail.
- **La machine est écrite sur chaque demande et sur son bouton**, pour ne jamais autoriser une commande sur le mauvais serveur.
- Une question, un plan, ou une demande trop longue pour être lue ici se répondent dans la conversation : « Ouvrir » t’y emmène.
- Un espace coupé est en ambre, avec l’heure de ce que tu vois ; ses demandes sont grisées, sans bouton, jusqu’à la reprise.

## Changer d’espace

- Un clic sur une pile, « Ouvrir » dans « Partout », ou une notification.
- `Alt+1` à `Alt+9` (`⌃1` à `⌃9` sous macOS) : le n-ième espace, ce poste étant le premier. `Ctrl+²` (la touche sous Échap) : l’espace suivant. `Ctrl+1` à `Ctrl+9` restent aux projets ; une question à l’écran garde `Alt+1` à `Alt+9` pour ses options.
- « Ouvrir dans une fenêtre » met un espace dans sa propre fenêtre, avec sa barre de titre et sa machine. Une seule fenêtre par espace : la redemander ramène la première.

## Les notifications

Une demande d’un autre serveur te notifie avec le nom de son espace en tête : « Travail · facturation · relances-impayes ». Celles de ce poste disent « Ce poste ». Un clic t’emmène à l’agent, dans son espace.

## Les appareils d’un serveur

Sur un serveur, Réglages › « Appareils » liste les appareils appairés (ce poste est marqué), avec leur dernier passage. « Retirer » révoque un autre appareil : ses connexions se ferment. « Créer un code » donne un code d’appairage de 10 minutes pour un nouvel appareil, sans passer par le serveur.

## Sur un poste géré

Les [stratégies du poste](managed-policies) peuvent :

- **`servers.personalSpace` à `false`** : n’ajouter que les serveurs de la liste des serveurs permis (`servers.allowed`) ; ce poste reste un espace.
- **`servers.aggregateView` à `false`** : « Partout » ne montre d’un serveur de travail que ses nombres, ni texte ni bouton ; le détail reste dans son espace.
- **`servers.contentlessNotifications` à `true`** : les notifications d’un serveur de travail disent qu’un agent attend, jamais quoi.

Un serveur de travail est un serveur de la liste `servers.allowed` quand elle est posée, sinon tout serveur. Ces verrous se voient dans Réglages › « Serveurs », sous « Imposé sur ce poste ».
