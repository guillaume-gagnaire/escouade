---
title: "Réseau : proxy et certificats"
subject: "fenetre"
sources: ["docs/security/audit-2026-11/rapport.md (branche ticket/esc-418), fiche SEC-07","website/content/docs/fr/settings-network.md"]
tickets: ["ESC-425"]
decisions: ["certificats-par-un-fichier-d-autorites-p"]
compared: null
---

## Ce que c'est

Réglages › Avancé › Réseau : le proxy HTTP(S), ses exclusions, et à qui les connexions d'Escouade font confiance. Toutes les connexions de l'app (quotas, intégrations, store, mises à jour, dictée, images distantes, réseau des plugins en `fetch` et en WebSocket) suivent la même confiance (`src-tauri/src/tls.rs`).

## Comportement

### La confiance

- Les autorités de Mozilla, celles du magasin du système, et celles du fichier « Autorités de certification supplémentaires » (un `.pem`, par exemple le certificat racine de Zscaler). Le fichier s'ajoute aux autres, il ne les remplace pas.
- « Choisir… » prend le fichier ; sous le champ, l'app dit combien de certificats elle y lit (« 1 certificat lu »), ou pourquoi elle ne le lit pas (« aucun certificat PEM dans … »). Un fichier illisible au moment d'une connexion est écrit au journal et laissé de côté : le reste tient.
- Les agents reçoivent le fichier au prochain démarrage de leur process : `NODE_EXTRA_CA_CERTS` (Claude Code, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode) et `CODEX_CA_CERTIFICATE` (Codex). Les programmes des plugins les reçoivent aussi. Mistral Vibe (Python) n'a pas de variable qui ajoute sans remplacer : rien ne lui est passé.

### « Ignorer la vérification des certificats TLS »

- Gardée pour les installations qui l'ont cochée, éteinte par défaut. Elle ne vaut que pour les connexions qui passent par le proxy configuré : sans proxy, elle n'a aucun effet ; vers un hôte des exclusions, ou une route directe d'un plugin, le certificat est vérifié. Le contrôle des mises à jour qui retente sans le proxy vérifie le certificat.
- Les agents reçoivent `NODE_TLS_REJECT_UNAUTHORIZED=0` seulement quand un proxy est configuré.
- Tant qu'elle est cochée, un bandeau permanent, sous la barre de titre, le dit : « Certificats TLS non vérifiés par le proxy <hôte:port> : une interception, jetons compris, passerait inaperçue. » (sans proxy : « … est cochée, sans proxy : elle n'a pas d'effet. »). Il ne montre jamais l'utilisateur ni le mot de passe du proxy. Son bouton « Choisir le certificat racine » ouvre Réglages › Avancé › Réseau.
- Sa description propose le fichier à la place : « Avec Zscaler ou un proxy d'entreprise : choisis son certificat racine (un fichier .pem). »

## Limites

- Les exclusions sont comparées par nom d'hôte et domaine (`*` pour tout) ; une plage d'adresses (CIDR) des exclusions n'est pas reconnue par la vérification : un tel hôte est tenu pour passé par le proxy.
- Les terminaux intégrés ne reçoivent ni le fichier ni la case.
