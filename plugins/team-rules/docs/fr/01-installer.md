# Installer Règles de l’équipe

## 1. L’installer

1. Dans Escouade, ouvre la vue Plugins (l’icône en forme de pièce de puzzle de la barre des projets).
2. **Store**, cherche « Règles de l’équipe », puis **Installer**. Tu peux aussi partir de sa page sur <https://store.escouade.dev/plugins/dev.escouade.team-rules> et **Ouvrir dans Escouade**.
3. La fenêtre « Installer Règles de l’équipe ? » dit ce qu’il pourra faire : tourner dans Escouade, isolé, et, en rouge, « Décide des autorisations de tes agents ». **Installer**.

Il lui faut Escouade 2.2.0 ou plus récent.

Escouade le démarre aussitôt, et ensuite avec l’app : sa première décision n’attend pas qu’il démarre.

## 2. Choisir les projets

Onglet **Projets** de sa page :

- il est actif dans chaque projet qui n’est pas privé : les agents de ces projets lui sont soumis ;
- un projet privé est coupé d’office : ses agents ne lui sont soumis que si tu l’y actives ;
- un `.escouade.yml` ne l’active jamais : seul ton geste le fait.

## 3. Lire ses règles

Il n’a pas de réglage. Ses règles sont celles du `rules.json` publié avec lui ; l’onglet Utiliser les détaille. Pour d’autres règles, fais ta propre copie du plugin (l’onglet Utiliser dit comment).

## Ce qu’Escouade garde, et où

- **Le plugin** : ses fichiers, `rules.json` compris, dans `~/.escouade/plugins/dev.escouade.team-rules`, et son installation (version, empreinte, tes choix par projet) dans `~/.escouade/plugins.json`. Son module WASM compilé est gardé dans le cache d’Escouade, et refait s’il disparaît.
- **Son journal** : chaque décision, dans l’onglet **Journal** de sa page (filtre « Décision ») et en entier dans `~/.escouade/plugin-logs/dev.escouade.team-rules.log`.

Il n’a ni secret ni réglage. Désinstaller efface ses fichiers et son journal.
