---
title: "Raccourcis"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Les raccourcis clavier d'Escouade, par défaut. Ils couvrent l'app entière, la conversation, la dictée, l'éditeur et les fenêtres. Dans un terminal, les touches du shell lui restent, sauf quelques raccourcis de l'app.

## Comportement

### Partout

- Sous macOS, `Ctrl` devient `Cmd`, sauf dans `Ctrl+Tab`.
- `Ctrl+Alt` (AltGr) n'est jamais un raccourci.

### App

- `Ctrl+1..9` : les projets.
- `Ctrl+Tab` / `Ctrl+Maj+Tab` : agent suivant / précédent.
- `Ctrl+N` : nouvel agent.
- `Ctrl+T` : nouveau terminal.
- `Ctrl+J` : le focus sur la carte du prochain agent en attente ; `1`, `2`… y répondent, `Entrée` l'ouvre (voir *Barre latérale*).
- `Ctrl+K` : rechercher dans les conversations.
- `Ctrl+P` : ouvrir un fichier.
- `Ctrl+Maj+A` : vue d'ensemble.
- `Ctrl+Maj+B` : panneau des fichiers.
- `Ctrl+Maj+L` : disposition.
- `Ctrl+,` : réglages.
- `Ctrl+=` / `Ctrl+-` / `Ctrl+0` : zoom de l'interface, une fenêtre ouverte aussi (voir *Fenêtre*).

### Conversation

- `Ctrl+Entrée` / `Ctrl+Maj+Entrée` : autoriser / toujours autoriser.
- `Alt+1..9` : une option d'une question ; `Ctrl+Entrée` pour valider.
- `Échap` : interrompre.
- `↑`, champ vide : rappeler le dernier message.

### Dictée

- `Ctrl+Maj+Espace` : commencer ou finir ; maintenu, la dictée dure tant qu'il est enfoncé. Dans le champ de message, la description d'un ticket ou le terminal de la vue Claude Code. Il se change dans Réglages › Voix › « Raccourci » (voir *Dictée*).
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
- L'app y prend `Ctrl+1..9`, `Ctrl+Tab`, `Ctrl+J`, `Ctrl+,`, `Ctrl+Maj+A` et le zoom (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`).
- Sous macOS, le shell ne reçoit jamais `Cmd` : les raccourcis de l'app y marchent, et `Cmd+C`, `Cmd+V`, `Cmd+A` restent au terminal (copier, coller, tout sélectionner).
