---
title: "Ce qui change de machine"
subject: "a-distance"
sources: ["docs/REMOTE.md, « Ce qui change de machine », « Ce qui change de machine, MS-4 (ESC-538) »","docs/SERVER-UX.md, § 4, § 7 (S12, S13), § 9, § 10","design/server-2-3/ (captures x-fichier, x-compte, x-notifs, *-projet)","src-tauri/src/serverfolders.rs, src-tauri/src/servers/received.rs, src-tauri/src/servers/mod.rs (relay)","src/lib/server-moves.svelte.ts, src/lib/server-browse.ts, src/components/servers/","Revue de sécurité de la 2.3 (ESC-624), constat SR1-06 (rapport privé, hors du dépôt)"]
tickets: ["ESC-538","ESC-629","ESC-651"]
decisions: [217,284,299,309]
compared: null
---

# Ce qui change de machine

Quand la fenêtre est sur un serveur (voir *Se connecter à un serveur*), le cœur, ses agents et leurs fichiers sont sur le serveur ; l'utilisateur, son navigateur, son micro, son presse-papiers et ses dossiers sont sur le poste. Cette page dit, ligne par ligne, ce que fait chaque chose qui changerait de sens d'une machine à l'autre. **Sur le poste, rien ne change.**

La règle commune : **rien de ce que le serveur envoie ne s'ouvre ni ne s'exécute sur le poste sans un geste de l'utilisateur** ; le poste n'écoute que sur son 127.0.0.1 ; ce qui part du poste vers le serveur est dit là où il part.

## « Nouveau projet » sur un serveur

- L'assistant dit en tête : « Les dossiers sont ceux de Travail (escouade.acme.internal), pas de ce poste : le projet et ses agents vivront sur le serveur. » Le sélecteur de dossier du système, celui du poste, n'apparaît jamais.
- Deux sources, dans cet ordre : **« Cloner un dépôt »** (choisie par défaut : le cas courant) et **« Un dossier du serveur »**. Le chemin reste aussi à taper.
- **Cloner un dépôt** : l'adresse (https ou ssh seulement : jamais un chemin local, `file://`, un autre transport, une option de git ou un identifiant dans l'adresse), le dossier du serveur où il va (celui que montre le navigateur, par défaut `~/projets`), le nom du dossier (celui que git donnerait). Le serveur clone avec **ses propres clés et identifiants git** : rien ne passe par le poste. Un dossier déjà plein est refusé. Le clone fait, l'assistant continue avec ce dossier.
- **Un dossier du serveur** : un navigateur servi par le cœur du serveur. Il s'ouvre sur `~/projets` (ou `~/projects`, `~/dev`, `~/src`, `~/code`), sinon sur le dossier personnel du compte du service. Chaque ligne dit :
  - si le dossier est un dépôt git, et sa branche (ou le début de son commit, détaché) ;
  - s'il est **déjà un projet** : grisé, « Déjà un projet », il ne se choisit pas une seconde fois ;
  - sinon « dossier ».
