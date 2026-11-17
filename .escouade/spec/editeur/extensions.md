---
title: "Extensions"
subject: "editeur"
sources: ["docs/EDITOR-STUDY.md","https://open-vsx.org (API publique : /api/-/search, /api/<espace>/<extension>, files.sha256, files.signature, files.publicKey, verified)"]
tickets: ["ESC-481"]
decisions: [201]
compared: null
---

## Ce que c'est

Les extensions de la vue VS Code (voir *Éditeur*), prises sur **Open VSX** seulement (le marketplace de Microsoft est exclu : ses conditions le réservent à ses produits). Tout passe par le cœur d'Escouade : la recherche, les fiches, leurs icônes, le téléchargement, la vérification et les fichiers. La page ne demande jamais rien à Open VSX ni au disque.

Dans la 2.3, seules les extensions **déclaratives** tournent : thèmes de couleurs, grammaires, extraits, langages (leur configuration), thèmes d'icônes. Aucun code d'extension ne s'exécute dans Escouade.

## Comportement

### Le panneau

- « Extensions », à droite de la barre de la vue (une icône de blocs et son nom), ouvre le panneau « Extensions · Open VSX » à côté du plan de travail, qui garde sa place et ses onglets. Son × le ferme. Dans une barre étroite (une petite fenêtre, un volet ; moins de 900 px), le bouton n'a plus que son icône, avec son nom et son infobulle, et l'étiquette « VS Code » s'efface : la source, le compte des changements et « Comparer » gardent leur place.
- La recherche (Entrée) donne au plus 20 résultats : icône, nom, description, et un bouclier quand l'espace de noms est vérifié. Un clic ouvre la fiche.
- La fiche : nom, « par <éditeur> », l'identifiant (`éditeur.nom`) et la version, « Vérifié » ou « Non vérifié » (l'espace de noms d'Open VSX, avec une info-bulle qui dit ce que ça veut dire), le nombre de téléchargements, la description, puis « Installer », « Mettre à jour vers <version> » ou « Active ». Sous elle : « Ce que montre une fiche vient d'Open VSX : ce sont les données de son éditeur. », puis son README.
- « Installées » : chaque extension avec sa version, son état, ce qu'elle apporte (« 2 thèmes, 1 grammaire »), « Somme SHA-256 et signature d'Open VSX vérifiées par Escouade. », et « Couper » (ou « Réactiver »), « Retirer », « Utiliser « <thème> » » pour un thème d'une extension active.
- Les états : « Active » ; « Coupée » ; « Non permise » (la liste permise l'exclut, sa raison dessous ; elle n'est plus chargée) ; « En attente de l'hôte » (une extension avec du code pour l'hôte d'extensions web : « En attente de l'hôte d'extensions : son code ne tourne pas encore dans Escouade. ») ; « Demande Node » (« Demande l'hôte Node, pas encore dans Escouade. », à la place du message trompeur de VS Code, « not available … for the Web platform »).
- Un toast dit chaque installation (« « Acme Night » est installée. »), mise à jour (« … est passée en version 1.1.0. ») et retrait.
- En bas : « Les extensions viennent d'Open VSX seulement. Escouade télécharge chacune, vérifie sa somme et sa signature, et ne fait tourner que les extensions déclaratives (thèmes, grammaires, extraits, langages, icônes). »

### Installer : ce que fait le cœur, dans l'ordre

Chaque étape arrête les suivantes ; une installation refusée ne laisse rien sur le disque.

1. **La liste permise**, sur l'identifiant seul : une extension refusée n'est même pas demandée à Open VSX.
2. **La fiche** de la version (la dernière, ou celle d'une mise à jour), puis la liste permise sur sa version (une liste de versions, `stable`).
3. **La confiance dans l'éditeur** : la question de VS Code, gardée. Pour un éditeur pas encore approuvé : « Faites-vous confiance à l'éditeur « <éditeur> » ? », avec « Open VSX a vérifié son espace de noms « <espace> ». » ou « Open VSX n'a pas vérifié son espace de noms « <espace> » : n'importe qui a pu le prendre. », puis « Ses extensions pourront s'installer sans que la question revienne. Escouade vérifie toujours la somme et la signature de chacune. », « Faire confiance et installer », « Annuler ». Rien n'est téléchargé avant la réponse. La confiance est gardée par éditeur, sur ce poste.
4. **Le téléchargement**, chacun dans une borne : le VSIX (200 Mo), sa somme (`.sha256`), sa signature (`.sigzip`) et la clé publique d'Open VSX. Toute adresse que donne Open VSX doit être sur son origine (son stockage peut répondre après une redirection).
5. **La somme SHA-256**, puis **la signature** : Ed25519, de l'octet près du VSIX entier, avec la clé qu'Open VSX publie (`/api/-/public-key/<id>`, au format SPKI). Une version **sans signature n'est pas installée** : « La version <v> n'est pas signée par Open VSX : Escouade ne l'installe pas. »
6. **Le déballage**, dans un dossier temporaire du magasin : sa partie `extension/` seule, chaque entrée vérifiée avant que rien ne soit écrit. L'archive entière est refusée pour une entrée en `..`, un chemin absolu ou de lecteur (`C:`), une barre oblique inverse, un lien, un nom que Windows ne sait pas écrire (`con`, `a:b`, un point ou une espace finale), deux noms qui ne diffèrent que par la casse, plus de 20 000 fichiers, plus de 512 Mo décompressés (256 Mo par fichier) : les tailles annoncées par l'archive ne sont pas crues, chaque entrée est lue jusqu'à ce qui reste (une bombe de décompression s'arrête à la borne). Sans `extension/package.json` : refusée.
7. **Son `package.json`** doit être l'extension et la version de la fiche ; sinon « L'archive n'est pas l'extension de la fiche ».
8. **Le magasin** : `<données d'Escouade>/extensions/<éditeur.nom>-<version>/`, et `installed.json` (la liste, les éditeurs approuvés, le thème choisi), écrit de façon atomique. Une mise à jour remplace la version d'avant une fois la nouvelle vérifiée, et garde son état (coupée ou non).

