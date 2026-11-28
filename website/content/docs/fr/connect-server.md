# Se connecter à un serveur

Escouade peut tourner sur ton serveur, jour et nuit, et ce poste le piloter par un tunnel SSH. Cette page explique comment s’y connecter la première fois, ce que l’app dit de la machine où tu es, ce qui arrive quand la connexion tombe, et comment revenir sur ce poste ou retirer un serveur.

## Avant de commencer

- Sur le serveur, `escouade-server` est installé et lancé. Il n’écoute que sur une socket Unix qui n’est qu’à son utilisateur (le service d’`install.sh`) ou sur son `127.0.0.1` (port `46480`) : rien n’est ouvert sur Internet.
- Ce poste se connecte déjà au serveur par `ssh`, avec une clé chargée dans ton agent SSH. Escouade lance le `ssh` de ce poste (sous Windows, OpenSSH du système), avec ta config, ton agent et tes hôtes connus. Il ne lit, ne copie ni ne garde aucune clé, et ne tape jamais de mot de passe.
- Le plus simple : un hôte dans `~/.ssh/config`, par exemple :

```
Host acme-escouade
  HostName escouade.acme.internal
  User escouade
```

## La première connexion

1. Ouvre Réglages › « Serveurs » et choisis « Se connecter à un serveur ». Dès qu’un serveur est connu, l’entrée est aussi dans le menu du « + » de la barre des projets.
2. **L’hôte SSH** : choisis un hôte de `~/.ssh/config` (ceux à motif, comme `Host *`, et ceux sans `HostName` ne sont pas listés), ou tape `utilisateur@adresse:port`. Le « Port d’Escouade sur le serveur » ne se change que si `escouade-server` tourne avec un autre `--port`.
3. **La connexion** : Escouade lit par SSH où écoute le serveur (`escouade-server status --json`), puis ouvre `ssh -N -L <port local>:<sa socket> <hôte>`, ou `ssh -N -L <port local>:127.0.0.1:46480 <hôte>` sur un port, et vérifie qu’Escouade répond sur le serveur, et dans quelle version. Si le serveur SSH refuse la redirection vers la socket, le dialogue le dit et propose le repli : le service sur un port (voir [Dépanner le tunnel](server#depanner-le-tunnel)).
4. **L’appairage**, une seule fois : sur le serveur, lance `escouade-server pair` et tape le code qu’il affiche (8 caractères, valable 10 minutes, pour un seul appareil). Ou clique « le lancer d’ici par SSH » : Escouade le lance par la même connexion et remplit le code. Donne un nom à l’espace (« Travail ») et à ce poste, tel que le serveur le verra.
5. La fenêtre passe sur le serveur : ses projets, ses agents, ses comptes et ses réglages sont ceux du serveur.

Le jeton de ce poste est gardé dans le trousseau de ce poste, un par serveur ; sans trousseau (Linux), dans un fichier lisible de toi seul. Il n’est jamais écrit dans les réglages ni dans le journal.

> [!NOTE]
> Escouade n’accepte jamais une clé d’hôte à ta place. Si l’hôte n’est pas encore dans tes `known_hosts`, connecte-toi une fois par `ssh <hôte>` dans un terminal, vérifie la clé, puis réessaie.

## Savoir sur quelle machine tu es

Dès qu’un serveur est connu, la machine se dit partout où se tromper coûterait :

- **La barre de titre** : la plaque et le nom de l’espace avant le projet (« Travail acme-escouade / facturation »), ou « Ce poste ». À 1024 pixels de large, l’hôte SSH s’efface, le nom reste.
- **La barre d’état** : « Travail · ssh acme-escouade · 38 ms », le temps d’un aller-retour.
- **La barre des projets** : l’espace ouvert au-dessus de ses projets, les autres en piles sous le « + », à un clic (voir [les espaces](spaces)).
- **Le pied de la barre latérale** : « Travail : » devant le dossier du projet, l’icône d’un serveur à la place d’un dossier.
- **Un terminal** : « Travail · escouade@vps.example.net » dans son en-tête.
- **« Nouveau projet »** : « sur Travail · escouade.acme.internal » ; les dossiers sont ceux du serveur, pas de ce poste (voir plus bas).
- **Un fichier envoyé par un agent** : « envoyé depuis Travail · copié sur ce poste dans … ».
- **Les secrets** : « Gardée dans un fichier sur Travail » là où une clé se donne.
- **Les notifications** : le nom de l’espace en tête, « Travail · facturation · relances ».

## Ce qui se passe sur le serveur, et ce qui reste sur ce poste

Sur un serveur, les projets, les agents et leurs fichiers sont là-bas ; toi, ton navigateur, ton micro et tes dossiers restez ici. Rien de ce que le serveur envoie ne s’ouvre sur ce poste sans ton clic.

- **Un nouveau projet** : « Cloner un dépôt » (le serveur clone avec ses propres clés git), ou « Un dossier du serveur » : ses dossiers, s’ils sont des dépôts et sur quelle branche, ceux qui sont déjà des projets, et la place libre de son disque.
- **Ce que tu envoies** : le logo d’un projet, un fichier joint, une image collée partent de ce poste vers le serveur, toujours en copie.
- **Ce qu’un agent t’envoie** : « Ouvrir » le télécharge à ce moment-là dans `Téléchargements/Escouade/Travail` et ouvre la copie. Un programme ou un script ne s’ouvre jamais : « Télécharger sur ce poste » le copie et le montre dans son dossier. « Voir sur le serveur » le montre dans le panneau Fichiers.
- **Un serveur de test** (Lancement, ou un lien vers `localhost`) : son port est relayé sur ce poste, et la page s’ouvre ici, rechargement à chaud compris.
- **Les notifications, le son, le clignotement** sont ceux de ce poste ; leur choix (Réglages › Notifications) est celui du serveur.
- **La voix** est celle de ce poste : son micro, sa transcription, ses clés et ses réglages. Seul le texte part vers le serveur.
- **Connecter un compte** (Codex, Claude Code, Gemini CLI…) : le harness tourne sur le serveur ; « Ouvrir la page sur ce poste » ouvre la page de connexion ici. Quand elle revient sur `localhost` (Codex, Gemini CLI), Escouade relaie ce retour au serveur ; sinon, colle dans le terminal le code qu’elle affiche (Claude Code), ou tape sur la page le code de l’appareil (Copilot CLI). Le compte reste sur le serveur.
- **Les secrets** (clés de Cursor, des services, des plugins) sont gardés dans un fichier du serveur, lisible de son seul compte : un serveur n’a pas de trousseau.

## Quand la connexion tombe

Les agents continuent sur le serveur : fermer l’app, éteindre ce poste ou couper le réseau ne les arrête pas. Une bande dit « Connexion à Travail perdue à 14:32 », le prochain essai et que ce que tu vois date de 14:32. Escouade relance le tunnel et la connexion seul (après 250 ms, puis de plus en plus loin, toutes les 5 s au plus) ; « Réessayer » le fait tout de suite, « Aller sur Ce poste » te ramène ici.

Pendant la coupure, l’envoi est suspendu : ton brouillon reste dans la zone de saisie, rien ne part en file. À la reprise, l’écran est relu en entier, les terminaux reprennent où ils en étaient, et « Reconnecté à Travail · 12 événements rattrapés » s’affiche.

## Ce que disent les échecs

Chaque échec dit ce qui s’est passé, que rien n’a changé sur ce poste ni sur le serveur, et quoi faire :

| Ce que tu vois | Ce que ça veut dire | Quoi faire |
| --- | --- | --- |
| « acme-escouade ne répond pas » | `ssh` n’a pas joint l’hôte (VPN coupé, serveur éteint, pare-feu, mauvais `HostName`) | Vérifie le réseau, « Réessayer » ; la commande `ssh` se copie pour l’essayer dans un terminal |
| « Clé d’hôte inconnue » | L’hôte n’est pas dans tes `known_hosts` | Connecte-toi une fois par `ssh <hôte>` |
| « La clé de … a changé » | La clé de l’hôte n’est plus celle que ce poste connaissait | Vérifie-la avec l’administrateur avant de la remplacer |
| « … refuse la connexion SSH » | Aucune clé de ton agent n’est acceptée | Charge ta clé (`ssh-add`), puis réessaie |
| « Pas d’Escouade sur le serveur » | `escouade-server` ne répond pas sur son port | Installe-le, ou démarre son service |
| « Ce code n’est pas le bon » / « Ce code a expiré » | Le code tapé est faux, ou a expiré | Retape-le, ou relance `escouade-server pair` |
| « … a une autre version d’Escouade » | Ce poste et le serveur n’ont pas la même version | Mets à jour le côté en retard |
| « Ce poste n’est plus appairé à Travail » | Ce poste a été révoqué, avec la date et l’appareil qui l’a fait | « Appairer de nouveau » avec un nouveau code, ou retire le serveur |
| « Refusé par la stratégie de ce poste » | Un poste géré n’autorise que certains serveurs (`servers.allowed`) | Voir les [stratégies du poste](managed-policies) |

## Un serveur qui joint le réseau de ton entreprise

Si le serveur est dans le réseau de ton entreprise, tu t’y connectes par SSH à travers ton VPN, avec un `ProxyJump` s’il faut un bastion (voir [Escouade dans le réseau de ton entreprise](private-network/company-server)). S’il est ailleurs, ses agents peuvent joindre ce réseau par un connecteur (voir [Atteindre un réseau privé](private-network/introduction)).

## Revenir sur ce poste, retirer un serveur

- « Ce poste » dans la barre des projets, ou « Aller sur Ce poste » dans la bande d’une coupure, ramène la fenêtre ici. L’app rouvre sur la dernière machine où tu étais.
- Réglages › « Serveurs » liste les serveurs que ce poste connaît, leur état et le geste qui va avec : « Ouvrir », « Réessayer », « Copier la commande », « Appairer de nouveau ».
- « Retirer » oublie le jeton et la connexion sur ce poste, et demande au serveur de révoquer ce poste s’il est joignable. Les agents du serveur continuent ; pour revenir, il faudra un nouveau code.
