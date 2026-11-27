---
number: null
id: "pages-des-agents-ap-2-le-registre-la-cop"
title: "Pages des agents AP-2 : le registre, la copie qui ne sort pas du dossier de la page, et l'interrupteur qui retire les outils de la liste"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-673"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-673","ESC-661","ESC-666"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":41,"removed":17}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AP-2 (ESC-673, ligne 2.2) construit le modèle et les outils des pages des agents (docs/AGENT-PAGES.md § 3, § 4, § 9) avant la vue (AP-3), le retour des choix (AP-4) et la page à soi (AP-7). Il fallait fixer ce que l'étude laissait ouvert ou ce qui dépend des tickets suivants : quels fichiers relatifs prendre avec un fichier HTML, comment faire de la place sous les 500 Mo, ce que répondent show_page et l'interrupteur tant que la vue, ask et custom n'existent pas.

## Décision

- **Registre** : `p-<n>` jamais redonné, dans l'état durable (`pages`, `page_seq`) ; versions copiées dans `<données>/pages/<projet>/<p-n>/v<k>/`, écrites à part puis mises en place, 10 gardées. La ligne `page:<p-n>` vit le temps du lancement d'Escouade.
- **La copie** : seuls les fichiers que le document nomme en relatif (`src`, `href`, `poster`, `srcset`, `url()`, `@import`, `import "./…"`), chacun ouvert dans le dossier du fichier HTML et ses sous-dossiers, liens suivis et vérifiés sur le fichier ouvert. Jamais tout le dossier. Un chemin qui sort, un fichier caché (`.env`), un absent ou un nom d'Escouade (`_/`, `_t/`, `index.html`) est laissé de côté, jamais lu, et nommé dans `left_out`.
- **La place** : sous les 500 Mo du poste, la page la plus ancienne dont la ligne est fermée part d'abord ; sans place, refus sans rien effacer. 20 pages par agent : la 21e est refusée en nommant la dernière à mettre à jour.
- **La purge** : une page part après le délai de « Garder les fichiers des demandes closes » une fois son agent archivé ou supprimé (marquée, démarquée s'il revient), et aussitôt avec son projet.
- **En attendant les tickets suivants** : `opened` vaut toujours `waiting` et la webview s'ouvre cachée (AP-3) ; `ask` est nul et Claude hors d'Escouade est refusé (AP-4, qui l'ouvrira avec une demande, P9) ; `custom` est refusé comme « sans page à soi réglée » (AP-7).
- **L'interrupteur « Pages des agents »** (`agentPages`) décide seul, comme « Agents et navigateurs » : coupé, les trois outils quittent `tools/list` (`list_tools` et `get_tool` écrits à la main), un appel est refusé, la consigne se tait, les lignes se ferment, rien n'est effacé. Le serveur MCP coupé ferme aussi les lignes.
- **list_browsers** liste les lignes ouvertes des pages après les navigateurs ; **read_browser** les refuse (get_page en dit l'état).
- **Statistique** `page_shown` : modèle, `ask`, tranche de taille.

## Conséquences

- AP-3 remplace `opened: waiting` par l'ouverture à côté et la carte ; AP-4 ajoute `ask` et ouvre show_page à Claude hors d'Escouade avec une demande ; AP-7 lève le refus de `custom` ; AB-7 vise les lignes par `pages::of_key`.
- `capture_browser` (ESC-670) et `browser_console` (ESC-671), fusionnés après, refusent encore une ligne `page:`, chacun avec sa phrase (« Une page d'agent ne se lit ni ne se capture par les outils des navigateurs », « La console d'une page d'agent ne se lit pas encore »). L'étude veut qu'ils s'y servent sans demander : c'est à faire plus tard (la capture d'une page existe déjà dans la cloison ; une page n'a pas de tampons de console).
- Une page HTML dont les fichiers ne sont nommés qu'en JavaScript calculé (une chaîne construite) n'en reçoit pas : l'agent les nomme en clair ou les met dans le document.
- La copie ne lit rien hors du dossier de la page ni de fichier caché : un secret du worktree ne peut pas finir dans une page par un lien relatif.
