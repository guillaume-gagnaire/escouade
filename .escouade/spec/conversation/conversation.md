---
title: "Conversation"
subject: "Conversation"
sources: ["docs/SPEC.md"]
tickets: ["ESC-76","ESC-127","ESC-95","ESC-108","ESC-89"]
decisions: []
compared: null
order: 13
---

## Ce que c'est

La conversation est la vue d'un agent dans Escouade : son en-tête, ses messages et ses appels d'outils, les cartes de ses demandes (permission, question, plan) et le champ d'où l'utilisateur lui écrit, lui joint des fichiers et règle son modèle, son effort et son mode. Elle est la même pour tous les harness (voir *Harness*) ; l'interface propre du harness est l'autre vue de l'agent (voir *Pilotage de Claude Code*).

## Comportement

### En-tête

- 60 px de haut, 69 avec ses chiffres. Le nom (double-clic : renommer) et l'état dans une pastille : En cours en vert, Question en ambre, Prêt et Terminé en gris, Erreur en rouge.
- Dessous : la marque du harness en entier (« Claude Code », dès que deux harness sont trouvés), le projet, « / » et la branche, les numéros des PR de l'agent avec l'icône de leur état (voir *Pull requests*) et, pour un harness qui a un bac à sable (Codex), « Bac à sable » : le sens du mode en infobulle, rien en Bypass (qui n'en a pas), l'icône seule sous 1000 px de conversation.
- Les chiffres, l'étiquette au-dessus de la valeur : Contexte (« 76,2 k / 200 k » et une jauge fine dessous, ambre au-delà de 80 %), Tokens, Coût (« ≈ … estimé » pour un harness dont Escouade estime le coût), Fichiers (ouvre le panneau des fichiers ; compteur seul en disposition moitié / moitié), Durée.
- Une seule place de bouton : « Ouvrir une PR » tant que la branche a des commits sans PR ouverte ; sinon « Tester » (agent à worktree qui a une recette de lancement) ou « Préparer le lancement » (agent à worktree sans recette, pas tant que son ticket est « En cours ») : voir *Lancement de test*. Le lancement reste dans le menu de l'agent.
- Les deux vues en segments, « Escouade | <harness> » : la vue affichée nommée, l'autre en icône ; jamais pour un agent de ticket.
- « … » : le menu de l'agent, celui du clic droit sur sa carte, ouvert sous le bouton, le clavier dedans : Renommer, Dupliquer, Archiver, Lancer…, Préparer le lancement, Intégrer, Ouvrir une PR…, Ouvrir dans l'éditeur, Ouvrir un terminal (dans le worktree de l'agent), la vue, les harness, le remote control, Supprimer…
- Le sélecteur de disposition. L'éditeur s'ouvre depuis la barre de titre ; le modèle se choisit dans la puce du champ.
- Quand la conversation se rétrécit : sous 1000 px, Tokens et Fichiers s'effacent et la vue cachée perd son mot ; sous 760 px, Durée aussi, et les boutons ne gardent que leur icône (toujours nommés pour les lecteurs d'écran) ; sous 580 px, les chiffres s'effacent (la carte de l'agent les montre aussi).
- Sous l'en-tête, quand l'agent a des tâches, le bandeau « Plan » (voir *Avancée d'un plan*).

### Disposition

- Globale, gardée d'un lancement à l'autre, `Ctrl+Maj+L` : classique (le panneau des fichiers s'ouvre à la demande) ou moitié / moitié : la conversation à gauche ; à droite, le panneau « Non commités » toujours visible, avec sous la liste le diff du fichier sélectionné (le premier par défaut, rafraîchi pendant que l'agent édite).

### Messages

- Dans une colonne de 780 px au plus.
- Utilisateur : une bulle à droite ; ses fichiers en puces sous le texte, l'image et le fichier dessinés ; sous la bulle, « Dit à voix haute », « transmis pendant le tour » ou « depuis claude.ai ». Un message venu du serveur MCP (envoyé par un client, ou premier message d'un agent qu'il a créé) est précédé de « Message de <auteur> : » (voir *Serveur MCP*).
- Assistant : du Markdown, sans avatar ; le code dans un cadre, colorié dans les deux thèmes. La réflexion est repliée, dépliable.
- Appels d'outils, en un fil, sans boîte par appel : l'icône de ce que fait l'appel, le verbe, la cible, le résultat à droite, un trait entre les appels qui se suivent. Ils se déplient : le diff d'Edit/Write dans un cadre (unifié ou côte à côte, selon le style choisi pour les vues de diff), la sortie de Bash…
- Pendant un tour : « Claude travaille… » (une étincelle qui tourne, un reflet qui passe sur les mots, arrêtés l'un et l'autre par prefers-reduced-motion) et le bouton Stop, qui interrompt le tour.

### Longues conversations

- Seuls les 80 derniers éléments sont dessinés ; la vue suit les nouveaux tant qu'elle est en bas.
- « Afficher les N précédents » (« Afficher le précédent » pour un seul), en haut, ajoute les 80 d'avant sans déplacer le message lu, autant de fois qu'il le faut. Un lecteur d'écran entend « 80 messages précédents affichés » ; le focus reste sur le bouton (sur la conversation après la dernière tranche).
- Dès que la vue quitte le bas, ce qui est dessiné ne bouge plus ; revenir en bas (ou « ↓ Nouveaux messages ») redessine les 80 derniers. Seul un geste vers le haut quitte le bas : la molette vers le haut, ↑, Page↑, Début, Maj+Espace, Maj+Tab, ou le focus qui passe sur un élément plus haut ; ni la molette vers le bas, ni les touches qui descendent, ni Ctrl, Alt ou Maj seuls.
- Position de lecture, par agent, pendant la session : revenir sur un agent remet la conversation sur le message qu'on lisait (même décalage, tenu une seconde pendant que les messages voisins prennent leur vraie hauteur), avec les messages plus anciens qu'on avait affichés ; laissée en bas, elle reste en bas et suit les nouveaux messages.
- La conversation d'un agent qui n'est pas affichée depuis 10 minutes (vérifié chaque minute) quitte la mémoire de la fenêtre, sauf si l'agent travaille, attend une réponse, ou qu'un message attend la préparation de son worktree ; elle est relue à sa prochaine ouverture.

### Demandes : questions, permissions, plans

- Une carte de question ou de permission est ambre tant qu'elle attend, ses options en boutons (la première en blanc quand un clic répond) ; une réponse libre est possible dans le champ.
- Répondue, elle devient une carte à filet : « <harness> t’a posé une question », la question, ses choix avec celui retenu marqué (sinon la réponse libre seule), « Répondu ». Une autorisation répondue dit l'outil, sa cible et « Autorisé », « Autorisé pour cette session », « Toujours autorisé » ou « Refusé » (icône verte ou rouge, et le message d'un refus). Une réponse donnée par un client du serveur MCP ajoute « par <auteur> » et la pastille teal « via Escouade ».
- Ce que « Toujours autoriser » garde, quand ce n'est pas la demande elle-même (le motif d'OpenCode), est dit sur la carte avant le clic.
- Le plafond de crédits d'une session de Copilot CLI et le formulaire d'un serveur MCP sont des cartes de question comme les autres.
- Les demandes d'un agent restent dans l'ordre d'arrivée ; le clavier et le champ répondent à la plus ancienne.

