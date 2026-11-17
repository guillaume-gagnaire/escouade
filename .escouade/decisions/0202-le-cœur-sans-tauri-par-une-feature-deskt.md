---
number: 202
id: "le-cœur-sans-tauri-par-une-feature-deskt"
title: "Le cœur sans Tauri par une feature desktop et un crate qui tient lieu de Tauri, sans déplacer de fichier"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-514"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-514"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":54,"removed":0},{"page":"a-distance/api-distante","added":14,"removed":13}]
accepted_at: "2026-11-17T20:13:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

escouade-server (mode serveur, MS-1) doit être un binaire Linux x64 et arm64 sans WebKit ni GTK. Le crate tauri lie GTK sous Linux quelles que soient ses features, et tauri-runtime lie WebKitGTK : le serveur ne peut pas lier tauri du tout. docs/REMOTE.md prévoyait de sortir le cœur dans un crate à lui, mais une dizaine de tickets touchent src-tauri/src en parallèle : un déplacement en masse rendrait leurs fusions impraticables.

## Décision

Le crate de l'app garde tous ses fichiers. Une feature `desktop` (par défaut) porte Tauri, ses plugins, tauri-build, GTK, WebKitGTK, WebView2 et les notifications du système. Sans elle, le petit crate `escouade-tauri-shim` tient la part de l'API de Tauri dont le cœur se sert (async_runtime, ipc::Channel, les corps de l'IPC, http, Url, Error), sous le nom `tauri` (`extern crate escouade_tauri_shim as tauri;`) : les modules du cœur ne changent pas. Ce qui ne sert que la fenêtre se compile avec `desktop` seulement (#[cfg(feature = "desktop")] au-dessus des éléments) ; les commandes refusées à distance par leur classe ne sont pas compilées dans le serveur, la table de l'API les refuse. escouade-server se construit par `cargo build --bin escouade-server --no-default-features`. Écartés : un crate du cœur (≈ 190 fichiers déplacés), ou chaque usage de Tauri réécrit (≈ 100 lignes dans ≈ 40 fichiers).

## Conséquences

Aucun fichier déplacé ; une vingtaine de fichiers prennent des attributs cfg, presque aucune ligne existante n'est réécrite. Une commande nouvelle qui prend l'AppHandle va dans la classe desk et prend #[cfg(feature = "desktop")] ; le cœur peut écrire tauri::async_runtime, tauri::ipc::Channel, tauri::http, tauri::Url comme avant, tout autre nom de Tauri est du bureau. La CI construit escouade-server pour Linux x64 et arm64, vérifie ses bibliothèques liées (ldd) et le démarre dans un conteneur Debian nu. La voix sur la machine (cpal, whisper.cpp) reste une dépendance, laissée hors du binaire par l'édition des liens. Avec lui : escouade-server (run, pair, devices, --data-dir, --port, --listen), l'appairage d'un appareil par un code à usage unique de 10 minutes et 5 essais, échangé contre son jeton à la place de la poignée de main, les secrets du serveur dans secrets.json (0600), dits au client par secretsInFile. Détail dans docs/CRATES.md § 14 et docs/REMOTE.md (« Le mode serveur, MS-1 »).