**Ce que la signature prouve** : les octets installés sont ceux qu'Open VSX a reçus et publiés, sans changement en route (son stockage, un miroir, un cache, un proxy qui servirait un autre fichier). **Ce qu'elle ne prouve pas** : que l'extension est inoffensive, ni que son auteur est qui il dit (c'est l'espace de noms vérifié, montré à part) ; ni rien contre Open VSX lui-même, dont la clé est lue chez lui, par TLS.

Les refus sont dits par le cœur, dans la langue de l'interface : « La somme SHA-256 de l'archive n'est pas celle qu'Open VSX donne : elle n'est pas installée. », « La signature d'Open VSX ne correspond pas à l'archive : elle n'est pas installée. », « L'archive est refusée : « extension/../../x » remonte hors du dossier de l'extension. Rien n'en est installé. », « L'extension « x.y » n'est pas permise par la stratégie de ce poste. »

### Ce qui tourne

- Une extension active est posée dans la vue : son `package.json` (sans `main` ni `browser`) et les fichiers que ses contributions lisent (JSON, grammaires TextMate, plists, extraits, SVG, PNG, polices, 32 Mo au plus), lus par le cœur et donnés à VS Code en `blob:` de leur type, une police en `data:`. VS Code les lit comme ceux de ses extensions intégrées : la grammaire colore les fichiers de son langage, le langage est nommé dans la barre d'état, les extraits se proposent.
- Une extension avec du code s'installe et reste « en attente » : son code ne tourne pas dans la page d'Escouade, qui n'a pas `unsafe-eval` et ne l'aura pas. Il tournera dans l'hôte d'extensions web sur une origine à part (A6), ou dans l'hôte Node (N2) pour celles qui le demandent.
- Une extension coupée, retirée ou devenue non permise quitte la vue tout de suite ; une mise à jour y remplace la version d'avant.

### Le thème

- Le thème d'Escouade reste celui de la vue par défaut (voir *Éditeur › Le thème*). Le thème d'une extension se choisit exprès : « Utiliser « <thème> » ». Il passe alors devant celui d'Escouade (ses couleurs ne sont plus posées), et le panneau le dit : « La vue a le thème « <thème> ». », avec « Revenir au thème d'Escouade », un geste.
- Le choix est gardé par le cœur. Couper ou retirer l'extension ramène le thème d'Escouade.

### Les fichiers, servis par le cœur

