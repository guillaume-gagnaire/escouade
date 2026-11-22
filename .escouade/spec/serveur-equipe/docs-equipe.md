---
title: "Les docs d'un projet d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","src-tauri/src/teamdocs.rs","src-tauri/src/teampour.rs","src/lib/i18n/fr/teamDocs.ts","src-tauri/src/docs.rs"]
tickets: ["ESC-482","ESC-493","ESC-518","ESC-644"]
decisions: [192,194]
compared: null
---

# Les docs d'un projet d'équipe

La spec, les décisions et les documents d'un projet d'équipe vivent sur le serveur d'équipe, chiffrés de bout en bout (TS-9, `docs/TEAM-SERVER.md` §2.2, §2.7, C2-A, C4-B, C5-A). Chaque poste en garde une copie, que la fenêtre, les agents et les outils MCP lisent, hors ligne compris : rien ne lit le serveur.

## Où ils vont

- **Dans le journal du projet** : celui de son Kanban d'équipe (`Project::team_project`, voir « La synchronisation des projets »). Le projet qui n'a pas encore de journal en reçoit un en choisissant « Sur le serveur d'équipe » ; un collègue suit le même journal par son identifiant (32 caractères, montré dans les réglages), ou sans le copier quand le projet est dans la liste de l'équipe (TS-7).
- **Une page de la spec** va en versions entières : son chemin et son texte, chiffrés et signés comme une opération.
- **Une décision** est faite de petits champs (titre, contexte, décision, conséquences, état, liens…) : le dernier qui écrit gagne, champ par champ.
- **Un document déposé** est un fichier de l'équipe, chiffré sous une clé à lui, tirée sur le poste ; la clé, son chemin, sa taille et son empreinte vont dans le journal. **20 Mo au plus** : un document plus lourd reste sur ce poste, et la fenêtre le dit (« … dépasse 20 Mo : il reste sur ce poste, l'équipe ne le voit pas »).
- **Ce que voit le serveur** : des opérations et des fichiers chiffrés, leur taille, leur heure, l'appareil qui les envoie. Jamais un titre, un chemin, un nom de fichier, un texte : les identifiants des objets sont tirés de leur chemin sur le poste, dans les opérations chiffrées.
- **Ce qui reste sur le poste** : le texte extrait des documents, l'index et les vecteurs, recalculés depuis la copie ; les écarts des comparaisons au code.

## La copie sur le poste

