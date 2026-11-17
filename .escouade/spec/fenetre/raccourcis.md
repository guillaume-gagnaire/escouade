---
title: "Raccourcis"
subject: "Fenêtre"
sources: ["docs/INBOX.md (§ 8.2, § 8.3, choix I4)","design/inbox-2-3/README.md (direction B)"]
tickets: ["ESC-392","ESC-432","ESC-510"]
decisions: [128,158,"l-ecran-de-la-boite-a-toi-une-vue-de-tou"]
compared: null
---

## Ce que c'est

Les raccourcis clavier d'Escouade, par défaut. Ils couvrent l'app entière, la conversation, la dictée, l'éditeur et les fenêtres. Dans un terminal, les touches du shell lui restent, sauf quelques raccourcis de l'app.

## Comportement

### Partout

- Sous macOS, `Ctrl` devient `Cmd`, sauf dans `Ctrl+Tab`.
- `Ctrl+Alt` (AltGr) n'est jamais un raccourci, sauf `Ctrl+Alt+flèche` de l'écran partagé : une flèche ne tape rien.

### App

- `Ctrl+1..9` : les projets.
- `Ctrl+Tab` / `Ctrl+Maj+Tab` : agent suivant / précédent.
- `Ctrl+N` : nouvel agent.
- `Ctrl+T` : nouveau terminal.
- `Ctrl+J` : le focus sur la carte du prochain agent en attente ; `1`, `2`… y répondent, `Entrée` l'ouvre (voir *Barre latérale*).
- `Ctrl+Maj+J` : ouvrir ou refermer la boîte « À toi », seulement quand le serveur MCP est allumé ; éteint, il ne fait rien (voir *La boîte « À toi »*).
- `Ctrl+K` : rechercher dans les conversations.
- `Ctrl+P` : ouvrir un fichier.
- `Ctrl+Maj+A` : vue d'ensemble.
- `Ctrl+Maj+B` : panneau des fichiers.
- `Ctrl+Maj+L` : disposition.
- `Ctrl+\` (la touche de code `Backslash`, `*` en AZERTY) : partager l'écran ou revenir à un volet ; `Ctrl+Maj+\` : tourner la séparation du volet actif (voir *Écran partagé*).
- Écran partagé : `Ctrl+Alt+←→↑↓` passe au volet de ce côté, `Ctrl+Maj+W` ferme le volet actif.
- `Ctrl+,` : réglages.
- `Ctrl+=` / `Ctrl+-` / `Ctrl+0` : zoom de l'interface, une fenêtre ouverte aussi (voir *Fenêtre*).

### Conversation

- `Ctrl+Entrée` / `Ctrl+Maj+Entrée` : autoriser / toujours autoriser.
- `Alt+1..9` : une option d'une question ; `Ctrl+Entrée` pour valider.
- `Échap` : interrompre.
- `↑`, champ vide : rappeler le dernier message.

### Boîte « À toi »

Hors d'un champ, d'un menu ou d'une boîte de dialogue, sans modificateur :

- `J` / `K` : la demande suivante / précédente de la file (« 3 sur 9 ») ; `↓` / `↑` aussi, hors de la file ; dans la file, `↓` / `↑` passent d'une ligne à l'autre.
- `1` à `6` : une option d'un choix (le focus de la réponse s'y déplace ; avec `multi`, elle se coche ou se décoche).
- `Entrée` : le geste principal de la demande (« Choisir B », « Oui », « C'est fait »), sauf sur un bouton ou un lien qui a le focus. Dans le champ « Un mot pour l'agent », `Entrée` l'envoie aussi ; dans une réponse libre, `Ctrl+Entrée`.
- `L` : « Plus tard ».
- `Échap` : refermer la boîte.
- Comparaison en grand : `1` `2` `3` choisissent, `←` `→` passent d'une option à l'autre, `Espace` montre l'option seule dans la visionneuse, `Entrée` choisit, `Échap` ferme.
- Lecture d'un PDF : `←` `→` (ou `Page précédente` / `Page suivante`), `Début`, `Fin` changent de page ; `Échap` quitte le plein écran.

### Dictée

- `Ctrl+Maj+Espace` : commencer ou finir ; maintenu, la dictée dure tant qu'il est enfoncé. Dans le champ de message, la description d'un ticket, le champ d'une réponse de la boîte « À toi » ou le terminal de la vue Claude Code. Il se change dans Réglages › Voix › « Raccourci » (voir *Dictée*).
- `Échap` : l'annuler.

### Vue d'un sous-agent

- `Échap` : revenir à la conversation de l'agent, sauf dans un champ qui contient du texte.
- `Ctrl+Entrée`, dans « Lui écrire » : envoyer le message ; il ne répond à aucune demande qui attend.

### Éditeur

- `Ctrl+S` : enregistrer.
- `Ctrl+F` : chercher dans le fichier.
- `Ctrl+Maj+F` : rechercher dans les fichiers.
- `Ctrl+clic` / `F12` : aller à la définition.
- `Maj+F12` : trouver les références.
- `Alt+←` / `Alt+→` : revenir / avancer (sous macOS : `⌃-` / `⌃⇧-`, touche Contrôle).
- `Alt+clic` : ajouter un curseur.
- `F2` : renommer la ligne de l'arborescence ; `Suppr` (`⌘⌫`) : la supprimer.

### Fenêtres

- Fenêtre d'un ticket : `Ctrl+Entrée` ajoute ou enregistre le ticket depuis n'importe quel champ.
- Réglages : `Alt+↑` / `Alt+↓` déplacent une commande de préparation ou de démontage.
- Barre des projets : `Alt+↑` / `Alt+↓` déplacent la tuile qui a le focus (voir *Barre des projets*).

### Dans un terminal

- Les touches du shell lui restent (`Ctrl+C`, `Ctrl+R`, `Ctrl+N`, `Ctrl+K`, `Ctrl+P`…).
- `Alt+←` / `Alt+→` reculent / avancent d'un mot : le shell reçoit `Ctrl+←` / `Ctrl+→` (sous macOS, `Échap b` / `Échap f`, comme readline). Hors macOS, `Alt+↑` / `Alt+↓` lui arrivent en `Ctrl+↑` / `Ctrl+↓`. Avec un autre modificateur en plus, la touche lui arrive telle quelle. Un log ne les prend pas.
- L'app y prend `Ctrl+1..9`, `Ctrl+Tab`, `Ctrl+J`, `Ctrl+Maj+J`, `Ctrl+,`, `Ctrl+Maj+A`, le zoom (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`), `Ctrl+\` (qui y serait SIGQUIT) et, l'écran partagé ouvert, `Ctrl+Maj+\`, `Ctrl+Maj+W` et `Ctrl+Alt+flèche`.
- Sous macOS, le shell ne reçoit jamais `Cmd` : les raccourcis de l'app y marchent, et `Cmd+C`, `Cmd+V`, `Cmd+A` restent au terminal (copier, coller, tout sélectionner).
