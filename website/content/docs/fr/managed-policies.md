# Stratégies du poste

Un service informatique peut imposer des réglages d’Escouade sur les postes qu’il gère, par ses propres outils : les stratégies de groupe ou Intune sous Windows, un profil de configuration sous macOS, des fichiers de configuration sous Linux.

> [!NOTE]
> Cette page est générée depuis la liste des stratégies d’Escouade (`crates/escouade-policy/src/list.rs`), comme les fichiers qu’elle cite. Aucun serveur d’équipe n’est nécessaire.

## Ce que fait une stratégie

- Une stratégie posée vaut pour toute l’app, dans tous les projets. Elle passe devant le réglage du poste, le `.escouade.yml` du dépôt et ce qu’une équipe recommande.
- Le réglage qu’elle verrouille est grisé, avec un cadenas et la ligne « Imposé par la stratégie de ce poste ».
- C’est le cœur d’Escouade qui refuse, pas seulement la fenêtre : une écriture par le serveur MCP ou par le `.escouade.yml` du dépôt est refusée aussi, avec la raison.
- Chaque stratégie a sa sorte :
  - **restreindre** : elle ne s’impose que dans un sens (les statistiques éteintes, jamais allumées). Une autre valeur est ignorée et dite ;
  - **valeur** : elle pose une valeur (le mode par défaut, l’adresse du serveur d’équipe) ;
  - **sensible** : l’adresse d’un proxy, un fichier d’autorités. Posée par le poste géré, elle s’applique ; venue d’une équipe, elle attendra l’accord de chaque membre.
- Une stratégie changée pendant que l’app tourne s’applique en quelques secondes, sans redémarrer. Un agent dans un mode devenu interdit finit son tour, puis passe au mode permis le plus proche, et le dit dans sa conversation. Un agent d’un harness devenu interdit s’arrête à la fin de son tour.

## Les stratégies

Chaque stratégie a une clé, le même nom partout : la valeur du registre, la clé du profil, la clé du fichier JSON. Un booléen se donne `true` ou `false` (`1` ou `0` dans le registre). Une liste est une liste ADMX, un tableau du profil ou du JSON ; une chaîne seule vaut une liste d’un élément. Ce qui est « à venir » est lu et gardé, et s’appliquera avec la fonction qui le porte.

### Modes de permission

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Interdire le mode Bypass | `permissions.allowBypass` · booléen `false` (`0`) · restreindre | Le mode où un harness ne demande rien (bypassPermissions) n’est plus permis à aucun agent. Seule la valeur false (0) s’impose. | Réglages › Agents › « Autoriser le mode Bypass », éteint et grisé ; le mode Bypass refusé partout. |
| Modes de permission permis | `permissions.modes` · liste parmi `plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions` · restreindre | La liste des modes que les agents peuvent prendre : plan, default, acceptEdits, auto, bypassPermissions. Un agent dans un autre mode finit son tour, puis passe au mode permis le plus proche. | Les modes non permis grisés dans « Mode par défaut des agents », le mode du Kanban et le mode d’un agent. |
| Mode par défaut des agents | `permissions.defaultMode` · chaîne `plan`, `default`, `acceptEdits`, `auto` · valeur | Le mode des nouveaux agents : plan, default, acceptEdits ou auto. Il reste dans les modes permis : hors d’eux, c’est le plus proche permis. | Réglages › Agents › « Mode par défaut des agents », grisé. |

### Harness et modèles

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Harness permis | `harnesses.allowed` · liste · restreindre | La liste des harness que les agents peuvent utiliser, par leur id (claude, codex, copilot, cursor, opencode, gemini…). Un agent d’un harness devenu interdit s’arrête à la fin de son tour. | Le harness d’un projet, d’un agent, d’un ticket : un harness non permis est grisé et refusé. |
| Modèles permis par harness | `harnesses.allowedModels` · JSON · restreindre | Un objet JSON qui donne, pour chaque harness, la liste de ses modèles permis. Un harness absent de l’objet garde tous ses modèles. | Un agent ou un ticket sur un modèle non permis est refusé à son démarrage. |
| Effort au plus | `harnesses.maxEffort` · chaîne `low`, `medium`, `high`, `xhigh`, `max` · restreindre | L’effort le plus haut qu’un agent peut recevoir : low, medium, high, xhigh ou max. | L’effort des agents du Kanban ramené à ce plafond ; un effort plus haut refusé. |