- Le pied dit **la place libre du disque du serveur** (« Le disque du serveur : 41 Go libres sur 80 Go »). Un clic choisit un dossier, un double clic (ou Entrée) y entre ; « Dossier parent », « Dossier personnel », « Choisir ce dossier » (le dossier montré lui-même).
- **Les bornes** : des dossiers seulement (jamais le nom ni le contenu d'un fichier), les dossiers cachés laissés de côté, 500 au plus (le reste est compté : « 12 dossiers de plus, non listés : tape le chemin »), un chemin absolu que le compte du service peut lire ; sinon l'erreur le dit. Un appareil appairé a déjà le shell du serveur (ses terminaux) : le navigateur ne lui donne rien de plus.

## Les fichiers envoyés au serveur

- **Le logo d'un projet** : l'image est choisie et lue sur le poste ; ses octets sont envoyés au serveur (corps brut de l'API distante), jamais son chemin. Le serveur la vérifie comme un fichier (format lu dans les octets, 1 Mo au plus).
- **Les fichiers joints** aux messages, aux tickets et aux réponses de la boîte « À toi » : toujours une copie envoyée en morceaux d'octets bruts. « Joindre le fichier d'origine » ne vaut pas sur un serveur : un fichier du poste n'est jamais là où sont les agents.
- **Coller une image** : le presse-papiers est lu sur le poste, l'image envoyée comme une image collée.

## Les fichiers des agents, reçus sur le poste

Ce qu'un agent envoie (l'outil `SendUserFile` de Claude Code, une image qu'un message montre par son chemin, un fichier d'une demande de la boîte « À toi ») est sur le serveur. Un serveur, ou un agent du serveur égaré par le contenu d'un dépôt, d'une issue ou d'un plugin, peut envoyer n'importe quel fichier : le poste n'ouvre donc **que ce qui est sur une liste**, jamais d'après une liste de ce qui serait dangereux.

**Ce qui s'ouvre** (le nom et les octets disent le même type) :

| Type | Comment il s'ouvre |
| --- | --- |
| Image PNG, JPEG, GIF, WebP | La copie du poste, avec ce qui lit les images sur le poste |
| PDF | La copie du poste, avec le lecteur de PDF du poste |
| Texte brut, Markdown, CSV, JSON (`.txt`, `.md`, `.markdown`, `.csv`, `.json`, des octets de texte) | Dans l'éditeur d'Escouade, sur le serveur ; jamais confié au système du poste (un tableur calculerait les formules d'un CSV) |

**Tout le reste ne s'ouvre jamais d'Escouade** : programmes et scripts d'abord, mais aussi consoles, aides, images disque, bibliothèques et recherches de l'Explorateur, installeurs, compléments, pages web et SVG, raccourcis et liens de tous les systèmes, documents bureautiques, archives, fichiers sans extension. Leur seul bouton est « Télécharger sur ce poste » : la copie est faite puis montrée dans son dossier (« Afficher dans le dossier »), et la carte dit pourquoi : « Escouade n'ouvre que les images, les PDF et les textes : ce fichier s'affiche dans son dossier » ; pour un programme ou un script : « un script ne s'ouvre pas d'Escouade : il se télécharge seulement ». Le message qui suit la copie le redit avec son dossier. Est un programme ou un script :

- ce que disent ses octets (un exécutable Windows, Linux ou macOS, un `#!`), quel que soit son nom ;
- ce que dit son nom (`.exe`, `.ps1`, `.sh`, `.bat`, `.msi`, `.app`, `.desktop`…), quels que soient ses octets.

Un nom de la liste dont les octets disent autre chose (un « texte » qui contient un programme ou un octet nul, un « PDF » qui n'en est pas un) n'est pas sur la liste. Le cœur du poste en décide sur les octets de la copie, à chaque ouverture, quoi que la fenêtre ait montré avant le téléchargement.

- **« Ouvrir »** (une image, un PDF) télécharge le fichier sur le poste **au moment où on l'ouvre** (jamais d'avance), dans **`Téléchargements/Escouade/<espace>`**, puis ouvre la copie. La carte dit alors « envoyé depuis Travail · copié sur ce poste dans … », un badge « Téléchargé », et propose « Ouvrir la copie » et « Afficher sur ce poste ». Une seconde ouverture rouvre la copie, sans la télécharger de nouveau.
- **« Ouvrir dans l'éditeur »** (un texte) ouvre le fichier du serveur dans l'éditeur d'Escouade ; « Télécharger sur ce poste » en fait une copie, montrée dans son dossier.
- **Le type lu dans le contenu** : une image ou un PDF garde le type que disent ses octets ; un nom qui en dit un autre, ou aucun, reçoit la bonne extension (`capture` → `capture.png`), pour que le système l'ouvre avec ce qui lit ce type.
- **La copie est marquée comme venue d'ailleurs**, comme un téléchargement du web : sous Windows son `Zone.Identifier` (zone Internet), sous macOS l'attribut de quarantaine (Gatekeeper demande avant que quoi que ce soit la lance). Elle n'est jamais exécutable (sous macOS et Linux, sans droit d'exécution, quel que soit l'umask) ; Linux n'a pas d'autre marque. Un nom ou un espace ne mène jamais hors de `Téléchargements/Escouade`. Un fichier de plus de 512 Mo reste sur le serveur, et c'est dit.
- **« Voir sur le serveur »** montre le fichier dans le panneau Fichiers (l'éditeur), sur le serveur.
- La visionneuse d'images et les fichiers d'une demande de la boîte « À toi » suivent la même liste : « Ouvrir » et « Afficher dans le dossier » agissent sur la copie du poste ; un fichier hors de la liste (un `.docx` joint à une demande) est seulement montré dans son dossier, et un message dit pourquoi.

## « Afficher dans l'Explorateur »

- « Ouvrir le dossier » d'un projet (menu du projet, Réglages › Projet) ouvre, sur un serveur, le **panneau Fichiers** du projet : le dossier est celui du serveur.
- Les dossiers du serveur qui n'ont rien à faire sur le poste (le dossier des données, celui des copies) n'ont pas de bouton « Ouvrir » sur un serveur. Un chemin du serveur n'est jamais ouvert sur le poste : c'est dit si on le demande.

## Les ports des serveurs de test

- La page d'un lancement (« Lancement ») et tout lien vers l'adresse de bouclage du serveur (`localhost`, `127.0.0.1`, `0.0.0.0`) s'ouvrent **sur un port du poste relayé vers le 127.0.0.1 du serveur**, WebSocket du rechargement à chaud comprise. Le même numéro quand il est libre sur le poste, sinon un autre ; l'adresse ouverte est celle du poste.
- Le relai passe par l'hôte SSH du tunnel : une redirection de plus (`ssh -N -L 127.0.0.1:<port du poste>:127.0.0.1:<port du serveur>`), avec les mêmes options sûres que le tunnel (rien demandé, aucune clé acceptée à la place de l'utilisateur). Il n'écoute que sur le 127.0.0.1 du poste. Il dure tant que la fenêtre est sur ce serveur, et s'arrête quand on revient sur ce poste ou qu'on quitte l'app.

## Les notifications, le son et le clignotement

- Le cœur du serveur ne notifie rien lui-même : ce qu'il montrerait (le carillon, le clignotement de la barre des tâches, la notification système, le compte de ce qui attend dans le tray) est poussé au client, qui le rend sur le poste. Les choix des notifications (Réglages › Notifications) sont ceux du serveur ; le poste ajoute la règle du bureau : pas de clignotement ni de notification quand on regarde la fenêtre.
- Une notification d'un serveur porte **le nom de l'espace** en tête : « Travail · facturation · relances a fini son tour », sous l'en-tête Escouade du système.
- Un clic sur la notification fait ce qu'il ferait sur le bureau (l'agent montré) : le poste le demande au serveur, qui garde l'action de ses 64 dernières notifications.
- Ce que le serveur demanderait d'ouvrir (un lien, un de ses fichiers) n'est jamais ouvert sur le poste.
- **Après une coupure** : « Reconnecté à Travail · 12 événements rattrapés » (le nombre d'événements de l'état émis pendant la coupure ; « rien de neuf » sans aucun) ; l'écran reprend l'état entier.

## La voix

- La voix est **celle du poste** : son micro, ses haut-parleurs, sa transcription (Whisper sur le poste, ou le service en ligne choisi, avec la clé gardée dans le trousseau du poste). Seul le texte part vers le serveur.
- Réglages › Voix, sur un serveur, montre et enregistre **les réglages du poste**, et le dit : « La voix est celle de ce poste : son micro, sa transcription et ses clés. Seul le texte part vers Travail. »

## La connexion d'un compte de harness sur le serveur

Le harness tourne sur le serveur, où aucun navigateur ne s'ouvre. Dans la fenêtre de connexion du compte, au-dessus de son terminal :

1. Dès que le terminal donne l'adresse de la page de connexion : « La page de connexion est prête : elle s'ouvre dans le navigateur de ce poste. » La page s'ouvre **au clic** sur « Ouvrir la page sur ce poste » (jamais d'elle-même) ; « Copier l'adresse » la copie.
2. Selon le retour que la page attend :
   - **le retour par `localhost`** (le paramètre `redirect_uri` de la page, ou le serveur local que le harness a dit) : le poste écoute **ce même port** sur son 127.0.0.1 et le relaie au serveur ; « Le retour par localhost:1455 est relayé à Travail : finis la connexion dans la page, rien d'autre à faire. » Si ce port est pris sur le poste, c'est dit, et il reste à coller le code ;
   - **un code à coller** : « Si la page affiche un code à la fin, colle-le dans le terminal ci-dessous » ;
   - **un code d'appareil** (GitHub) : « Tape sur la page le code que le terminal affiche ci-dessous ».
