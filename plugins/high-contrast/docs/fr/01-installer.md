# Installer et utiliser Contraste élevé

## L’installer

1. Dans Escouade, ouvre la vue Plugins (l’icône en forme de pièce de puzzle de la barre des projets).
2. **Store**, cherche « Contraste élevé » (ou la catégorie **Thèmes**), puis **Installer**. Tu peux aussi partir de sa page sur <https://store.escouade.dev/plugins/dev.escouade.high-contrast> et **Ouvrir dans Escouade**.
3. La fenêtre « Installer Contraste élevé ? » dit « Sans programme : rien de lui ne s’exécute, ses contributions sont lues », et ce qu’il ajoute : un thème, dans Réglages › Général › Couleurs. **Installer**.

## Le choisir

1. **Réglages › Général**, rangée **Couleurs** (sous **Thème**).
2. Choisis **Contraste élevé**.

Le thème s’applique tout de suite : la fenêtre, les terminaux ouverts (sans les rouvrir) et l’éditeur. Il est gardé avec tes réglages et posé avant le premier affichage au démarrage suivant.

**Thème**, juste au-dessus, choisit toujours entre **Système**, **Clair** et **Sombre** : Contraste élevé a une version pour chacun.

Pour revenir aux couleurs d’Escouade, choisis **Escouade** dans **Couleurs**.

## Ce qu’il change

- Les surfaces, les bordures, les textes, l’accent, le focus et les couleurs d’état (en cours, en attente, terminé, ajouts et suppressions de code). En sombre, le fond est noir et l’accent jaune ; en clair, le fond est blanc et l’accent bleu.
- La palette de couleurs de projet : six couleurs (jaune, cyan, magenta, vert, orange, violet) remplacent celle d’Escouade dans l’assistant « Nouveau projet », dans Réglages › Projets › Projet et dans le menu d’une tuile. Un projet garde sa couleur : si elle n’est pas dans la palette, elle y est ajoutée à la fin.

Ce qu’il ne donne pas garde la couleur d’Escouade : le bouton plein (`--primary`), la coloration du code (`--syn-*`), les couleurs ANSI des terminaux (`--ansi-*`), l’historique de git (`--lane-*`). La forme (polices, rayons, largeurs) reste toujours celle d’Escouade.

## Ce qu’Escouade garde, et où

- Ses fichiers dans `~/.escouade/plugins/dev.escouade.high-contrast`, et son installation dans `~/.escouade/plugins.json`.
- Ton choix de couleurs, avec les réglages d’Escouade.

Il n’a ni secret, ni réglage, ni journal (« Un plugin sans programme n’a pas de journal »).

## Si ça ne marche pas

**Le choix « Contraste élevé » n’est pas dans Couleurs.**
Le plugin est désactivé pour l’app : réactive-le dans la vue Plugins, **Installés**. Un thème n’apparaît que si son plugin est installé et activé.

**« Le thème choisi n’est plus là : son plugin (Contraste élevé) est désactivé ou désinstallé. Escouade a repris ses couleurs. »**
Le plugin a été désactivé, désinstallé, ou ses fichiers ont changé (sa ligne dans **Installés** dit alors « Ses fichiers ont changé depuis son installation (empreinte …) : installe-le à nouveau. »). Ton choix reste : le plugin réactivé ou réinstallé, son thème revient.

**« Une valeur de ce thème est écartée : ce n’est pas un jeton ou une couleur permise. »**
Escouade a refusé une de ses valeurs, qui garde la couleur d’Escouade. Avec la version publiée, ça ne devrait pas arriver : signale-le sur sa page du store.