### Plugins

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Origines des plugins permises | `plugins.allowedOrigins` · liste parmi `store-verified`, `store`, `team-store`, `git`, `folder`, `dev` · restreindre | D’où un plugin peut s’installer : store-verified (le store, versions vérifiées), store (le store, toute version), team-store (le store de l’équipe), git, folder (un dossier), dev (un dossier en développement). | L’installation d’un plugin d’une autre origine est refusée et dite. |
| Plugins permis | `plugins.allowed` · liste · restreindre | La liste des ids de plugins qui peuvent s’installer. Absente, tous le peuvent. | L’installation d’un plugin hors de la liste est refusée et dite. |
| Plugins interdits | `plugins.denied` · liste · restreindre | La liste des ids de plugins qui ne s’installent jamais, même permis par ailleurs. | L’installation d’un plugin de la liste est refusée et dite. |
| N’installer que des versions vérifiées | `plugins.verifiedOnly` · booléen `true` (`1`) · restreindre | Une version du store sans le sceau « Vérifié par Escouade » est refusée, à l’installation comme à la mise à jour. Seule la valeur true (1) s’impose. | Plugins › « N’installer que des versions vérifiées », allumé et grisé. |
| Interdire le fichier des secrets des plugins | `secrets.fileFallback` · booléen `false` (`0`) · restreindre | Sur un poste sans trousseau système, le secret d’un plugin ne peut plus être gardé dans un fichier lisible de son seul utilisateur : il n’est gardé que par le trousseau. Un fichier déjà choisi n’est plus lu ; ses secrets se déplacent vers le trousseau ou s’effacent avec lui. Seule la valeur false (0) s’impose. | « Garder dans un fichier lisible de toi seul » n’est plus proposé au champ d’un secret, et Plugins › « Secrets gardés dans un fichier » porte le cadenas. |

### Éditeur

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Extensions permises de l’éditeur | `extensions.allowed` · JSON · restreindre | Un objet JSON comme extensions.allowed de VS Code : ses clés sont * (toutes), un éditeur d’Open VSX ou une extension (éditeur.nom), ses valeurs true, false, stable (pas de préversion) ou la liste des versions permises. La clé la plus précise dit. Un objet qui nomme quelque chose ne permet que ce qu’il nomme. Il s’ajoute aux listes des réglages et des fichiers .escouade.yml : une extension doit être permise par chacune. | Général › « Extensions permises de l’éditeur », grisé ; une extension hors de la liste est refusée avant tout téléchargement, et une extension installée qu’elle exclut n’est plus chargée. |

### Fonctionnalités

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Fonctionnalité Kanban | `features.kanban` · booléen `true`, `false` · valeur | Les tickets, le pilote auto et les sources de tickets : true l’allume, false la coupe (son onglet, ses outils MCP, son pilote auto). Ses tickets restent. | Réglages › Fonctionnalités › Kanban, grisé. |
| Fonctionnalité Éditeur | `features.editor` · booléen `true`, `false` · valeur | Ouvrir, modifier et rechercher dans les fichiers du projet : true l’allume, false la coupe (la vue de l’éditeur, Ctrl P, ses serveurs de langage). | Réglages › Fonctionnalités › Éditeur, grisé. |
| Fonctionnalité Espace Docs | `features.docs` · booléen `true`, `false` · valeur | La spécification, les décisions, les documents et les discussions : true l’allume, false la coupe (son onglet, ses outils MCP). Ses pages restent. | Réglages › Fonctionnalités › Espace Docs, grisé. |
| Fonctionnalité Coût | `features.cost` · booléen `true`, `false` · valeur | Le coût des tours, des agents et des tickets, et la température du cache : true l’allume, false la coupe. | Réglages › Fonctionnalités › Coût, grisé. |
| Fonctionnalité Statistiques | `features.stats` · booléen `true`, `false` · valeur | L’activité et les dépenses dans le temps : true l’allume, false la coupe (sa vue). Elle n’a rien à voir avec la télémétrie (telemetry.enabled). | Réglages › Fonctionnalités › Statistiques, grisé. |
| Fonctionnalité Workflows | `features.workflows` · booléen `true`, `false` · valeur | Les enchaînements d’agents et de skills d’un projet : true l’allume, false la coupe (son onglet, son outil MCP, leur lecture). | Réglages › Fonctionnalités › Workflows, grisé. |
| Fonctionnalité Tâches programmées | `features.scheduled` · booléen `true`, `false` · valeur | Les tickets qui reviennent à heure fixe : true l’allume, false la coupe (ses tâches ne partent plus). Elle part aussi avec le Kanban. | Réglages › Fonctionnalités › Tâches programmées, grisé. |
| Fonctionnalité Overseer | `features.overseer` · booléen `true`, `false` · valeur | L’agent qui pilote tous les projets : true l’allume, false la coupe (son écran, son raccourci, ses outils MCP ; il s’arrête). | Réglages › Fonctionnalités › Overseer, grisé. |
| Fonctionnalité Voix | `features.voice` · booléen `true`, `false` · valeur | La dictée et la conversation à voix haute : true l’allume, false la coupe (le micro, son raccourci, l’écoute). | Réglages › Fonctionnalités › Voix, grisé. |

