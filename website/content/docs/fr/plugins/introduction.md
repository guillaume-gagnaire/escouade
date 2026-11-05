# Les plugins

Un plugin ajoute à Escouade ce qu’elle ne fait pas seule : un thème, un harness, des notifications, des règles pour les autorisations de tes agents. Cette page dit ce qu’un plugin peut faire, sous quelles formes il vient, d’où il vient et par où commencer.

## Ce qu’un plugin apporte

Un plugin se décrit dans un fichier, son [manifeste](plugins/manifest), et y déclare ce qu’il ajoute :

- **des thèmes** : les couleurs de la fenêtre, en sombre et en clair, et une palette pour les projets ([les thèmes](plugins/themes)) ;
- **des réactions aux événements** : un webhook ou une commande quand un ticket passe « À tester », quand un agent attend, quand un tour finit ([les événements](plugins/events)) ;
- **des décisions d’autorisation** : autoriser ou refuser ce qu’un agent demande, avant que la question n’arrive jusqu’à toi ([les règles d’autorisation](plugins/permission-rules)) ;
- **des harness** : un agent qui parle ACP, comme Goose, décrit sans code ([un harness ACP décrit](plugins/harnesses)) ;
- **un lecteur de workflows**, pour un workflow qu’Escouade ne lit pas seule ;
- **ses réglages et ses secrets**, que tu saisis sur sa page.

Certaines intégrations d’Escouade sont elles-mêmes des plugins, livrés avec l’app : GitHub, GitLab, Bitbucket, Gitea et Forgejo, Trello, les harness ACP, isola et les services de voix en ligne (voir [les plugins livrés avec Escouade](plugins/installer#les-plugins-livres-avec-escouade)).

## Trois formes

| Forme | Ce qu’elle est | Ce que dit la fenêtre d’installation |
| --- | --- | --- |
| Déclaratif | Le manifeste seul : un thème, des webhooks, un harness décrit. Rien ne s’exécute. | Ses permissions, sans programme |
| Programme en WASM | Un bundle en JS ou en TypeScript (`run.js`), ou un module en Rust (`run.wasm`), qu’Escouade fait tourner dans l’app, sans Node ni Rust chez toi. Il n’atteint le réseau, les fichiers et les programmes que par Escouade, et ce qu’il déclare seulement. | « Tourne dans Escouade, isolé » |
| Programme natif | Un exécutable (`run.command`) lancé dans un process, avec tes droits. Le store n’en publie aucun : il ne vient que d’un dossier ou d’un dépôt git. | « Lance un programme sur ton ordinateur, avec tes droits » |

Un programme parle à Escouade par [le protocole](plugins/api) ; [le SDK](plugins/sdk) l’écrit pour toi en TypeScript, [le kit](plugins/rust) en Rust.

## Ce qu’un plugin ne peut pas faire

- **Lire les secrets d’un autre** : un plugin n’a que les siens, jamais ceux d’un compte, d’un harness ou d’un autre plugin.
- **Agir dans un projet privé** sans ton accord : un plugin installé y est coupé d’office (seuls les plugins livrés avec Escouade y sont actifs, comme le reste de l’app).
- **S’installer seul** : un projet peut proposer un plugin dans son [`.escouade.yml`](escouade-yml#plugins), jamais l’installer ; un lien `escouade://` ouvre sa page, jamais l’installation.
- **Recevoir ce qu’il n’a pas demandé** : un fait hors de `permissions.events` ne lui est pas envoyé, un secret hors de `permissions.secrets` lui est refusé (voir [les permissions](plugins/permissions)).
- **Changer l’interface** au-delà de ce que son manifeste déclare : pas de vue à lui.

## D’où il vient

- **Livré avec Escouade** : signé et mis à jour avec l’app ; il se coupe, il ne se désinstalle pas.
- **Le store**, [store.escouade.dev](https://store.escouade.dev) : chaque version y passe onze contrôles ; celles qu’Escouade a relues portent le sceau « Vérifié par Escouade », les autres sont « Non vérifié ».
- **Un dossier ou un dépôt git** : permis, toujours « Non vérifié » : personne ne l’a relu.
- **Un dossier en développement** : chargé en place, relu quand il change, pour l’écrire.

## Par où commencer

- **L’utiliser** : [installer un plugin](plugins/installer), puis [les permissions](plugins/permissions).
- **En écrire un** : [un premier plugin](plugins/quick-start) en dix minutes, en TypeScript ou en Rust, puis [le manifeste](plugins/manifest), [les capacités de l’hôte](plugins/capabilities), [les appels](plugins/calls) et [les plugins d’exemple](plugins/examples).
- **Le publier** : [tester un plugin](plugins/testing), [publier sur le store](plugins/publishing), [la sécurité des plugins](plugins/security) et [les versions](plugins/versioning).
- **Une question** : [les questions fréquentes](plugins/faq).
