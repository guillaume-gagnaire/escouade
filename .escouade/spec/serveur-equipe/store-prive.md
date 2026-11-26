---
title: "Le store de plugins privés"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §3.7, §6.4, §6.5, §7, §9, §15.2 (C13), §16 (TS-14)","docs/STORE.md §4.5, §4.6 (la liste de révocation)","crates/escouade-team/src/signing.rs, crates/escouade-team/src/plugins.rs","crates/escouade-plugin-cli/src/team.rs","crates/escouade-plugins/src/team.rs","src-tauri/src/teamplugins.rs, src-tauri/src/teams_plugins.rs"]
tickets: ["ESC-579","ESC-648"]
decisions: [278,334]
compared: null
---

# Le store de plugins privés

Les plugins internes d'une équipe, publiés sur son serveur, **signés par la clé de l'équipe**, installables par ses seuls membres. Leur format est celui du store public : tout en WASM, sans binaire natif. Le serveur est aveugle (C10-A) : il garde des fichiers chiffrés et des entrées scellées, et ne contrôle rien. Les contrôles tournent sur les postes.

## La clé de l'équipe (C13-A)

- **Une paire minisign à part** (Ed25519, la forme pré-hachée de minisign), avec **une clé de secours**, tirées sur le poste d'un administrateur, **jamais sur le serveur**. Ce sont de vraies clés minisign : leurs signatures se vérifient avec `minisign` lui-même.
- **Gardée hors ligne, sous un mot de passe** (12 caractères au moins) : la clé de tous les jours reste scellée sur le poste de l'administrateur, dans le dossier de données d'Escouade (hors du dossier de l'équipe : oublier l'équipe ne la perd pas) ; la clé de secours, scellée par le même mot de passe, est écrite dans le dossier qu'il choisit (une clé USB), à garder ailleurs. Le mot de passe ne quitte jamais le poste ; aucune moitié secrète n'est écrite en clair.
- **Déclarée à l'équipe** : ses deux clés publiques et **son espace de noms** (par exemple `fr.acme.*`), par une entrée du journal de l'équipe signée d'un administrateur. Chaque poste refuse une déclaration faite par qui n'était pas administrateur.
- **Un autre administrateur** importe le fichier scellé (avec son mot de passe) pour signer à son tour : il n'est gardé que s'il s'ouvre et que c'est une clé de confiance de l'équipe.
- **L'espace de noms** est un domaine à l'envers de deux parties au moins, suivi de `.*`. Jamais `dev.escouade.*`, ni un espace qui le contient (`dev.*`).

## Les refus en dur de l'app

Joués deux fois : par le client de l'équipe à la lecture de son flux, puis par l'app avant d'écrire quoi que ce soit du plugin.

