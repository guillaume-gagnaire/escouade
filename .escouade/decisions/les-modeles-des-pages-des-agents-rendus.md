---
number: null
id: "les-modeles-des-pages-des-agents-rendus"
title: "Les modèles des pages des agents : rendus par le cœur en texte, Mermaid 12.3.0 embarqué et vérifié"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-674"
pages: []
tickets: []
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: ""
accepted_by: ""
---

## Contexte

AP-5 (ESC-674) construit les modèles review, diagram et compare des pages des agents (docs/AGENT-PAGES.md § 7), servis dans la cloison d'AP-1. Il fallait choisir où rendre les données de l'agent, comment embarquer Mermaid sans réseau, et comment l'agent vise une partie d'une page.

## Décision

Le cœur (Rust, agentpage_templates.rs) valide les données contre un schéma fermé par modèle (tous les problèmes dits d'un coup, par chemin) puis écrit lui-même le HTML de la page, chaque valeur échappée, les champs md par un Markdown à soi qui n'écrit que quelques balises ; le script du modèle ne fait jamais de HTML d'un texte de l'agent. Les fichiers communs des modèles sont servis par la cloison sous _t/ (page.css, page.js, mermaid.js). Mermaid 12.3.0 est versé dans le dépôt, gzippé (1,6 Mo) et vérifié par son SHA-256 par un test (scripts/mermaid-vendor.sh pour le changer), lancé en securityLevel strict, sans étiquettes HTML. Chaque partie a une ancre stable nommée par une clé du modèle (criteria.2 → #crit-3, nodes.<id>, options.<id>…), pour drive_browser. Et dans la cloison, une requête arrivée avant le filtre de WebView2 attend sa pose (5 s au plus) au lieu d'un 404.

## Conséquences

Aucune donnée d'agent ne devient du balisage, testé en Rust et par l'e2e avec un nom piégé. L'app pèse environ 1,6 Mo de plus. Le build ne dépend ni de node_modules ni du réseau pour Mermaid ; le mettre à jour est un geste relu. La langue est fixée au rendu (AP-2 rend à nouveau si elle change) ; le thème change par ?theme= sans nouveau rendu. AP-2 écrit index.html et copie les images ; AP-4 ouvre _/choose ; AB-7 résout target par anchor().