### Répondre au clavier

- Depuis la conversation ou son champ (pas un terminal, pas l'éditeur, pas derrière une boîte de dialogue ; jamais avec AltGr ni une touche maintenue) :
  - `Ctrl+Entrée` autorise (« Approuver le plan » pour un plan) ; `Ctrl+Maj+Entrée` autorise toujours, quand Claude propose une règle ;
  - `Alt+1..9` choisit l'option n de la question (coche ou décoche pour un choix multiple ; avec plusieurs questions, celle qui attend sa réponse, puis la suivante) ; `Ctrl+Entrée` valide quand la réponse est complète, sinon passe à la question sans réponse, ou envoie le texte du champ comme réponse libre ;
  - un texte tapé puis Entrée refuse toujours, avec ce message.
- Les boutons de la demande en cours montrent leur raccourci.
- Pendant 500 ms après qu'une demande devient celle à laquelle le clavier répond (elle arrive, sa carte s'affiche, ou elle prend la place de celle qu'on vient de régler), ces touches et Entrée dans le champ (pas Maj+Entrée) ne font rien : le texte tapé reste dans le champ.
- Une demande que Claude Code refuserait par défaut n'a ni raccourci ni réponse au clavier : seul son bouton « Autoriser » l'autorise, et « Refuser » (« Continuer à planifier » pour un plan) est le bouton principal de sa carte ; `Ctrl+Entrée` dans le champ y agit comme sans carte (un texte la refuse).
- Après une réponse donnée depuis la carte, le focus revient au champ.

### Fin d'un tour

- Le dernier tour d'un agent « Terminé » finit par la carte « Tâche terminée » : durée, tokens, coût, nombre de fichiers modifiés, puis leur liste (chemin, +ajouts −suppressions), chaque chemin ouvrant le fichier dans l'éditeur.
- Les autres tours finissent par une ligne « durée · tokens · coût » (« Interrompu » à la place de la durée).
- Un tour en erreur : « Le tour s'est terminé en erreur », sa raison, et « Reprise automatique … » avec « Annuler la reprise » quand une reprise est prévue.
- Un tour arrêté par la limite d'usage propose en plus « Reprendre sur <compte> », et, si cette reprise a échoué, « Revenir sur <compte> » (voir *Comptes*).
- Un tour arrêté par une connexion que le harness n'a pas pu rafraîchir n'a pas de carte rouge : une ligne comme les autres si l'agent repart, sinon la carte « Connexion à refaire » (voir *Comptes*).
- Aucun compte : un message refusé faute de compte du harness de l'agent donne, au lieu d'un toast, une carte ambre au bout de la conversation : « Aucun compte <harness> : ajoute-en un dans Réglages › Comptes. », « Cet agent ne démarre pas tant qu’aucun compte <harness> n’est ajouté… Ton message reste dans le champ. », « Ajouter un compte <harness>… » (la fenêtre d'ajout sur ce harness) et « Continuer sur <harness> » vers le premier harness de passation de l'agent. Elle s'en va dès qu'un compte actif de ce harness existe, ou au message suivant.
- Changement de compte au seuil : un séparateur comme celui d'une passation, des filets de chaque côté, « L’agent passe sur le compte <compte> » en gras (voir *Comptes*).

### Préparation du worktree

- « Préparation du worktree · 2/3 · npm ci — tes messages partiront une fois terminée. » ; « Voir la sortie » déplie la sortie de l'étape en cours, en direct : ses 500 dernières lignes, telles qu'un terminal les laisse (codes ANSI retirés, barres de progression ramenées à leur dernier état), qui défilent seules tant qu'on reste en bas (« Pas encore de sortie. » avant la première ligne). Les lignes arrivent par lots, toutes les 50 ms au plus.

### Champ de saisie

- Menus ouverts vers le haut :
  - **Modèle** : pour Claude Code, ceux que son `/model` liste pour le compte de l'agent, dans son ordre et sous ses noms, `default` à part : un modèle qu'un compte a en plus (comme Mythos) y est, un modèle qu'il n'a pas n'y est pas ; avant la première lecture du compte, Fable · Opus · Sonnet · Haiku (voir *Comptes* ; pour les autres harness, voir *Harness*). Au-delà de 8 modèles, ou pour des modèles de fournisseurs (OpenCode) : un champ « Rechercher », une étoile au bout de chaque ligne (en contour discret, pleine à la couleur d'accent pour un favori), les favoris d'abord sous « Favoris », puis un groupe par famille sous son intitulé. Les familles : pour Claude Code, Fable, Opus, Sonnet, Haiku, Mythos, puis toute autre famille annoncée, de la plus récente à la plus ancienne ; pour OpenCode, le fournisseur ; pour Codex et Copilot CLI, l'éditeur (GPT, Claude, Gemini…) ; « Autres » pour un modèle sans famille reconnue. La recherche cache un groupe vide ; ↑ ↓ passent d'un modèle à l'autre, Entrée choisit.
  - **Compte** : seulement avec plusieurs comptes Claude (voir *Comptes*).
  - **Effort** : Bas · Moyen · Élevé · Très élevé · Max.
  - **Mode** : Auto · Demander · Plan · Édits auto · Bypass ; Auto par défaut. « Auto » est grisé, « indisponible avec ce modèle », pour un modèle que le Claude Code du compte de l'agent dit sans mode Auto (lu pour chaque compte ; avant sa première réponse, un Haiku plus ancien que 5). Un mode que le harness n'a pas reste à sa place, grisé avec sa raison (« OpenCode n’a pas de mode Auto : rien ne décide à ta place. ») : les cinq modes gardent leurs places.
- À droite : joindre un fichier, dicter et converser, en icônes ; puis Stop avec sa touche Échap (pendant un tour) et Envoyer, en blanc, avec la touche Entrée dessinée.
- Le champ flotte au-dessus du panneau, de la largeur de la colonne ; son anneau est à la couleur du projet quand il a le focus, ambre si une question attend ; sa barre passe à la ligne quand la place manque. Sous 760 px de conversation, les libellés des puces et la touche de Stop s'effacent ; sous 520 px de champ, Stop n'est plus que son icône.
- La dictée s'affiche dans le champ même (niveau, durée, ce que le service a compris, « <service> · en direct », les touches), comme l'offre du modèle et son téléchargement (voir *Dictée*). Un fichier glissé entoure le champ et dit ce qui arrivera aux fichiers.
- L'autocomplétion `@fichier` ; les slash commands (intégrées, `.claude/commands` et skills).

### Envoyer

- Un message envoyé pendant un tour est transmis tout de suite : Claude Code l'intègre à l'étape suivante du tour, avec la mention « transmis pendant le tour » jusqu'à sa fin (pour les autres harness, voir *Harness*).
- Un message envoyé avant que l'envoi du précédent ait rendu la main quitte le champ tout de suite et part juste après lui, jamais ignoré.
- Pendant la préparation du worktree, le message s'affiche tout de suite avec « En attente de la préparation… » et part une fois la préparation finie ; un deuxième, envoyé avant, est refusé par le toast « Un message attend déjà la fin de la préparation. » et reste dans le champ.
- Un message refusé à l'envoi (harness introuvable…) revient dans le champ de son agent, même si un autre agent est affiché entre-temps, au-dessus de ce qui a été tapé depuis (séparé par une ligne vide), ses fichiers joints avant ceux ajoutés depuis.
- Brouillons : le texte du champ de chaque agent est gardé sur la machine, 300 ms après la dernière frappe, et tout de suite avant un redémarrage pour mise à jour ou un rechargement de la fenêtre ; il revient après un redémarrage, une mise à jour ou un rechargement. Vidé à l'envoi (remis si l'envoi est refusé), supprimé avec l'agent. Les puces des fichiers joints ne sont gardées que pendant la session (la copie d'un fichier joint reste sur le disque avec son agent).