### Projets

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Projets privés d’office | `projects.forcePrivate` · booléen `true` (`1`) · restreindre | Chaque projet est privé : ses agents ne voient aucun autre projet, et aucun autre projet ne le voit. Seule la valeur true (1) s’impose. | « Nouveaux projets privés » et « Projet privé » de chaque projet, allumés et grisés. |
| Motifs de fichiers interdits | `projects.deniedFiles` · liste · restreindre | Des motifs de fichiers (à la manière de .gitignore) que les agents ne lisent ni n’écrivent, ajoutés à ceux des projets. | Lu et gardé ; appliqué aux agents avec la clôture des fichiers d’un projet (à venir). |

### Proxy et certificats

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Adresse du proxy | `proxy.url` · chaîne · sensible | Le proxy HTTP(S) de l’app, des plugins et des agents, par exemple http://proxy.example.com:3128. | Réglages › Avancé › Réseau, l’adresse du proxy grisée. |
| Exclusions du proxy | `proxy.exclusions` · liste · sensible | Les hôtes joints sans le proxy (NO_PROXY), un par ligne. | Réglages › Avancé › Réseau, les exclusions grisées. |
| Interdire d’ignorer la vérification TLS | `proxy.denyInsecureTls` · booléen `true` (`1`) · restreindre | « Ignorer la vérification des certificats TLS » est éteint et ne s’allume plus. Seule la valeur true (1) s’impose. | Réglages › Avancé › Réseau › « Ignorer la vérification des certificats TLS », éteint et grisé. |
| Fichier d’autorités de certification | `proxy.authorities` · chaîne · sensible | Le chemin d’un fichier .pem d’autorités crues en plus du magasin du système (la racine d’un proxy d’entreprise). | Réglages › Avancé › Réseau › « Autorités de certification supplémentaires », grisé. |

### Télémétrie et voix

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Éteindre les statistiques d’usage | `telemetry.enabled` · booléen `false` (`0`) · restreindre | Les statistiques anonymes ne partent plus. Seule la valeur false (0) s’impose. | Réglages › Général › « Statistiques d’usage », éteint et grisé. |
| Interdire les voix en ligne | `voice.onlineAllowed` · booléen `false` (`0`) · restreindre | La dictée et la voix de la conversation restent sur le poste (Whisper et Piper) : aucun service en ligne. Seule la valeur false (0) s’impose. | Réglages › Voix, « En ligne » ramené au local (Whisper, Piper) et refusé. |
| Éteindre la dictée | `voice.dictation` · booléen `false` (`0`) · restreindre | La dictée ne démarre plus. Seule la valeur false (0) s’impose. | Réglages › Voix › Dictée, grisé ; la dictée refusée. |

### Remote Control

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Interdire le Remote Control | `remoteControl.enabled` · booléen `false` (`0`) · restreindre | Aucun agent n’est joignable depuis claude.ai. Seule la valeur false (0) s’impose. | « Remote control pour tous les agents », celui d’un projet et d’un agent, éteints et grisés. |

