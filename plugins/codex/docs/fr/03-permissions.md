# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `codex` et lance `codex app-server`. Le plugin décrit le lancement (ses options `-c`, la variable du jeton, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé dans le lancement, une discussion tenue en lecture seule. Une description qui ne tient pas est refusée et l’agent ne démarre pas.

## Ce qu’il lit

Par l’hôte d’Escouade, qui juge chaque chemin et refuse ceux d’un projet clôturé :

- **le dépôt de l’agent** (`agentRepo`, en lecture) : pour savoir si sa configuration et, pour un commit, ses hooks sont ceux que Codex protège (`fs/gitProtected`). Le plugin ne lit pas `.git` lui-même : l’hôte répond oui ou non ;
- **le dossier du compte** (`agentHome`, en lecture) : seulement `config.toml`, et seulement quand le serveur d’Escouade est refusé à l’agent, pour couper une entrée à toi du même nom.

Il reçoit aussi ce que Codex écrit et, pour la vue Codex, les lignes du journal de la session.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin met le jeton dans une variable du lancement (`ESCOUADE_MCP_TOKEN`), que Codex lit (`bearer_token_env_var`) : jamais dans les arguments. Il n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune connexion de compte.** Il ne lit jamais `auth.json`, même si c’est un fichier du dossier qu’il peut lire.

## Ce qu’il accepte seul, et ce qui va à la carte

Le plugin accepte sans carte une seule chose : **`git add` et `git commit` du worktree de l’agent, en Édits auto et en Auto**. Il faut tout cela à la fois :

- la demande vient du process de l’agent, pas de la vue Codex ;
- la commande tourne dans le dossier de l’agent ;
- elle n’est que `git add` et `git commit`, lue comme le shell la lirait, sur la commande que Codex va lancer et non sur son résumé : refusée au moindre doute (un opérateur autre que `&&` entre deux commandes acceptées, `$`, un accent grave, un saut de ligne, une option de git avant sa sous-commande) ;
- la configuration du dépôt est protégée, et pour un commit ses hooks sont ceux de son `.git` ;
- pour un agent clôturé, le commit ne prend rien dans un fichier (`-F`, `--file`, `--template`…).

Ces commandes tournent hors du bac à sable de Codex, qui n’écrit jamais dans `.git` : c’est pourquoi tout autre forme demande.

Toute autre demande de Codex (une commande, une modification de fichiers, plus de permissions) est une carte. Ce que Codex fait sans demander dépend de son bac à sable :

| Mode | Sans carte |
|---|---|
| Plan | Les lectures et les commandes qui ne font que lire, dans son bac à sable en lecture seule. |
| Demander | Ce que son bac à sable permet : écrire dans le dossier de l’agent, ses commandes sans réseau. |
| Édits auto | Pareil, et `git add` et `git commit` du worktree, acceptés par le plugin. |
| Auto | Pareil ; les autres demandes, son relecteur automatique les décide, et la conversation le dit. |
| Bypass | Tout : Codex ne demande rien et n’a pas de bac à sable. |

## La clôture d’un projet privé

Le plugin déclare à Codex, par ses options, deux profils de permissions où chaque chemin clôturé est refusé (`deny`), l’un pour le mode Plan et la discussion, l’autre pour les modes qui écrivent. Le bac à sable de Codex les applique à toutes ses commandes. En Bypass, il n’y a pas de profil : Codex n’a plus de bac à sable, et la clôture ne tient plus pour ses commandes.

Si le plugin redémarre, Escouade lui redonne la clôture et la lecture seule de chaque agent : elles tiennent toujours. Le process de Codex, lui, continue avec ses profils.

## Ce qu’il ne peut pas faire

- lancer `codex` ou tout autre programme ;
- ouvrir une connexion réseau ;
- écrire un fichier, ou lire hors du dépôt de l’agent et du dossier de son compte ;
- lire un fichier d’un projet clôturé : l’hôte le refuse, quoi que dise son code ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- accepter autre chose que `git add` et `git commit`, comme dit plus haut.
