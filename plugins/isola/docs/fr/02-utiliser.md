# Utiliser isola

isola lance les services de chaque worktree sur ses propres ports et ses adresses `*.localhost`. Deux tickets testés en même temps ne se marchent pas dessus : chacun a son front, son API, sa base.

## Tester un ticket

« ▶ Tester » est sur la carte « À tester », dans l’en-tête de la conversation de l’agent et dans le bilan des critères. Avec isola :

1. La fenêtre « Tester » montre le `.isola.toml` du worktree sous « Configuration isola (.isola.toml) », avec l’avertissement : isola lance les commandes de ce fichier dans ton shell, hors du mode de permission du harness, et l’agent peut l’avoir écrit ou modifié. Elle montre aussi l’adresse qui s’ouvrira.
2. Rien ne tourne avant **Lancer**. Approuvé une fois, le fichier n’est plus montré tant qu’il ne change pas ; modifié, il l’est de nouveau.
3. Escouade lance `isola up` dans un terminal du projet. Son entrée n’est pas un terminal : isola ne t’y demande jamais de mot de passe (pour faire confiance à un certificat, il dit comment faire).
4. Quand `isola up` a fini, Escouade demande au plugin les services du worktree (`isola ls --json`, ceux de la branche du worktree). Chaque service a sa ligne dans la fenêtre.
5. Escouade attend que chaque service réponde, 3 minutes au plus, puis ouvre l’adresse que l’agent a donnée, ou à défaut celle du premier service qui en a une.

**Tout arrêter** lance `isola down`.

## Les services, leurs ports et leurs adresses

Pour chaque service du worktree, isola donne :

- son **adresse** : celle de son proxy (`http://ticket-dem-1.demo.localhost:3000`), sinon celle de son propre port (`http://localhost:8117`) ;
- son **port à lui** : c’est lui qu’Escouade sonde pour savoir si le service est prêt, parce que le proxy répond (avec une page d’erreur) avant le service ;
- son **état**, comme isola le dit (`running`, `stopped`…).

Un service sans adresse (un processus de fond) est compté prêt dès qu’il tourne. Un service qui ne tourne pas fait échouer le test : « {nom} ne tourne pas ».

Les ports et les adresses sont ceux qu’isola choisit d’après le `.isola.toml` : Escouade ne les fixe pas, et ne réserve pas de ports pour un worktree d’isola.

## Quand les services s’arrêtent

Escouade lance `isola down` pour le worktree :

- quand tu cliques **Tout arrêter**, ou quand les lancements de test de l’agent s’arrêtent (ticket validé, agent archivé) ;
- quand l’agent est supprimé en gardant son worktree ;
- quand le projet est fermé.

Ces arrêts se font en arrière-plan : un échec va seulement au log de l’app.

## Quand le worktree s’en va

Avant de supprimer un worktree (agent supprimé avec son worktree, nettoyage du Kanban, « Nettoyer »), Escouade lance les commandes « Avant sa suppression » du projet, puis `isola destroy`, qui détruit les services et ce qu’ils gardent (leurs bases). Un échec est signalé une fois l’agent supprimé : « Agent supprimé, mais … ».

## Ses limites

- Chaque appel à isola (`ls`, `down`, `destroy`) a 60 secondes. Au-delà, Escouade l’arrête.
- isola ne voit que les worktrees qui ont un `.isola.toml` à leur racine.
- Seul un plugin livré avec Escouade peut être un environnement de lancement : `isola up` est une commande du terminal de l’app, qu’un plugin venu d’ailleurs ne reçoit pas.
- Il ne lit pas les journaux des services : ils restent ceux d’isola.
