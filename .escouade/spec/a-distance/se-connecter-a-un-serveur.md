---
title: "Se connecter à un serveur"
subject: "a-distance"
sources: ["docs/SERVER-UX.md (§ 2 le parcours, § 3 la connexion et ses états, § 4 dire sur quelle machine on est, § 7 S1, S4, S10, S11, S15, § 9, § 10, § 11)","design/server-2-3 (maquettes et captures : x-hote, b-connexion, b-erreur, b-coupure, x-etats, x-version, x-jeton, b-deux)","docs/REMOTE.md, « Le mode serveur », « Le tunnel : SSH d'abord », « Le client, MS-3 (ESC-537) »","src-tauri/src/servers (mod.rs, sshconfig.rs, tunnel.rs, client.rs)","src-tauri/src/remoteapi (proto.rs, server.rs, devices.rs, pairing.rs)","src/lib/transport.ts, src/lib/servers.svelte.ts, src/components/servers","tests/fixtures/fake-ssh.mjs, tests/e2e/server-connect.spec.ts, src-tauri/src/servers_tests.rs"]
tickets: ["ESC-537","ESC-539"]
decisions: [212,218]
compared: null
---

# Se connecter à un serveur

Le client de bureau pilote un Escouade qui tourne sur un serveur (voir *Le mode serveur*) par un tunnel SSH qu'il ouvre lui-même. Une fenêtre est sur une machine à la fois : **Ce poste** (son cœur est dans l'app) ou un serveur. La direction retenue pour ranger les machines est B, « les piles » : chaque serveur connu est un espace de plus, tenu connecté, et la vue « Partout » les réunit (voir *Les espaces*).

## L'entrée

« Se connecter à un serveur » est dans Réglages › Serveurs, et dans le menu du « + » de la barre des projets dès qu'un serveur est connu ou qu'un projet a été fermé (sinon le « + » ouvre « Nouveau projet » comme avant). Il n'y a pas de palette de commandes dans l'app.

## Le dialogue

Quatre étapes, à gauche : **Hôte SSH**, **Connexion**, **Appairage**, **L'espace**. Une étape réussie est cochée, une étape en échec est en rouge.