- une signature de l'équipe pour un id **hors de son espace** ;
- **jamais `dev.escouade.*`**, quoi que dise l'espace ;
- **jamais un espace qui contient un plugin installé d'une autre origine** : un tel plugin sur le poste bloque le store de l'équipe sur ce poste (sa fenêtre le dit), et l'espace ne peut pas être déclaré depuis un poste qui en a un ;
- **un plugin privé ne remplace jamais un plugin du même id venu d'ailleurs**, ni l'inverse : l'identité d'un plugin installé est son origine plus son id. Sa page dit « Acme · privé ».
- une signature d'une clé qui n'est pas de confiance (non déclarée, ou révoquée par la liste de l'équipe), ou pour une autre version, une autre équipe, un autre espace ;
- une archive ou un module qui ne sont pas ceux que l'énoncé signé nomme (par leurs SHA-256).

## La publication, contrôlée par un client

- **Qui propose** : n'importe quel membre, depuis la fenêtre des équipes, « Plugins » › « Publier un plugin » (un dossier, une étiquette). Son poste joue les contrôles du store public :
  - l'archive saine : pas de lien, pas de `..`, pas de chemin absolu, 50 Mo au plus (200 Mo décompressée), ni sous-module ni pointeur Git LFS ;
  - le manifeste et ce que dit `escouade-plugin validate`, pour la main de n'importe qui ;
  - la version égale à l'étiquette (`v1.2.0` ou `1.2.0`) et plus grande que la dernière publiée par l'équipe ;
  - les permissions cohérentes avec les contributions ;
  - aucun binaire : un programme natif est refusé ;
  - **la compilation, deux fois**, chacune dans un dossier neuf : les deux modules doivent être les mêmes octets.

  Rien ne part si un contrôle échoue ; la fenêtre dit lequel. Sinon, l'archive et le module partent comme **deux fichiers chiffrés par la clé d'équipe** sur le serveur, avec la proposition et son **empreinte** (celle de l'archive et du module ensemble).
- **Qui signe** : un administrateur dont le poste garde la clé, « Contrôler à nouveau et signer ». Son poste relit l'archive sur le serveur, **rejoue les mêmes contrôles** et ne signe, mot de passe donné, **que si son empreinte est celle de la proposition**. Sinon : « Les empreintes diffèrent… Rien n'est signé. »
- **La compilation tourne sur le poste de l'administrateur, et c'est dit** (ESC-648) : rejouer les contrôles compile le plugin d'un membre ici (rustc, les `build.rs` et macros natives de la liste fermée, ce que son code lit du disque à la compilation : `include_bytes!`, `include_str!`, `env!`). Ce n'est pas le bac à sable du store public : un plugin privé n'y va pas. Le formulaire de signature le dit par une case à cocher : « Je sais que les contrôles compilent ici le plugin de qui l'a proposé : rustc, ses build.rs et macros de la liste fermée, et ce que son code lit du disque à la compilation (include_bytes!, env!). Ce poste peut le compiler. » ; « Contrôler à nouveau et signer » reste grisé tant qu'elle n'est pas cochée, et **le cœur refuse de signer sans elle** (`team_plugin_sign`, `compileHere`) : « Signer compile ici le plugin de qui l'a proposé : confirme d'abord que ce poste peut le compiler. Rien n'est signé. » Ce qu'une compilation lit du disque ne sort pas tant que les empreintes des deux postes doivent être égales ; la sortie de rustc (4 000 caractères au plus) est montrée dans le rapport.
- Ce qui est signé est l'**énoncé** de la version (son id, sa version, son étiquette, son nom, l'espace, l'empreinte de l'équipe, les SHA-256 de l'archive et du module, l'empreinte des contrôles), avec un commentaire de confiance propre au store privé, jamais celui du store public.
- Les mêmes contrôles se jouent sans l'app : `escouade-plugin team-check <dossier> --tag <étiquette> [--last <version>]`.
- Une proposition s'écarte, par un administrateur ou par son auteur.
- Le serveur ne lit ni l'id d'un plugin, ni son nom, ni son archive.

## La liste de révocation de l'équipe

- **Signée par la clé de l'équipe**, au format de la liste du store (`revocations.json`), avec un `serial` qui **ne recule jamais** : une liste plus ancienne, ou une autre sous le même numéro, est refusée à la lecture et la liste prise reste.
- Un administrateur retire une version d'un plugin de l'équipe (« Retirer… », une raison, le mot de passe de la clé) : la liste suivante porte le numéro d'après.
- **Appliquée à chaque lecture de l'équipe, aux seuls plugins de cette équipe**, comme la liste du store aux siens : une version retirée est désactivée et son programme arrêté, sa ligne dit « Retiré par l'équipe : <raison> », et elle ne s'installe plus. C'est elle que lit le coupe-circuit des plugins (§9, TS-15).
- **La clé de secours prend la suite** d'une clé de tous les jours perdue : une liste signée par la clé de secours peut révoquer l'autre (`keys_revoked`) ; une liste qui révoque la clé qui la signe est refusée.

## La politique d'installation (§6.4)

- **Les origines permises** (`plugins.allowedOrigins`, dont `team-store`), **les ids permis** (`plugins.allowed`) **ou interdits** (`plugins.denied`) : les clés des stratégies du poste et du `.escouade.yml` d'équipe (*Réglages imposés*, *Le .escouade.yml d'équipe*). Celles de l'équipe valent, comme ses autres réglages de l'app, pour tout ce qui s'installe tant que le poste est dans l'équipe ; ce qui vient de son store repasse en plus par les siennes seules.
- **Ce qu'un membre installe seul, c'est ce que ces bornes permettent.** Il n'y a pas d'autre installation : ni par l'équipe, ni par un administrateur. L'équipe propose, elle n'installe jamais (§6.5).
- **Installer reste le geste du membre** : la vue Plugins, « Équipe », montre les plugins de chaque équipe (« Acme · privé », l'espace, la version, retiré ou installé) ; « Installer » relit la version sur le serveur, la vérifie de nouveau, puis ouvre **la fenêtre de ses permissions** (« Signé par la clé de l'équipe Acme »). Rien n'est installé avant son accord.

## Le MCP

Aucun outil du serveur MCP ne touche au store privé : **un agent ne publie ni ne signe** un plugin d'équipe, ne crée pas la clé et ne retire rien. Les commandes sont celles de la fenêtre.

## Limites

- L'archive et le module sont chacun un fichier du serveur, borné par sa limite des fichiers (25 Mo d'office, `--max-file`) : une archive plus grosse ne part pas, même sous les 50 Mo du contrôle.
- Une nouvelle version publiée est offerte par la page « Équipe » de la vue Plugins, pas encore par « Mises à jour ».
- Le coupe-circuit de TS-15 n'est pas encore là : la liste de l'équipe s'applique par sa lecture.
- La compilation d'un plugin d'équipe n'a pas de bac à sable sur le poste de l'administrateur : elle est dite et consentie, pas isolée.