### Fichiers joints

- N'importe quel fichier se joint, par le trombone, le collage ou le glisser-déposer ; la fenêtre de choix n'a pas de filtre. Il part par l'une de deux routes :
  - **Dans le message** : une image PNG, JPEG, GIF ou WebP (5 Mo), un PDF (18 Mo), un texte (256 Ko ; reconnu à son extension ou à son type ; un fichier sans extension ni type connu, de 256 Ko au plus, est lu pour voir si c'est du texte), dans la limite de 18 Mo pour le message (un fichier joint par son chemin n'y compte pas).
  - **Par chemin**, tout le reste : un autre format, un fichier trop gros pour le message (image de plus de 5 Mo, PDF de plus de 18 Mo, texte de plus de 256 Ko), un total de 18 Mo dépassé, un « texte » qui ne se décode pas. Le message donne alors son chemin à l'agent, qui le lit avec ses outils : le texte de l'utilisateur, une ligne vide, puis une ligne par fichier, `Fichier joint : rapport.xlsx, C:\…\rapport.xlsx` (`Attached file: …` selon la langue des textes pour Claude : voir *Langues*) ; sans texte, les lignes seules. La conversation montre le message complet ; le nom de l'agent n'est tiré que du texte de l'utilisateur. Le nom et le chemin tiennent sur une ligne (un saut de ligne devient une espace). Le chemin est celui que lit le harness de l'agent (`/mnt/c/…` sous WSL), recalculé par l'app, jamais pris de la fenêtre.