3. Toujours : « Le compte sera gardé sur Travail, pas sur ce poste. »

Harness par harness, d'après leurs flux de connexion (aucun compte réel n'a été connecté pour l'écrire ; un faux harness le vérifie) :

| Harness | Sa connexion | Sur un serveur |
| --- | --- | --- |
| Codex (`codex login`) | Un serveur local sur `localhost:1455`, son `redirect_uri` dans la page | Relayé sur le même port |
| Gemini CLI, Antigravity CLI | Le compte Google, retour sur un port de `127.0.0.1` dit dans la page | Relayé sur le même port |
| Claude Code (`/login`) | Retour par `localhost` quand son navigateur s'ouvre ; sinon une adresse dont la page affiche un code à coller | Le code à coller (sa page de secours) |
| Copilot CLI (`copilot login`) | Le code d'appareil de GitHub | Le code tapé sur la page |
| Cursor CLI (`cursor-agent login`) | La page, puis le CLI attend que le compte soit lié | La page ouverte sur le poste suffit ; ou sa clé d'API |
| OpenCode (`opencode auth login`) | Selon le fournisseur : une clé, un code à coller, un code d'appareil | Selon le fournisseur, comme ci-dessus |
| Mistral Vibe | Sa clé d'API | La clé, gardée sur le serveur |

