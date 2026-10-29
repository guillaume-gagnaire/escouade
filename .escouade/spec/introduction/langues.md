---
title: "Langues"
subject: "Introduction"
sources: ["docs/SPEC.md","src-tauri/src/core.rs","src-tauri/src/engine/mod.rs","src-tauri/src/engine/claude/mod.rs","src-tauri/src/worktrees.rs"]
tickets: ["ESC-32"]
decisions: []
compared: null
---

## Ce que c'est

Escouade parle français et anglais : son interface, ce qu'elle dit aux agents ou leur fait écrire, ses formats, son site et sa vidéo. L'interface suit la langue du système par défaut ; les textes pour les agents suivent l'interface par défaut.

## Comportement

### Langue de l'interface

- La langue du système est lue une fois par lancement : français si la première étiquette de locale a pour langue `fr` (`fr`, `fr-CA`, `fr_FR.UTF-8` ; pas `frr`), anglais sinon. La fenêtre et le cœur appliquent la même règle. Jusqu'à la lecture des réglages, la fenêtre part de la langue de sa WebView (même règle) et le cœur parle français.
- Un changement s'applique à l'enregistrement, sans redémarrer ni recharger : la fenêtre, la langue de la page, le menu de l'icône de la zone de notification et son infobulle, et sous macOS le menu de l'app (mêmes entrées, avec les mots de macOS) sont réécrits. Les menus ne le sont que lorsque la langue de l'interface change.
- Sous macOS, ce que le système ajoute de lui-même aux menus (dictée, emoji et symboles, liste des fenêtres) suit la langue du système parmi celles de l'app (`en`, `fr`), pas le réglage.
- L'installeur Windows est en anglais ou en français selon la langue du système.
- Seule une installation neuve suit la langue du système. Des réglages sans langue enregistrée (écrits avant la 1.7) gardent l'interface en français après la mise à jour, et avec elle les textes pour les agents ; l'utilisateur choisit « Système » ou « English » dans Réglages › Application.
- Tout texte montré est traduit, y compris ceux du cœur : erreurs, toasts, notices de la conversation, activité d'un agent, raisons d'un ticket bloqué, issue d'une validation, ce que dit le Kanban quand rien ne démarre, notifications du système, menus, étapes des statistiques (« 27/09 », « S39 », « sept. » / « Sep 27 », « W39 », « Sep »). Le journal de l'app reste en anglais.
- Une phrase entière par texte, jamais de fragments recollés : l'ordre des mots change d'une langue à l'autre. Les pluriels suivent la langue (français : 0 et 1 au singulier ; anglais : 1 seul).
- Glossaire : `src/lib/i18n/GLOSSARY.md`. « À tester » se dit « To review », « Principal » « Main », « fenêtre de 5 h » « 5-hour window ».

### Formats

Les mêmes partout, fenêtre et cœur :

- montants « 0,10 $ » (espace insécable) / « $0.10 » ;
- pourcentages « 42 % » / « 42% » ;
- tokens « 1,2 k · 2,35 M · 3,20 Md » / « 1.2k · 2.35M · 3.20B » ;
- durées « 1h 48m » ; comptes à rebours « 4j 12h » / « 4d 12h » ;
- tailles « 312 Mo » / « 312 MB » ;
- dates « 2 octobre 2026 à 09:30 » / « October 2, 2026 at 9:30 AM » ;
- temps relatifs « il y a 5 min », « hier », « il y a 4 j » / « 5 min ago », « yesterday », « 4 days ago ».

Les touches des raccourcis s'écrivent comme sur le clavier de la langue (`Ctrl+Maj+Entrée` / `Ctrl+Shift+Enter`, `Échap` / `Esc`). L'heure d'une reprise s'écrit en 24 h dans les textes du cœur, dans les deux langues (« reprise à 15:00 ») ; la fenêtre écrit les heures avec la locale de l'interface (« Quota atteint — reprise à 15:00 » / « Quota reached — resumes at 3:00 PM »).

### Texte enregistré

Ce qui est écrit une fois puis relu plus tard garde la langue où il a été écrit ; un changement de langue ne le réécrit pas. C'est le cas :

- des notices de la conversation ;
- de la raison d'un ticket bloqué ;
- de l'issue d'une validation, avec sa version dans la langue des textes pour les agents, que cite le commentaire publié ;
- de l'étape de validation en cours et des critères par défaut d'un ticket ;
- du nom d'une copie (« (copie) » / « (copy) ») ;
- de la ligne d'origine d'un ticket importé (« Ticket Jira X : <adresse> ») ;
- du nom d'un état GitHub gardé dans la correspondance des statuts ;
- du protocole et de la consigne de copie gardés avec l'agent à sa création.

Le nom du compte Principal est enregistré dans la langue du premier enregistrement, mais s'affiche « Principal » / « Main » selon la langue tant qu'il n'a pas été renommé.

Suivent au contraire la langue du moment : le texte d'un quota illisible (l'état est gardé, le texte réécrit), un problème signalé par le Kanban (réécrit à son passage suivant) et les étapes des statistiques (relues au changement de langue).