- Un dossier des données d'Escouade (`team-docs/.escouade/`) tient les docs comme le dépôt les tiendrait : tout ce qui lit les docs (la fenêtre, les outils MCP, l'index, les vecteurs, le texte extrait) marche comme avant.
- **Hors ligne**, tout se lit et s'écrit dans cette copie. Ce qui est écrit attend dans la file d'envoi et part au retour. En tête de l'onglet Docs : « Docs sur le serveur d'équipe », « … · N changements en attente », ou « Hors ligne · N changements en attente : les docs se lisent et s'écrivent sur ce poste, ils partiront au retour ». La barre d'état dit « Docs sur le serveur d'équipe » au lieu d'une branche, et « Partager » n'a plus lieu d'être : les docs partent seules.
- **Le passage** : à chaque écriture des docs (fenêtre, agent, validation d'un ticket), ce qui a changé dans la copie part au journal ; à chaque lecture de l'équipe (en direct, ou toutes les 15 s), le journal revient dans la copie, sauf ce que ce poste a changé et pas encore envoyé.

## Les conflits

- **Une page écrite des deux côtés** depuis la même version est fusionnée à trois voies sur le poste, ligne par ligne, après la version dont les deux viennent. La fusion est écrite après les deux versions, et chaque poste fusionne pareil.
- **Un vrai conflit** (les mêmes lignes changées de deux façons) se montre en tête de l'onglet Docs : « La page « X » a été écrite ici et par un collègue en même temps, sur les mêmes lignes », avec « Comparer » (les deux versions côte à côte), « Garder la mienne », « Prendre la sienne » et « Donner à l'agent rédacteur ». Pendant ce temps, ce poste garde sa version. La version choisie, ou celle qu'écrit l'agent rédacteur (`write_spec_page`), est écrite après les deux, et le conflit se ferme chez tous.
- **Une décision** : le dernier qui écrit gagne, champ par champ. Celui qui a perdu le voit : « Décision « X » : un collègue a changé « champ » en même temps ; sa version a gagné. La tienne : « … » », jusqu'à « Compris ».

## Les numéros des décisions acceptées

- **Un seul compteur pour l'équipe**, dans le journal. Accepter une décision la rend acceptée et demande un numéro.
- **Le numéro est sa place** parmi les demandes du compteur, dans l'ordre que le serveur donne au journal, après le plus haut donné avant. Chaque poste lit le même ordre et donne donc le même numéro : **deux acceptations au même moment n'ont jamais le même numéro**. Le serveur ne voit ni le compteur ni les numéros.
- Jusqu'à ce que l'ordre du serveur soit relu (hors ligne), la décision est acceptée sans numéro et se cite par son id : « N décisions acceptées attendent leur numéro : le serveur les donne au retour en ligne, sans doublon possible ». Le numéro venu, les pages qui la citaient par son id la citent par son numéro, les décisions qu'elle remplace deviennent remplacées, les tickets suivent.
- Un numéro donné ne change plus ; un instantané les garde.

## Le brouillon d'un ticket

Il n'y a pas de branche sur le serveur. Ce qu'écrit l'agent d'un ticket (pages, décisions proposées) reste un **brouillon du ticket**, dans le dossier de données de ce poste, que seuls ses agents lisent. Il est versé dans les docs de l'équipe **à la validation du ticket**, jamais avant : c'est le mécanisme de la branche fixe (voir « Spec tenue à jour »), repris tel quel. Les décisions du ticket y sont acceptées sans numéro, puis prennent celui du compteur. La spec de l'équipe ne décrit donc jamais un comportement pas encore fusionné.

## Le choix par projet

Réglages › Projets › Git et worktrees › « Docs du projet d'équipe », pour un projet d'une équipe :

- **« Sur le serveur d'équipe »** (par défaut pour un projet d'équipe qui a son journal) : ce qui précède.
- **« Dans le dépôt »** : `.escouade/` comme avant, partagé par git, la mémoire partagée comprise ; le serveur ne porte que le Kanban.

C'est un choix de l'équipe, gardé dans le journal : le même pour tous ses membres. Le réglage dit avant ce que fait le changement :