## Les secrets sur un serveur

Là où un secret se donne (la clé de Cursor ou de Mistral Vibe, les jetons des services connectés, les secrets des plugins), la fenêtre le dit quand le serveur les garde en fichier : « **Gardée dans un fichier sur Travail** · ~/.escouade/secrets.json, lisible du seul compte du service : un serveur n'a pas de trousseau. Pas dans le trousseau de ce poste. » Les clés de la voix en ligne restent au trousseau du poste (la voix est celle du poste).

## Ce qui ne change pas, ou pas encore

- **Le serveur MCP** reste sur le 127.0.0.1 du serveur, où ses agents l'atteignent ; un Claude Code du poste qui piloterait Escouade par le tunnel viendra plus tard.
- **WSL** est sans objet sur un serveur Linux : ses réglages n'y apparaissent pas.
- **La mise à jour** : le client et le serveur, chacun la sienne. Le serveur a sa mise à jour signée (`escouade-server update`, ou une nouvelle image Docker), et la poignée de main dit comment le mettre à la version du client (voir *Le mode serveur*) ; l'écran « Mettre à jour le serveur » du client reste à faire.
- **Les serveurs de langage de l'éditeur** sont encore refusés à distance.

## L'ouverture bornée, sur le poste comme sur un serveur (ESC-651)

Ce qui suit réunit l'ouverture bornée de la 2.2.6 (audit SEC-12, *L'ouverture par le système*) et les hôtes de la 2.3.

- **Le cœur nomme les dossiers permis, l'hôte juge** : chaque chemin qu'Escouade fait ouvrir ou montrer (« Ouvrir », « Afficher dans le dossier », un fichier envoyé par un agent, l'image d'un message, un fichier d'une demande de la boîte « À toi ») part avec les dossiers où il peut être : ceux des projets, de leurs worktrees, des agents, et le dossier de données. Le chemin est jugé là où il est vraiment (liens, jonctions, `..` résolus) avant que rien ne s'ouvre.
- **Sur le poste**, l'app l'ouvre ensuite. **Sur un serveur**, le serveur le juge sur sa machine avant d'en dire quoi que ce soit au client : un chemin hors de ces dossiers est refusé avec sa raison, et le client n'en apprend rien.
- **Les copies reçues d'un serveur** ne s'ouvrent ou ne se montrent que dans `Téléchargements/Escouade` : c'est le seul dossier permis pour elles. La copie reçue est marquée en quarantaine sous macOS dès qu'elle est écrite, comme un fichier qu'un agent envoie.
- **Les octets** qu'un client télécharge d'un serveur (un fichier envoyé, une image d'un message) sont lus comme un fichier ordinaire seulement : jamais un tube nommé ni un périphérique, et 512 Mo au plus.