### Textes pour les agents

Le réglage « Langue des textes rédigés par Claude » porte sur tout ce qu'Escouade dit aux agents ou leur fait écrire :

- le protocole d'un ticket et ses messages (boucles, relance du bilan, reprise, retour de test, tests en échec, merge et rebase de la branche cible) ;
- « Préparer le lancement » ;
- le message de commit d'un ticket et la description de sa pull request ;
- le message proposé pour un commit direct ;
- le nom donné à un agent ;
- les questions de « ✦ Remplir automatiquement » et les noms des commandes proposées ;
- la consigne d'une copie d'agent, le « continue » de la reprise, la demande de commit envoyée à l'agent ;
- le refus d'une autorisation sans message, le refus d'un plan ;
- les commentaires publiés dans Jira, Trello et GitHub.

Les marqueurs que le code relit ne se traduisent pas : la clôture ```` ```escouade ```` et les clés JSON (`criteres`, `n`, `ok`, `note`, `avancement`, `lancement`, `preparation`, `processus`, `nom`, `commande`, `dossier`, `env`, `url`, `ouvrir`, `demontage`, `commandes`) sont les mêmes dans les deux langues. Certaines lectures acceptent aussi des équivalents anglais (`criteria`, `progress`, `setup`, `teardown`, `name`, `command`…).

Le message de commit d'un ticket nomme sa langue : un ticket importé dans une autre langue ne la décide pas. Le message proposé pour un commit direct est écrit dans la langue choisie quand le réglage en nomme une, même sans commit dans le dépôt ; avec « Comme l'interface », il imite la langue des derniers commits du dépôt, ce que dit l'aide du réglage.

Serveur MCP : les descriptions des outils sont en anglais seulement ; leurs refus et erreurs, dans la langue de l'interface.

### Textes rédigés par un modèle

Les questions posées hors de toute session vont au harness choisi dans « Modèle des textes rédigés ». Il y en a de deux sortes :

- **Un texte court** (nom donné à un agent, message de commit et description de pull request proposés) : le modèle le plus rapide du harness, sans aucun outil ; « Claude Haiku » pour Claude Code. 90 s au plus, l'écriture de la question comprise.
- **Une lecture du projet** (« ✦ Remplir automatiquement » des commandes de préparation des worktrees et des commandes de lancement) : un modèle plus fort, dans le dossier du projet, avec des outils qui lisent seulement : il ne modifie et n'exécute rien. Pour Claude Code, Claude Sonnet avec Read, Glob et Grep. 4 min au plus.

Les autres harness ont chacun leurs deux réglages (modèle ou effort, outils de lecture) : voir la ligne « Textes rédigés » de la table de *Harness*. La liste du réglage ne montre que les harness installés, chacun par le nom de son modèle de texte court. Sans choix, ou quand le harness choisi n'est plus trouvé : Claude Code s'il est installé, sinon le premier trouvé. La question part avec le compte de l'agent ou du projet qu'elle concerne ; un projet pas encore créé (assistant « Nouveau projet ») est lu avec un compte qui n'est réservé à aucun projet.

Au-delà de son délai, le process est tué avec tout ce qu'il a lancé. La réponse est lue jusqu'à 1 Mo ; au-delà, le process est tué et la réponse refusée.

### Site et vidéo

- Le site a deux langues : le français à la racine (les liens existants restent bons), l'anglais sous `/en/`. Chaque page a sa langue, son titre, sa description, ses balises de partage, son lien canonique et ses `hreflang`. Les textes sont dans deux catalogues aux mêmes clés ; liens, version et noms propres sont communs.
- Sélecteur « FR / EN » dans l'en-tête, choix gardé dans le navigateur. À la première visite de la racine, un navigateur sans français est envoyé vers `/en/`, sauf choix gardé ou `?lang=fr` ; sans JavaScript, rien ne bouge.
- Une adresse inconnue affiche une page « introuvable » dans la langue de l'adresse.
- La vidéo garde sa voix française. La page anglaise propose ses sous-titres anglais par défaut (mêmes repères que les français) et une affiche faite avec l'interface reconstituée de la vidéo, en anglais.
- Les captures des deux pages sont prises dans la vraie app, sur une démo fictive, dans la langue de la page (voir tests/site-shots/README.md).

## Réglages

- Réglages › Application › « Langue de l'interface » : « Système (…) » (défaut ; la langue du système y est nommée dans celle de l'interface), « English », « Français » (chacune nommée dans elle-même).
- Réglages › Application › « Langue des textes rédigés par Claude » : « Comme l'interface » (défaut), « English » ou « Français ».
- Réglages › Application › « Textes rédigés » › « Modèle des textes rédigés » : un des harness installés ; par défaut, Claude Code s'il est installé, sinon le premier trouvé.

## Limites

- Deux langues seulement : français et anglais.
- Pas de README anglais ; la vidéo n'a pas de voix anglaise.
