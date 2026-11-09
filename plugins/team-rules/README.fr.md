# Règles de l’équipe

Autorise ou refuse les commandes de tes agents selon des règles écrites dans un fichier, `rules.json`, avant qu’elles n’arrivent jusqu’à toi. Une commande interdite est refusée avec un message à l’agent, une commande sûre passe, et le reste t’est demandé comme sans le plugin.

**Pour qui** : toi ou ton équipe, quand les mêmes demandes d’autorisation reviennent (`npm test`, `git status`) et que tu veux en interdire d’autres (`rm -rf`, `git push --force`), sans passer tes agents en mode Bypass.

**Ce qu’il demande** : rien à régler. Ses règles sont celles de son `rules.json`. Pour des règles à toi, il faut ta propre copie du plugin (voir l’onglet Utiliser).

**Ce qu’il peut faire** :

- **décider des autorisations de tes agents** : c’est la permission la plus forte d’un plugin, montrée en rouge à l’installation ;
- faire tourner son programme dans Escouade, isolé (en WASM) : pas de Node à installer.

Il ne se connecte à rien, ne garde aucun secret, ne lit aucun fichier hors de son `rules.json`. Il ne juge que les commandes ; une modification de fichier, une page web ou un outil MCP te sont toujours demandés.

La suite est dans les onglets de la documentation :

1. Installer : l’installer et choisir ses projets.
2. Utiliser : les règles, et ce qu’elles décident.
3. Permissions : ce qu’il voit, ce qu’il décide, ce qu’il ne peut pas faire.
4. Dépannage : son journal et ses messages.
5. Développer : son dossier, ses tests, son build.