- Le cœur sert les fichiers d'une extension installée (donc vérifiée), en lecture seule, avec leur type MIME (`text/javascript` pour le JavaScript, contrairement au point `unpkg` d'Open VSX qui le sert en `text/plain`) et `X-Content-Type-Options: nosniff` : par son protocole `escouade-ext:` (`escouade-ext://localhost/<éditeur.nom>/<chemin>`, `http://escouade-ext.localhost/…` sous Windows) et par la commande de la page.
- Jamais un autre fichier du disque : un chemin en `..` (même encodé, `%2e%2e`), absolu, de lecteur, un lien (même vers l'intérieur), une extension qui n'est pas installée sont refusés (403, 404). Seules les méthodes `GET` et `HEAD` répondent (405 sinon).
- La politique de contenu (CSP) de la fenêtre **n'est pas élargie** : elle ne nomme pas `escouade-ext:`. La page reçoit les fichiers par la commande du cœur, en `blob:` et `data:`, que la CSP permettait déjà. Le protocole est celui de l'hôte d'extensions d'une origine à part (A6), qui aura sa propre CSP.

### Rien de distant sans le cœur

- Ce qui vient d'Open VSX est une donnée de tiers. Une icône se demande au cœur quand elle s'affiche : une image (PNG, JPEG, GIF, WebP, SVG, reconnue à ses octets), 1 Mo au plus, d'Open VSX seulement, rendue en `data:`.
- Le README se lit par le cœur (512 Ko au plus) et se rend comme un Markdown de tiers, assaini comme ceux de la conversation : son HTML en texte, une image du web montrée par son texte et « Afficher l'image », que le cœur ne télécharge qu'à ce clic.
- Ouvrir la vue ne demande rien à Open VSX : la liste des extensions installées est celle du cœur.

### Les mises à jour

- « Rechercher des mises à jour » demande au cœur la fiche de chaque extension installée : « Mises à jour proposées », « 1.0.0 → 1.1.0 », « Mettre à jour vers 1.1.0 » (ou la raison du refus de la liste permise). Rien ne s'installe seul. Une mise à jour repasse par toutes les étapes de l'installation ; refusée (une signature fausse), la version d'avant reste.

### La liste permise

`extensions.allowed`, la règle de VS Code : un objet dont les clés sont `*` (toutes), un éditeur d'Open VSX (`dracula-theme`) ou une extension (`dracula-theme.theme-dracula`), et les valeurs `true`, `false`, `"stable"` (pas de préversion) ou, pour une extension, la liste de ses versions permises (`["2.25.1"]` ; un `@plateforme` après une version est lu comme la version seule). La clé la plus précise dit : l'extension, sinon son éditeur, sinon `*`. Un objet vide (ou absent) permet tout ; un objet qui nomme quelque chose ne permet que ce qu'il nomme.

Le cœur la tient, en quatre couches, chacune devant permettre l'extension, la plus forte d'abord, et la première qui refuse le dit :

| Couche | Où | Dit |
|---|---|---|
| Le poste géré | la stratégie `extensions.allowed` (ADMX, profil macOS, `/etc`), voir *Réglages imposés* | « par la stratégie de ce poste » |
| L'équipe | `extensions.allowed` dans `enforced` du `.escouade.yml` d'équipe | « par l'équipe <nom> » |
| Ce poste | Réglages › Général › Avancé › « Extensions permises de l'éditeur » | « par les réglages de ce poste » |
| Le dépôt | `editor.extensions_allowed` du `.escouade.yml` de chaque projet ouvert dans la vue | « par le .escouade.yml de <projet> » |

- La stratégie est de la sorte « restreindre » : une équipe peut l'imposer. Imposée, le réglage du poste est grisé avec qui l'impose, et le panneau le dit : « Liste permise imposée par la stratégie de ce poste. ».
- Une extension hors de la liste : sa fiche le dit (« L'extension « x.y » n'est pas permise par … »), « Installer » est grisé, et rien d'elle n'est téléchargé (ni VSIX, ni somme, ni signature). Une extension installée que la liste exclut ensuite passe « Non permise » et quitte la vue.
- Le réglage du poste est un objet JSON dans un champ de texte ; vide, il permet tout. Ce qui n'est pas un objet est dit (« Ce n'est pas un objet JSON : … ») et pas gardé ; une clé ou une valeur illisible est dite et ignorée.

### Un projet privé

Le moins bavard : quand un projet ouvert dans la vue est privé, **rien n'est demandé à Open VSX** depuis elle (ni recherche, ni fiche, ni icône, ni installation, ni mise à jour), refusé par le cœur. Le panneau le dit à la place de la recherche : « « <projet> » est un projet privé : Escouade ne demande rien à Open VSX depuis cette vue (ni recherche, ni fiche, ni mise à jour). Les extensions déjà installées marchent. »

## Réglages

- Réglages › Général › Avancé › « Extensions permises de l'éditeur » (voir *La liste permise*).
- `.escouade.yml` : `editor.extensions_allowed` (voir *.escouade.yml*).
- Stratégie du poste et de l'équipe : `extensions.allowed` (voir *Réglages imposés*).

## Limites

- Aucune extension avec du code ne tourne dans la 2.3 : ni dans la page (pas d'`unsafe-eval`), ni dans un hôte d'extensions web (A6), ni dans l'hôte Node (N2).
- Pas de dépendances ni de packs d'extensions : une extension qui en demande d'autres s'installe seule.
- Les extensions sont celles du poste, pour toute la vue : un projet n'a pas les siennes.
- Les captures d'une fiche (images de son README) ne se chargent qu'au clic sur « Afficher l'image ».
- Poids (mesuré sur le build) : +24 Ko compressés (brotli) de frontend, dont 4 Ko dans le morceau principal ; côté cœur, le crate `escouade-vsx` sans aucune crate tierce nouvelle (Ed25519, zip, SHA-256 étaient déjà dans l'app).