### Mises à jour

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Canal stable seulement | `updates.channel` · chaîne `stable`, `beta` · restreindre | Aucune version bêta n’est proposée. Seule la valeur stable s’impose. | Réglages › Général › « Recevoir les versions bêta », éteint et grisé. |
| Couper la mise à jour automatique | `updates.automatic` · booléen `false` (`0`) · restreindre | Une nouvelle version est proposée, jamais installée seule : le service informatique déploie les siennes. Seule la valeur false (0) s’impose. | Réglages › Général › « Installer les mises à jour automatiquement », éteint et grisé. |

### Serveurs

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Serveurs permis | `servers.allowed` · liste · restreindre | Les adresses des serveurs d’équipe et des serveurs en mode serveur que l’app peut joindre. Absente, tous. | Un serveur hors de la liste est refusé, avec la raison. |
| Interdire l’espace perso | `servers.personalSpace` · booléen `false` (`0`) · restreindre | Seuls les serveurs de la liste des serveurs permis s’ajoutent comme espaces ; sans liste, aucun. Ce poste reste un espace. Seule la valeur false (0) s’impose. | Un serveur hors de la liste est refusé avant toute connexion, avec la raison. |
| Interdire la vue agrégée | `servers.aggregateView` · booléen `false` (`0`) · restreindre | La vue « Partout » ne montre d’un serveur de travail (de la liste des serveurs permis, ou tout serveur sans liste) que ses nombres, ni texte ni geste. Posée sur un serveur, elle vaut pour lui. Seule la valeur false (0) s’impose. | « Partout » ne donne que les nombres de ces espaces. |
| Notifications sans contenu | `servers.contentlessNotifications` · booléen `true` (`1`) · restreindre | Les notifications venues d’un serveur de travail (de la liste des serveurs permis, ou tout serveur sans liste) disent qu’un agent attend, jamais quoi. Posée sur un serveur, elle vaut pour lui. Seule la valeur true (1) s’impose. | Les notifications de ces espaces n’ont pas de texte. |
| Interdire l’API distante | `servers.remoteApi` · booléen `false` (`0`) · restreindre | L’API distante du cœur (le mode serveur, le téléphone) ne s’allume pas sur ce poste : aucun autre appareil ne le pilote. Seule la valeur false (0) s’impose. | L’API distante refusée à son démarrage et à chaque connexion. |

### Clients MCP extérieurs

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Clients MCP extérieurs | `mcp.externalClients` · chaîne `denied`, `readOnly`, `allowed` · restreindre | Ce que Claude hors d’Escouade (un client MCP extérieur) peut faire par le serveur MCP d’Escouade : denied (rien), readOnly (lire seulement), allowed (tout). Les agents d’Escouade ne sont pas concernés. | Un client extérieur refusé à la connexion (denied), ou ses outils qui écrivent refusés (readOnly). |

### Enrôlement

| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |
| --- | --- | --- | --- |
| Serveur d’équipe | `team.server` · chaîne · valeur | L’adresse du serveur d’équipe de l’entreprise : le membre n’a plus qu’à accepter son invitation. | Gardé par l’app ; l’écran d’entrée dans l’équipe le préremplira (à venir). |
| Empreinte de l’équipe | `team.fingerprint` · chaîne · valeur | L’empreinte de l’équipe attendue sur le serveur d’équipe : une équipe d’une autre empreinte est refusée. | Gardée par l’app ; vérifiée à l’entrée dans l’équipe (à venir). |
| Refuser tout autre serveur d’équipe | `team.onlyThisServer` · booléen `true` (`1`) · restreindre | Seul le serveur d’équipe posé par la stratégie peut être joint. Seule la valeur true (1) s’impose. | Un autre serveur d’équipe est refusé, avec la raison. |

## Déployer

### Windows : stratégies de groupe et Intune

1. Prends `escouade.admx` et ses traductions `fr-FR/escouade.adml` et `en-US/escouade.adml` dans [le dossier policies/windows du dépôt](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/windows).
2. Pose-les dans `C:\Windows\PolicyDefinitions` (avec les dossiers `fr-FR` et `en-US`) pour un poste, ou dans le magasin central de ton domaine (`\\<domaine>\SYSVOL\<domaine>\Policies\PolicyDefinitions`).
3. Dans l’éditeur de stratégie de groupe, les stratégies sont sous Modèles d’administration › Escouade, côté ordinateur comme côté utilisateur.
4. Dans Intune : Appareils › Configuration › Importer un ADMX, puis un profil de type Modèles importés.

