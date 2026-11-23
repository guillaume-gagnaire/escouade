---
number: 299
id: "l-ouverture-par-le-systeme-bornee-aux-do"
title: "L'ouverture par le système bornée aux dossiers des projets et des données, les balises de style sans unsafe-inline"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-593"
pages: ["fenetre/fenetre"]
tickets: ["ESC-593"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fenetre","added":17,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité de la 2.2.0 relève deux constats faibles sur la fenêtre (docs/security/audit-2026-11/rapport.md). SEC-12 : la fenêtre principale peut faire ouvrir par le système n'importe quel chemin du disque (`opener:allow-open-path` sur `**`), et appeler toutes les commandes du cœur. SEC-05 : la politique de contenu permet encore `'unsafe-inline'` pour les styles, et ne fixe ni `form-action`, ni `base-uri`, ni `object-src`, ni `frame-ancestors`. Aucune injection n'a été trouvée. Mais si un message d'agent passait un jour l'assainissement, une balise `<style>` pourrait redessiner l'interface (recouvrir « Refuser » d'un faux « Autoriser »), et un script ferait ouvrir par le système un exécutable déposé dans le projet. Le 2026-11-21 (a-6), l'utilisateur a choisi « A · Borner l'ouverture et séparer les styles ».

## Décision

1. Le système n'ouvre un chemin (fichier dans son programme, dossier dans le gestionnaire de fichiers, élément montré dans son dossier) que dans le dossier de données, le dossier d'un projet, le dossier de ses nouveaux worktrees, ou le dossier et le worktree de l'un de ses agents. Le cœur le juge là où il mène vraiment : lien, jonction, `..`, `\\?\` résolus ; un chemin réseau hors de ces dossiers refusé sans interroger le disque ; un périphérique de Windows jamais ouvert. Hors de ces dossiers, refusé avec sa raison, dite dans un toast. La fenêtre perd la permission `opener:allow-open-path` : toute ouverture d'un chemin passe par une seule garde du cœur (`opening::allow`). Un programme ou un script ne s'ouvre jamais, même dans un projet. Les adresses http(s) gardent leur chemin.
2. Une balise `<style>` ne s'applique qu'avec le nonce que Tauri pose à chaque chargement sur la balise `<style>` de `index.html` (`style-src 'self' 'nonce-…'`, sans `'unsafe-inline'`). Les balises que l'interface crée (xterm, CodeMirror, thème d'un plugin) le reçoivent à leur création. Les attributs `style` restent permis par `style-src-attr 'unsafe-inline'`. La politique fixe aussi `object-src`, `base-uri`, `form-action` et `frame-ancestors` à `'none'`.
3. Les commandes de la fenêtre restent d'un bloc : leur découpage par capacité est à reprendre avec l'app mobile.

## Conséquences

Le geste le plus direct d'un script injecté (faire ouvrir un exécutable ou un chemin quelconque) disparaît, et un vecteur d'injection (redessiner l'interface par une balise de style) aussi, à peu de frais : aucun changement de rendu, aucun réglage nouveau. Un fichier qu'un agent envoie hors de ces dossiers (un dossier temporaire du poste) ne s'ouvre plus et ne se montre plus depuis la conversation. Une bibliothèque qui créerait une balise `<style>` hors de `document.createElement` / `createElementNS` ne s'appliquerait pas : l'e2e csp.spec.ts vérifie qu'aucune balise de l'interface n'est refusée. Le serveur de développement n'applique pas la politique : seule l'app construite le fait, et les e2e la lisent. Le découpage des commandes reste la défense à poser pour SEC-12.
