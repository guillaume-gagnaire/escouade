---
title: "Documents"
subject: "Docs"
sources: ["docs/SPEC.md"]
tickets: ["ESC-256","ESC-260"]
decisions: []
compared: null
---

## Ce que c'est

L'espace Documents de l'onglet Docs est un drive des fichiers du projet (cahiers des charges, comptes rendus, maquettes…), rangés en dossiers sous `.escouade/documents/`. On y dépose des fichiers par glisser-déposer, et Escouade range chacun. Les agents les cherchent, les lisent et les citent. Le texte des fichiers est lu sur la machine : rien n'en sort.

## Comportement

### Déposer

- Les fichiers déposés passent l'un après l'autre, chacun rangé en connaissant les précédents. Un fichier n'en écrase jamais un autre : le second prend un nom libre (`nom-2.ext`).
- Quand la fenêtre connaît le chemin d'origine du fichier, il est lu sur place. Sinon il est copié par morceaux dans les brouillons, puis effacé une fois dans le drive.
- « Nouveau dossier » crée un dossier, avec un `.gitkeep` (git ne garde pas un dossier vide). Une carte glissée sur un dossier y est déplacée, sans rien écraser.

### Le rangement

Un fichier déposé dans un dossier y reste. Déposé à la racine, Escouade lui cherche sa place, dans cet ordre :

1. **par le nom** : le dossier dont tous les mots du nom sont dans le nom du fichier ou dans sa première ligne (un nom fait de chiffres, « 2026 », ne compte pas) ;
2. **par la nature** : comptes rendus, cahiers des charges, maquettes, juridique, devis et factures, présentations, planning, quand le drive a un dossier de ce nom. La nature est reconnue par des expressions de son nom, qui comptent 3, et de son début, qui comptent 1 ; le seuil est 2 ;
3. **par le sens** : le dossier dont le fichier le plus proche par le sens dépasse un seuil propre au modèle (0,82 pour multilingual-e5-base) ;
4. **par les mots** : la proximité des mots (tf-idf) entre le fichier et chaque dossier (le nom du dossier compté trois fois, les noms et les débuts de ses fichiers ; les nombres écartés), seuil 0,12 ;
5. **un nouveau dossier** : celui de sa nature, « Images » pour une image, sinon « Divers ». Un dossier de ce nom déjà là le prend.

La fenêtre dit où le fichier est rangé et pourquoi ; « Déplacer vers » le change.

### Lire un fichier

Le texte est tiré sur la machine :

- texte et Markdown tels quels, 20 Mo au plus ;
- PDF ; un PDF abîmé ne donne pas de texte ;
- Word `.docx` et OpenDocument `.odt`, 200 Mo et 64 Mo de XML au plus. Leurs titres deviennent des titres Markdown, pour que chaque morceau sache de quoi il parle ;
- images : leurs métadonnées seulement, format et taille (PNG, JPEG, GIF, WebP, BMP, SVG).

Le texte tiré est gardé dans le dossier de données d'Escouade (`projects/<id>/docs-text/`), jamais dans le dépôt. Il n'est relu que quand le fichier change (chemin, taille ou date). Un fichier sans texte n'est pas relu.

### La recherche

Un document est indexé dès qu'il est déposé, en tâche de fond : son nom, puis son texte (Word et OpenDocument découpés comme du Markdown, PDF comme du texte). Cela se fait avec le modèle de la recherche par le sens, une fois téléchargé (voir *Index du code*). L'index se remet à jour à chaque recherche : un fichier changé est relu, un fichier parti oublié. Sans le modèle, la recherche se fait par les mots.

### Lu et cité

- **Lu** : chaque lecture d'un document par un agent est comptée (son premier morceau seulement), hors du dépôt. Le compte suit un déplacement.
- **Cité** : les pages de la spec dont une source le nomme, et les décisions dont un document le nomme, une fois par page ou par décision. Un document est nommé par son chemin (avec ou sans `documents/` ou `.escouade/documents/`), par le chemin de la fenêtre (`Documents › A › b`), ou par son nom, avec ou sans extension, s'il est seul à le porter.
- **Auteur** : le dernier qui l'a commité ; l'utilisateur de la machine tant qu'il n'est pas commité.

### Fichiers lourds

Un fichier que git confie à LFS passe sans question. Sinon, au-delà de 50 Mo, la fenêtre demande : « Ajouter quand même » ou « Ne pas les ajouter ».

### Pour les agents (MCP)

- `list_documents` : les documents et les dossiers, avec leur date, leur auteur, leurs lectures et où ils sont cités.
- `read_document` : le texte, celui des PDF, Word et OpenDocument compris ; les métadonnées d'une image.
- `search_documents` : par le sens et par les mots avec le modèle, par les mots sinon ; la réponse dit lequel (`by`).
- `add_document` : ajoute un document texte. Sans dossier, il est rangé comme un fichier déposé, et la réponse dit où (`placement`).

La consigne donnée à chaque nouvel agent lui dit de chercher et lire les documents par ces outils, et de citer ceux d'où vient ce qu'il écrit dans la spec. Un projet privé garde ses documents : son index n'est qu'à lui (voir *Serveur MCP*).

## Limites

- `.doc` et `.rtf` ne sont pas lus.
- D'une image, seules ses métadonnées sont lues, pas son contenu.
