---
title: "Raccourcis"
subject: "Fenêtre"
sources: ["docs/SPEC.md","docs/INBOX.md (§ 8.2, § 8.3, choix I4)","design/inbox-2-3/README.md (direction B)"]
tickets: ["ESC-392","ESC-432","ESC-464","ESC-468","ESC-510","ESC-539"]
decisions: [128,158,176,181,240,218]
compared: null
---

## Ce que c'est

Les raccourcis clavier d'Escouade, par défaut. Ils couvrent l'app entière, la conversation, la dictée, l'Overseer, l'éditeur et les fenêtres. Dans un terminal, les touches du shell lui restent, sauf quelques raccourcis de l'app ; dans la vue VS Code de l'éditeur, celles de VS Code, sauf celles qu'Escouade garde. Un seul est global, hors de la fenêtre : celui de l'Overseer.

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

### Overseer

- **Global**, depuis n'importe où, Escouade au premier plan ou non : `Alt+Maj+O` sous Windows et Linux, `⌥⌘O` sous macOS, ouvre son écran (la fenêtre revient au premier plan). Il ne vaut que l'Overseer allumé, se change ou se retire dans Réglages › Overseer › « Raccourci global », et un raccourci déjà pris par une autre app y est refusé. Sous Wayland, il n'existe pas : un raccourci du bureau lance `xdg-open escouade://overseer` (voir *L'Overseer*).
- Dans son écran : `Entrée` envoie, `Maj+Entrée` va à la ligne ; `Espace` maintenu, le champ vide, fait parler, relâché il envoie ; `Échap` annule ce qui est dit ou arrête sa voix, sinon ferme l'écran.

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

- `Échap` : revenir à la conversation de l'agent, sauf dans un champ qui contient du texte ou dans la vue VS Code.
- `Ctrl+Entrée`, dans « Lui écrire » : envoyer le message ; il ne répond à aucune demande qui attend.

### Éditeur (la vue VS Code)

- Les touches de VS Code lui restent : `Ctrl+S`, `Ctrl+F`, `Ctrl+H`, `Ctrl+Maj+F`, `Ctrl+Maj+P` et `F1`, `Ctrl+K` et ses accords, `Ctrl+B`, `Ctrl+Tab`, `Ctrl+Maj+L`, `Ctrl+W`, `F12`, `Alt+clic`…
- L'app y garde celles qui mènent ailleurs dans l'app ou ouvrent une de ses fenêtres : `Ctrl+P` (sur l'index du cœur, le fichier dans un onglet de la vue), `Ctrl+1..9`, `Ctrl+N`, `Ctrl+Maj+N`, `Ctrl+J`, `Ctrl+T`, `Ctrl+,`, `Ctrl+Maj+A`, `Ctrl+Maj+B`, le zoom, `Ctrl+\`, la dictée et, l'écran partagé ouvert, `Ctrl+Maj+\`, `Ctrl+Maj+W` et `Ctrl+Alt+flèche`. VS Code ne les reçoit pas et ne les affiche plus ; leurs commandes restent dans sa palette. La règle et ce que chaque touche faisait dans VS Code : *Éditeur › Les touches de la vue et celles d'Escouade*.

### Fenêtres

- Fenêtre d'un ticket : `Ctrl+Entrée` ajoute ou enregistre le ticket depuis n'importe quel champ.
- Réglages : `Alt+↑` / `Alt+↓` déplacent une commande de préparation ou de démontage.
- Barre des projets : `Alt+↑` / `Alt+↓` déplacent la tuile qui a le focus (voir *Barre des projets*).

### Dans un terminal

- Les touches du shell lui restent (`Ctrl+C`, `Ctrl+R`, `Ctrl+N`, `Ctrl+K`, `Ctrl+P`…).
- `Alt+←` / `Alt+→` reculent / avancent d'un mot : le shell reçoit `Ctrl+←` / `Ctrl+→` (sous macOS, `Échap b` / `Échap f`, comme readline). Hors macOS, `Alt+↑` / `Alt+↓` lui arrivent en `Ctrl+↑` / `Ctrl+↓`. Avec un autre modificateur en plus, la touche lui arrive telle quelle. Un log ne les prend pas.
- L'app y prend `Ctrl+1..9`, `Ctrl+Tab`, `Ctrl+J`, `Ctrl+Maj+J`, `Ctrl+,`, `Ctrl+Maj+A`, le zoom (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`), `Ctrl+\` (qui y serait SIGQUIT) et, l'écran partagé ouvert, `Ctrl+Maj+\`, `Ctrl+Maj+W` et `Ctrl+Alt+flèche`.
- Sous macOS, le shell ne reçoit jamais `Cmd` : les raccourcis de l'app y marchent, et `Cmd+C`, `Cmd+V`, `Cmd+A` restent au terminal (copier, coller, tout sélectionner).
- Le raccourci global de l'Overseer est pris par le système avant tout terminal.

### Espaces (ESC-539)

Dès que ce poste connaît un serveur (voir *Les espaces*), lus par la place de la touche (AZERTY compris), jamais avec `Maj` ni AltGr, ni depuis le pavé numérique :

- `Alt+1..9` (`⌃1..9` sous macOS, où `Alt` tape un caractère) : le n-ième espace, ce poste étant le premier.
- `Ctrl+²` (la touche sous Échap, de code `Backquote`) : l'espace suivant.
- Ils valent dans un terminal et dans la vue VS Code (qui ne reçoit plus `Alt+1..9` ni `Ctrl+²`), jamais dans un dialogue. **Une question à l'écran, là où l'on y répond, garde `Alt+1..9` pour ses options**, comme avant. Avec un seul espace, ils ne prennent rien.