1. **L'hôte SSH** : les hôtes de `~/.ssh/config`, avec ce qu'ils visent (`utilisateur@adresse · port`, « par bastion (ProxyJump) ») ; ou un hôte tapé, `utilisateur@adresse:port`. Les hôtes à motif (`Host *`) et ceux sans `HostName` ne sont pas listés. Le « Port d'Escouade sur le serveur » vaut 46480. La config est lue avec ses `Include` (relatifs à `~/.ssh`, avec `*` et `?`), la première valeur donnée pour un hôte l'emporte, les blocs `Match` sont ignorés. Un hôte tapé qui commence par `-` ou contient un caractère de shell est refusé.
2. **La connexion** : la stratégie du poste d'abord (`servers.allowed`), puis le tunnel, puis la poignée de main sans jeton (Escouade répond ? dans quelle version ?).
3. **L'appairage**, la première fois : « Sur le serveur, lance `escouade-server pair` » (copiable), **ou « le lancer d'ici par SSH »**, qui lance `escouade-server pair` par le même hôte (cherché dans le `PATH`, `~/.local/bin`, `/usr/local/bin`, `/opt/escouade`) et remplit le code. Le code a huit cases (un `group` « Code d'appairage, 8 caractères ») : chaque lettre passe à la case suivante, un collage entier les remplit, `Retour arrière` revient en arrière, une lettre qui ne peut pas être d'un code (0, O, 1, I) n'est pas prise. Puis le nom de l'espace (pris de l'alias SSH, modifiable) et le nom de ce poste vu du serveur (« DESKTOP-7QK2 · Windows »).
4. **L'espace** : la fenêtre passe sur le serveur.

## Le tunnel

- Le `ssh` du système (sous Windows, OpenSSH de `System32`, sans console) : `ssh -N -L 127.0.0.1:<port local libre>:127.0.0.1:<port du serveur> <hôte>`, avec la config, l'agent et les `known_hosts` de l'utilisateur. **Escouade ne lit, ne copie ni ne garde aucune clé.**
- **Jamais une clé d'hôte acceptée à la place de l'utilisateur** : `BatchMode=yes` et `StrictHostKeyChecking=yes` sont donnés sur la ligne de commande, qui l'emporte sur la config. Aucun mot de passe n'est tapé.
- **Rien n'écoute ailleurs que sur 127.0.0.1 de ce poste.** Rien n'est envoyé au port local avant que `ssh` dise que la redirection écoute.
- **Surveillé** : sa sortie est lue ; tombé, il est relancé aux délais de la fenêtre (250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s), sur le même port local s'il est libre ; une fin que réessayer ne répare pas (une clé, un compte refusé) l'arrête et le dit. Il s'arrête avec l'app. **Chaque serveur connu garde le sien**, ouvert au démarrage de l'app, avec ses propres essais ; aller sur ce poste ne le ferme pas (voir *Les espaces*).

## Le jeton

Le jeton d'appareil reçu à l'appairage est gardé **dans le trousseau du poste, un par serveur** ; sans trousseau (Linux), dans un fichier des données de l'app lisible du seul propriétaire, et Réglages › Serveurs le dit. **Jamais dans les réglages ni dans le journal.** La liste des serveurs connus (nom de l'espace, hôte SSH, port, nom de ce poste là-bas, version et machine vues, dernier état) est dans les données de l'app, sans jeton.

## Les états

Chaque état dit **ce qui s'est passé**, **ce qui n'a pas changé** (« Rien n'a été changé sur ce poste ni sur le serveur », « Les agents du serveur continuent ») et **le geste suivant**, en français et en anglais.

| État | Ce qu'il dit, et le geste |
| --- | --- |
| Injoignable | « acme-escouade ne répond pas » ; la commande `ssh` et sa sortie ; les causes plausibles (le VPN pour une adresse `.internal`, `.lan`, `.corp`…, le serveur ou son pare-feu, le `HostName` de la config) ; « Copier la commande ssh », « Modifier l'hôte », « Réessayer » |
| Compte refusé | « … refuse la connexion SSH » ; charger sa clé dans l'agent (`ssh-add`) |
| Redirection interdite | « … refuse la redirection » (`AllowTcpForwarding no`) |
| Clé d'hôte inconnue | « Escouade n'accepte pas de clé à ta place : connecte-toi une fois par `ssh <hôte>` » ; « Copier la commande » |
| Clé d'hôte changée | La vérifier avec l'administrateur avant de la remplacer dans `known_hosts` |
| Pas d'Escouade sur le serveur | « escouade-server ne répond pas sur le serveur : est-il installé et lancé ? » |
| Code faux, ou expiré | Sous le code : « Ce code n'est pas le bon. » / « Ce code a expiré : relance escouade-server pair. » / trop d'essais ; le code se vide, le focus y revient |
| Version différente | Les deux versions et le côté à mettre à jour ; pour ce poste, « Mettre à jour Escouade » ; pour le serveur, la version à y installer (sa mise à jour depuis le client viendra) |
| Coupure et reprise | Voir plus bas |
| Jeton révoqué | L'espace entier : « Ce poste n'est plus appairé à Travail », quand et depuis quel appareil (ou « depuis le serveur ») ; « Appairer de nouveau », « Aller sur Ce poste », « Retirer Travail de ce poste » |
| Refusé par la stratégie | Avant toute connexion : « Le serveur … est refusé : la stratégie de ce poste ne permet que … » |

## La coupure et la reprise

- La bande de l'espace, en ambre : « Connexion à Travail perdue à 14:32 · Nouvel essai dans 4 s (6ᵉ). Les agents continuent sur le serveur ; ce que tu vois date de 14:32. », avec « Réessayer » et « Aller sur Ce poste ». La machine de la barre de titre passe en ambre avec « reconnexion », la barre d'état dit « Travail · reconnexion dans 4 s ».
- **L'envoi est suspendu**, pas mis en file : la zone de saisie dit « Envoi suspendu jusqu'à la reprise ; ton brouillon est gardé. », le bouton d'envoi est inactif, le brouillon reste.
- À la reprise : l'écran est relu en entier, les terminaux reprennent là où ils en étaient (voir *L'API distante*), « Reconnecté à Travail » s'affiche brièvement.
- Une connexion muette depuis 20 s (un ping sans réponse) est tenue pour coupée.

## Dire sur quelle machine on est

Dès qu'un serveur est connu :

- **La barre de titre** : la plaque (une lettre pour un serveur, un écran pour ce poste) et le nom de l'espace avant le projet, avec l'hôte SSH (masqué sous 1100 px de large) ; « Ce poste » sur ce poste. Un `status` pour les lecteurs d'écran.
- **La barre d'état** : « Travail · ssh acme-escouade · 38 ms » (la latence d'un aller-retour), ou la reprise en ambre.
- **La barre des projets** (B) : l'espace ouvert au-dessus de ses projets ; les autres en piles sous le « + », qui y mènent (voir *Les espaces*).
- **Le pied de la barre latérale** : « Travail : » devant le dossier du projet, l'icône d'un serveur.
- **L'en-tête d'un terminal** : « Travail · escouade@vps.example.net ».
- **« Nouveau projet »** : « sur Travail · escouade.acme.internal » et « Les dossiers sont ceux de Travail, pas de ce poste. » ; le sélecteur de dossier du système n'y paraît pas (le chemin se tape ; le navigateur des dossiers du serveur viendra).

