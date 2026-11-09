# Dépannage d’isola

Les messages qu’affichent le plugin et Escouade pour lui, ce qu’ils veulent dire et quoi faire. Ils sont dans la langue de l’interface ; `{…}` est remplacé par la commande, la raison ou la valeur du moment.

## « ▶ Tester » ne propose pas isola

Le ticket part avec une recette sur les ports réservés, comme sans isola. Un worktree n’est celui d’isola que si les trois conditions tiennent :

- le plugin est actif pour l’app et pour le projet (onglet **Projets** ; un projet privé est coupé d’office) ;
- le worktree a un `.isola.toml` à sa racine ;
- `isola` est trouvé sur le PATH (sous Windows, un `isola.exe` ou `isola.com`).

Escouade relit le PATH au plus toutes les 30 secondes : après avoir installé isola, attends un peu. Si Escouade a été lancé avant que tu modifies le PATH, relance-le.

## Pendant un test

**« isola ne lance pas ce worktree (isola introuvable, ou pas de .isola.toml). »**
Le programme ou le fichier a disparu depuis l’ouverture de la fenêtre. Vérifie les deux, puis relance « ▶ Tester ».

**« La configuration d’isola (.isola.toml) n’a pas été approuvée : lance « ▶ Tester » pour la lire avant qu’elle tourne. »**
Le fichier a changé depuis ton accord, ou tu ne l’as jamais approuvé. Relance « ▶ Tester », relis-le, puis **Lancer**.

**« .isola.toml est trop gros pour être relu avant le lancement ({n} octets). »**
Escouade ne montre que des fichiers de 256 Kio au plus, en entier : il n’en coupe jamais un. Allège le fichier.

**« .isola.toml n’est pas en UTF-8. »** / **« .isola.toml illisible : {raison} »**
Le fichier n’a pas pu être lu tel quel. Réenregistre-le en UTF-8, vérifie ses droits.

**« isola up en échec (code {code}) »**
`isola up` s’est arrêté en erreur. Clique **Voir le log** : c’est la sortie d’isola, dans le terminal du test. La cause est le plus souvent dans le `.isola.toml` ou dans l’outil qu’isola utilise.

**« isola ls --json a échoué : {dernière ligne} »**, **« isola down a échoué : … »**, **« isola destroy a échoué : … »**
isola a rendu un code d’erreur. La fin du message est la dernière ligne non vide de ce qu’il a écrit en erreur. Lance la même commande dans le worktree, dans un terminal, pour voir le reste.

**« isola {commande} n’a pas répondu en 60 s »**
isola n’a pas fini à temps : Escouade l’a arrêté. Lance la commande à la main dans le worktree pour voir où elle bloque.

**« isola {commande} n’a pas pu démarrer : {raison} »**
Escouade n’a pas pu lancer le programme. Les raisons possibles :
- `no program isola on this machine` : `isola` n’est plus sur le PATH (sous Windows, seul un `.exe` ou un `.com` compte) ;
- `4 programs at once at most` : le plugin a déjà quatre commandes en cours ; réessaie quand elles ont fini ;
- `its instance has ended` : le plugin s’arrêtait ; réessaie.

**« réponse d’isola illisible : {raison} »**
`isola ls --json` n’a pas écrit du JSON. Ta version d’isola ne donne peut-être pas la même sortie : lance `isola ls --json` dans le worktree et regarde ce qu’il écrit.

**« Pas de réponse de {adresse} après 3 min »**
Un service tourne mais ne répond pas sur son port. Regarde sa sortie avec les outils d’isola, et vérifie le port que lui donne le `.isola.toml`.

**« {service} ne tourne pas »**
isola liste le service, mais pas dans l’état `running`. Son état est écrit sur sa ligne.

**« Le plugin dev.escouade.isola ne répond pas : {raison} »**
Le module du plugin n’a pas pu être appelé (arrêté, coupé, ou planté). Regarde l’onglet **Journal** de sa page, puis **Relancer**.

**Son état ou son journal dit `its module …plugin.wasm: …`**
Cette version d’Escouade n’a pas le module d’isola : il arrive dans les installeurs avec sa version du store. En attendant, coupe le plugin (pour l’app ou pour le projet) : les tickets repartent avec une recette sur les ports réservés.

## À la suppression d’un worktree

**« Agent supprimé, mais isola destroy a échoué : {dernière ligne} »**
L’agent et son worktree sont supprimés, mais isola n’a pas détruit ses services. Lance `isola ls` pour voir ce qui reste, puis détruis-le avec les commandes d’isola.

Un `isola down` lancé en arrière-plan (fin d’un test, projet fermé) qui échoue ne s’affiche pas : il est écrit au log de l’app.

## Le journal

L’onglet **Journal** de la page du plugin montre chaque lancement (`proc/spawn: ran isola down in …`), chaque refus et chaque fin de programme. Le fichier entier est `~/.escouade/plugin-logs/dev.escouade.isola.log`.