Escouade lit `HKLM\SOFTWARE\Policies\Escouade`, puis `HKCU\SOFTWARE\Policies\Escouade` ; pour une même clé, la machine gagne. Sans l’ADMX, un script peut écrire le registre lui-même : la valeur porte le nom de la clé, un booléen est un `REG_DWORD` à `0` ou `1`, une chaîne un `REG_SZ`, un réglage JSON un `REG_SZ` qui commence par une accolade, et une liste une sous-clé du nom de la clé dont les valeurs s’appellent `1`, `2`, `3`…

```bat
reg add "HKLM\SOFTWARE\Policies\Escouade" /v telemetry.enabled /t REG_DWORD /d 0 /f
reg add "HKLM\SOFTWARE\Policies\Escouade\permissions.modes" /v 1 /t REG_SZ /d plan /f
reg add "HKLM\SOFTWARE\Policies\Escouade\permissions.modes" /v 2 /t REG_SZ /d default /f
```

> [!WARNING]
> Escouade refuse une clé `Policies\Escouade` que l’utilisateur pourrait écrire sans être administrateur : son ACL a été relâchée. Si l’app tourne en administrateur, elle ne peut plus faire cette différence et lit tout.

### macOS : profil de configuration

Escouade lit le domaine de préférences gérées `dev.gagnaire.escouade`. Seules les valeurs qu’un profil de configuration force comptent : un `defaults write` du même domaine est ignoré. [Le dossier policies/macos du dépôt](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/macos) donne :

- `dev.gagnaire.escouade.plist` : le manifeste des clés au format ProfileManifests, que lisent ProfileCreator et iMazing Profile Editor ;
- `dev.gagnaire.escouade.json` : le schéma pour Jamf Pro (Réglages d’applications et personnalisés › Applications externes › Schéma personnalisé) ;
- `escouade-example.mobileconfig` : un profil d’exemple avec chaque clé, à réduire à ce que tu veux imposer avant de l’installer ou de l’envoyer par ton MDM (Jamf, Intune, Kandji).

### Linux : /etc/escouade/policies

Escouade lit les fichiers `/etc/escouade/policies/*.json`, par ordre de nom ; pour une même clé, le dernier gagne. Chaque fichier est un objet JSON de clés et de valeurs. Le dossier, ceux au-dessus et chaque fichier doivent être à root, sans écriture pour le groupe ni pour les autres ; un lien symbolique est refusé. Le schéma `escouade-policies.schema.json` et un fichier d’exemple sont dans [le dossier policies/linux du dépôt](https://github.com/guillaume-gagnaire/escouade/blob/main/policies/linux).

```sh
sudo mkdir -p /etc/escouade/policies
echo '{"telemetry.enabled": false, "permissions.modes": ["plan", "default"]}' \
  | sudo tee /etc/escouade/policies/10-company.json
sudo chmod 644 /etc/escouade/policies/10-company.json
```

## L’enrôlement

Le poste géré peut poser l’adresse du serveur d’équipe de l’entreprise (`team.server`) et l’empreinte de son équipe (`team.fingerprint`) : le membre n’aura plus qu’à accepter son invitation. Avec `team.onlyThisServer`, l’app refuse tout autre serveur d’équipe ; avec `servers.allowed`, tout serveur hors de la liste. Aujourd’hui, l’app garde ces valeurs et tient le refus ; l’écran d’entrée dans l’équipe, qui les préremplira, vient avec le serveur d’équipe.

## Ce que voit l’utilisateur

- Un réglage imposé est grisé, son contrôle inerte, avec un cadenas et « Imposé par la stratégie de ce poste ».
- Dans une liste bornée (les modes, les harness), les choix non permis sont grisés, avec la raison dans leur bulle.
- Une écriture refusée le dit : « Ce réglage est imposé par la stratégie de ce poste (telemetry.enabled) : il ne se change pas sur ce poste. »
- Ce que la lecture n’a pas pu appliquer (une clé inconnue, une valeur mal formée, une clé de registre à l’ACL relâchée) est listé dans Réglages › Général › « Stratégies de ce poste ».
