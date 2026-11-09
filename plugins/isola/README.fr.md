# isola

Chaque worktree a ses propres services, sur ses propres ports et ses adresses `*.localhost`, grâce à [isola](https://github.com/cyucelen/isola). Quand le projet a un `.isola.toml`, « ▶ Tester » lance `isola up`, attend que chaque service réponde et ouvre l’adresse. Escouade arrête les services avec `isola down` et les détruit avec `isola destroy` quand le worktree s’en va.

**Pour qui** : un projet dont les tickets se testent en lançant plusieurs services (un front, une API, une base), et qui ne veut pas d’une recette sur des ports réservés pour chaque agent.

**Ce qu’il demande** :

- le programme `isola` sur ton PATH (sous Windows, un `isola.exe`) ;
- un fichier `.isola.toml` commité à la racine du projet.

Pas de compte, pas de secret, pas de réseau.

**Ce qu’il peut faire** : lancer, dans le worktree d’un agent et nulle part ailleurs, trois commandes exactes : `isola ls --json`, `isola down` et `isola destroy`. Escouade refuse toute autre commande. `isola up`, c’est Escouade qui le lance, dans un terminal, après que tu as relu le `.isola.toml`.

Il est livré avec Escouade et actif dans chaque projet, sauf les projets privés, où tu l’actives toi-même.

La suite est dans les onglets de la documentation :

1. Installer : ce qu’il faut sur la machine et dans le projet.
2. Utiliser : les services d’un worktree, leurs ports et leurs adresses.
3. Permissions : ce qu’il lance, ce qu’il lit, ce qu’il ne fait jamais.
4. Dépannage : ses messages et quoi faire.
5. Développer : son dossier, ses tests.