- À l'envoi, chaque fichier est revérifié ; un seul refusé (une copie hors du dossier de l'agent, un fichier d'origine disparu ou illisible) refuse tout le message, qui reste dans le champ avec ses puces.
- **Fichier d'origine ou copie.** Un fichier joint par chemin est soit le fichier d'origine, lu là où il est, sans copie (« Joindre le fichier d'origine », allumé par défaut), soit une copie hors du dépôt. Ce que le message porte n'est jamais « d'origine ». Pour un fichier déposé, Escouade ne donne pas son dossier à l'agent : elle autorise la lecture de ce fichier seul, à son chemin d'origine, jamais l'écriture. Les tickets et la vue Claude Code gardent toujours une copie.
- Le chemin d'origine n'est connu que sous Windows : l'app le lit elle-même à l'instant du dépôt ou du collage, la page n'en donnant jamais. Un fichier du disque, déposé ou collé depuis l'Explorateur, a son chemin ; ce qui n'est pas un fichier du disque (une capture collée du presse-papiers) n'en a pas. Sous Linux et macOS, aucun chemin d'origine. Sans chemin connu, ou avec « Joindre le fichier d'origine » éteint : copie.
- La fenêtre ne peut désigner un fichier d'origine que par ce que l'app a lu au dépôt : un chemin que l'app n'a jamais lu est refusé, à l'attache comme à l'envoi. Un fichier d'origine doit avoir un chemin absolu, sans caractère de contrôle, hors du dossier de données d'Escouade (refusé sinon), et être un fichier ordinaire, existant et lisible ; rien n'est écrit, et il est revérifié à l'envoi.
- **Taille** : aucune limite par défaut. Au-delà de « Avertir au-delà de » (200 Mo), une marque « ! » sur la puce, avec l'infobulle « Gros fichier : 250 Mo, copié sur le disque » (ou « … joint là où il est, sans copie » pour un fichier d'origine) : le fichier se joint quand même. Au-delà de « Taille maximale d'un fichier joint » (aucune par défaut), il est refusé (« <nom> dépasse <max> », par exemple « 100 Mo »), à la jointure, pendant la copie et à l'enregistrement d'un ticket. Un fichier vide se joint.
- **Texte de plus de 256 Ko** : joint par son chemin (par défaut) ou refusé (« <nom> dépasse 256 Ko »), seulement pour un texte reconnu à son extension ou à son type. La règle vaut partout où un fichier part par son chemin : le champ, le formulaire d'un ticket et le terminal de la vue Claude Code.
- **Copie** : les octets passent par morceaux de 4 Mo au plus ; la taille écrite doit être celle annoncée (refus au-delà de plus d'un morceau, et au-delà du plafond). Pendant la copie, la puce montre un pourcentage et une barre (« Copie de <nom> » pour les lecteurs d'écran) et « Envoyer » est désactivé. Retirer la puce annule la copie, ou efface la copie finie (jamais hors du dossier de son propriétaire, ni un fichier d'origine). Les fichiers à moitié écrits d'une fenêtre rechargée sont effacés au démarrage, puis chaque heure.
- **Puces**, dans le champ et le formulaire d'un ticket : une icône par type (archive, tableur, document, vidéo, audio, code, générique), le nom, la taille, une croix qui retire (« Retirer <nom> ») et l'avertissement de taille. La puce d'un fichier d'origine porte en plus un marqueur et l'infobulle « Fichier d'origine, lu là où il est » (« Original file, read where it is »), pour qu'on ne la prenne pas pour une copie. Pas d'aperçu hors des images : les images du message gardent leur vignette, une image envoyée par son chemin (plus de 5 Mo) a une puce.
- **Où sont les copies** : `<données>/attachments/<agent>/<uniq>/<nom>`, `<uniq>` étant 8 caractères hexadécimaux au hasard : deux fichiers du même nom ne se marchent pas dessus, et l'agent voit le vrai nom. Le nom est nettoyé : ni séparateur ni caractère réservé de Windows (`<>:"/\|?*`, caractères de contrôle), ni marque d'écriture de droite à gauche (qui ferait lire `fdp.exe` comme `exe.pdf`), ni espace au début, ni point ou espace final, ni nom d'appareil de Windows (`con`, `nul`, `com1`… : préfixé de `_`) ; 150 caractères et 240 octets au plus, extension gardée ; `fichier` s'il est vide. Un fichier d'origine n'est jamais déplacé ni copié.
- **Lecture sans permission** : le dossier `attachments/<agent>` est donné à Claude Code par `--add-dir` (voir *Pilotage de Claude Code*). Un fichier d'origine, hors de ce dossier, est accordé à l'envoi, avant le tour : Escouade répond « autorisé » à la demande de lecture (`Read`) dont le chemin est exactement ce fichier (séparateurs, préfixe `\\?\` et casse d'un chemin Windows ou `/mnt/c/…` ne comptent pas ; aucun `..` n'est résolu), et à elle seule : jamais `Edit` ni une écriture. Une demande qu'elle ne peut pas accorder suit son cours, avec sa carte. Les 200 derniers fichiers accordés sont gardés avec l'agent (au-delà, le plus ancien est de nouveau demandé à la lecture) et effacés avec lui.
- **Tickets** : le formulaire d'un ticket (voir *Tickets*) joint ses fichiers par les mêmes puces et les copie toujours, quel que soit le réglage : l'agent les lit plus tard, l'original peut avoir bougé ou disparu. Avant l'enregistrement, la copie est dans `attachments/_drafts/` (vidé après 24 h, au démarrage puis chaque heure) ; à l'enregistrement, elle passe dans `<données>/tickets/<id>/files/`, 20 fichiers au plus par ticket, les images en plus. Tout est vérifié avant de déplacer quoi que ce soit : un ticket qui a quitté « À faire » est refusé (« Seul un ticket « À faire » se modifie. ») ; un fichier gardé doit être du ticket, un nouveau doit venir des brouillons ; un fichier au-delà du plafond est refusé ; un échec laisse le ticket intact. Un fichier gardé dont la copie a disparu (la purge) est laissé de côté à l'enregistrement.
- L'agent d'un ticket reçoit une ligne `Fichier joint : <nom>, <chemin>` par fichier à la fin de son premier message (après les critères et la ligne de boucle, comme à la reprise d'un agent qui ne l'a jamais reçue ; seule la note d'un échec de préparation du worktree vient ensuite), suivie, pour un fichier disparu, de « Fichier absent du dossier du ticket, non joint : <nom>. » (au pluriel, « Fichiers absents du dossier du ticket, non joints : … »). Ses images gardent leur route (voir *Tickets*). Le serveur MCP joint, liste, lit et transmet ces fichiers aux mêmes limites (voir *Serveur MCP*).
- **Vue Claude Code** : toujours une copie, son process ne pouvant lire sans demander que le dossier de l'agent. Un fichier collé ou déposé sur son terminal (un cadre en pointillés montre la zone) est copié, puis son chemin est tapé ; une image déposée suit la route des images collées. Le plafond s'y applique ; pas de puce, donc pas d'avertissement. Un message avec des fichiers y est refusé : « Pas de pièce jointe en vue Claude Code : colle-la dans son terminal. ».
- **Effacement** : la croix d'une puce efface sa copie ; les copies d'un agent partent avec lui, et avec le projet à sa suppression définitive (fermer un projet les garde : voir *Fermer un projet*) ; celles d'un ticket, avec le ticket. « Abandonner » le formulaire d'un ticket efface les copies qu'il a faites. Jamais un fichier d'origine.
- **Purge** : avec « Purger les copies après » N jours (0 par défaut : tant que leur propriétaire existe), au démarrage puis chaque heure, les copies des agents et des tickets modifiées il y a plus de N jours sont effacées, avec leurs dossiers vides. Le ticket garde l'entrée d'un fichier purgé : son agent est prévenu qu'il manque. Jamais un fichier d'origine.

## Réglages

- Réglages › Harness, groupe « Pièces jointes » (après l'arrêt des process inactifs), sous la note « Les images, les PDF et les textes dans leurs limites partent dans le message. Tout autre fichier, ou un fichier trop gros pour le message, est joint par son chemin : Claude le lit avec ses outils. » :
  - « Taille maximale d'un fichier joint » : « Mo, 0 = aucune limite » ; 0 ;
  - « Avertir au-delà de » : « Mo, 0 = jamais » ; 200 ;
  - « Joindre le fichier d'origine » : un interrupteur, allumé ; « Un fichier déposé est joint là où il est, sans copie : Claude le lit à son emplacement. Désactivé, Escouade en garde toujours une copie. Les tickets et la vue Claude Code copient toujours. » ;
  - « Texte de plus de 256 Ko » : « Joint par son chemin » (par défaut) ou « Refusé » ;
  - « Purger les copies après » : « jours, 0 = tant que leur propriétaire existe » ; 0.
- Les trois nombres sont des entiers de 0 ou plus ; un champ vidé vaut 0.
- La disposition se change par le sélecteur de l'en-tête ou `Ctrl+Maj+L`.

## Limites

- Le chemin d'origine d'un fichier déposé n'est lu que sous Windows : ailleurs, toujours une copie.
- L'interface ne montre pas la liste des fichiers accordés en lecture et ne permet pas d'en retirer un ; supprimer l'agent l'efface.
- Pas d'aperçu d'un fichier qui n'est pas une image ; pas d'envoi de fichier par `send_message` du serveur MCP ; la copie d'un agent garde les chemins des fichiers joints de l'original.