## Revenir sur ce poste, Réglages › Serveurs

- Aller d'une machine à l'autre recharge la fenêtre sur l'autre cœur. L'app rouvre sur la dernière machine où était la fenêtre, les tunnels de tous les serveurs connus relancés.
- **Réglages › Serveurs** (une page de l'app, sur ce poste) : les serveurs connus, chacun avec sa plaque, son hôte, son état (« Connecté » dès que sa connexion est ouverte, qu'il soit l'espace ouvert ou une pile, « Reconnexion », « Pas connecté », « Injoignable », « Clé d'hôte inconnue », « Pas d'Escouade », « Version différente », « Jeton révoqué », « Refusé par la stratégie ») et son geste (« Ouvrir », « Réessayer », « Copier la commande », « Appairer de nouveau », « Aller sur Ce poste ») ; « Retirer », après confirmation.
- **Retirer un serveur** : le serveur est prié de révoquer ce poste s'il est joignable (le refus suivant dira « depuis » ce poste), puis le jeton et la ligne sont oubliés ici. Les agents du serveur continuent ; revenir demandera un nouveau code.

## Ce que le serveur donne au client

- Le message d'accueil de l'API porte la machine du serveur (son nom et le compte du service).
- `ping` et sa réponse, aussitôt : la latence.
- Le refus d'un jeton porte la version du serveur, et, pour un appareil révoqué, son nom, la date et qui l'a révoqué (le serveur garde une trace des 32 derniers appareils révoqués : nom, date, auteur et empreinte du jeton, jamais le jeton).
- Le refus d'un code dit pourquoi : aucun code n'attend, expiré, faux, trop d'essais.
- Un appareil peut demander sa propre révocation (« Retirer »).

## La sécurité

- Un serveur est une autre machine : le client n'exécute rien de ce qu'il dit ; ses textes (et ceux de `ssh`) sont montrés comme des données.
- La politique de contenu de la fenêtre ne lui permet que les WebSockets de `127.0.0.1`.
- Les stratégies imposées du poste valent : `servers.allowed` (des hôtes ou des domaines, `*.acme.internal`) refuse un serveur avant toute connexion, à l'ouverture d'un serveur connu comme au démarrage de l'app.