- **Passer sur le serveur** importe la spec, les décisions et les documents de `.escouade/` du dépôt, ids et numéros gardés (la prochaine décision acceptée prend le numéro d'après). L'import se fait une fois pour l'équipe ; revenir sur le serveur après un passage par le dépôt y verse ce que le dépôt a changé, en nouvelles versions. **Rien n'est supprimé du dépôt** : l'onglet Docs nomme les fichiers restés dans `.escouade/` et propose « Retirer du dépôt », qui les retire du dossier ; l'utilisateur commite.
- **Revenir au dépôt** écrit dans son `.escouade/` les docs du serveur (git montre ce qui change, l'utilisateur commite) ; le serveur les garde aussi. Rien n'est perdu.

**Rien ne part sans un geste (TS-7).** Rattacher un projet à l'équipe, partager son Kanban ou rejoindre celui de la liste ne versent pas ses docs : tant que l'équipe n'a rien choisi ni rien importé, les docs du projet restent celles de son dépôt, sur chaque poste. Elles partent par ce réglage, ou par **« Verser à l'équipe »** du Kanban, dont l'aperçu les nomme (« Spec, décisions et documents (.escouade/) » : le nombre de pages, de décisions et de documents, chaque fichier, et « Verser aussi la spec, les décisions et les documents », coché) avec les tickets ; l'aperçu dit aussi quand elles ne partent pas (déjà sur le serveur : ce poste les lit de là ; gardées dans le dépôt par l'équipe ; projet privé ; aucune). La mémoire partagée (`.escouade/memory`) part avec elles, par le même geste (« N souvenirs », chaque fichier ; voir « La mémoire d'équipe »). Une fois l'import fait par un poste, ceux qui suivent le journal lisent les docs du serveur.

« Branche des docs » ne s'affiche plus pour un projet dont les docs sont sur le serveur : elles ne suivent aucune branche.

## Pour les agents (MCP)

`get_spec_tree`, `read_spec_page`, `search_spec`, `write_spec_page`, `list_decisions`, `read_decision`, `propose_decision`, `replace_decision`, `search_documents`, `read_document` et `add_document` marchent pareil sur un projet d'équipe, sur la copie du poste. **Ce qu'un collègue a écrit** (ou ce qui a été fusionné avec ce qu'il a écrit) est une donnée de tiers pour un agent : la réponse le dit en tête et le met entre `<third_party_data>` et `</third_party_data>`, `<` échappé, comme ce qui vient d'un hébergeur de code. Ce que ce poste a écrit vient tel quel.

## Les droits

- Tout membre qui a accès au projet écrit la spec et accepte une décision : aucun rôle ne le restreint (le rôle « lecteur » de §2.6 n'existe pas encore). Un projet gardé à quelques membres n'est lu que d'eux (la clé de son groupe).
- **Un membre retiré** ne lit plus rien de ce qui est écrit après son départ (la rotation de TS-2) ; ce qu'il avait lu reste sur son poste.
- **Un projet privé ne part jamais** sur le serveur : le choix ne s'y propose pas.

## Limites

- Les écarts des comparaisons au code et le texte extrait restent par poste.
- Un document de plus de 20 Mo ne part pas.
- L'auteur d'une version n'est pas nommé dans les conflits (« un collègue ») : l'annuaire chiffré des noms est TS-4.

## Ce que le poste refuse à la lecture (SR3-01)

Ce qu'un collègue écrit au journal est jugé sur le poste qui le lit, **avant tout accès au disque**, comme ce que ce poste écrit lui-même : un membre ne fait jamais écrire un fichier hors du dossier des docs chez un autre.

- **Une page** : son chemin doit être celui qu'une page écrite ici aurait (`docs::page_path`, le même jugement : des segments en minuscules et tirets, 4 niveaux au plus, ni `..`, ni chemin absolu, ni lecteur `C:`, ni partage `\\serveur`, ni `\`). Sinon elle est écartée.
- **Une décision** : son id nomme son fichier (`<id>.md`, `NNNN-<id>.md`). La forme d'un id : un seul nom de fichier, de 120 caractères au plus, sans `/`, `\`, `:`, ni caractère que Windows refuse dans un nom (`< > " | ? *`), sans caractère de contrôle, sans point en tête, ni point ou espace en fin, et pas un nom que Windows garde (`con`, `nul`…). Ce que fabrique Escouade (`decision_id`, le nom d'un fichier écrit à la main) l'a toujours. Une décision d'un autre id est écartée.
- **La résolution d'un conflit** (« Garder la mienne », « Prendre la sienne ») n'écrit la page qu'à un chemin de page, jugé de nouveau au moment d'écrire.
- **Les fichiers de la spec et des décisions** ne se lisent, ne s'écrivent et ne se retirent qu'à un chemin jugé avant (`DocFile`) : relatif, sans segment vide, `.`, `..`, absolu, lecteur ou partage.
- **Ce poste ne publie pas** une page ou une décision qui serait écartée chez les autres (un fichier nommé à la main hors de ces formes reste sur ce poste).

**Écartée, la lecture continue** : les autres pages, décisions et documents du même passage arrivent ; une page ou une décision qui ne s'écrit pas (un disque qui refuse) n'arrête plus le passage non plus.

**L'alarme** : en tête de l'onglet Docs, un bandeau rouge, sur le poste d'un **administrateur** et sur **l'appareil qui l'a écrite** : « Une page venue de l'équipe a été écartée : son chemin « … » sort du dossier de la spec. Rien n'en a été écrit sur ce poste ; les autres pages se lisent. Écrite par l'appareil 0123abcd. » (ou « Écrite par ce poste. »), et de même pour une décision (« son id « … » n'est pas celui d'une décision »). Un autre membre ne la voit pas ; le journal du poste la note toujours. Le bandeau dit ce qu'a écarté la dernière lecture : il part quand le journal n'en a plus.
