---
number: 348
id: "les-portees-des-appareils-et-le-jeton-d"
title: "Les portées des appareils, et le jeton d'un téléphone jamais envoyé"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-570"
pages: ["a-distance/api-distante","a-distance/app-mobile","a-distance/tunnel-du-telephone","a-distance/les-espaces"]
tickets: ["ESC-570","ESC-624"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":32,"removed":22},{"page":"a-distance/app-mobile","added":13,"removed":0},{"page":"a-distance/tunnel-du-telephone","added":6,"removed":0},{"page":"a-distance/les-espaces","added":6,"removed":0}]
accepted_at: "2026-11-27T10:56:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624) relève deux points à régler avant toute distribution de l'app du téléphone, même en bêta : SR1-01 (haute), tout appareil appairé a toute la fenêtre (terminal, fichiers, secrets, appairage, révocation, exposition réseau), donc un téléphone perdu ou une page piégée dans sa WebView vaut un poste ; SR1-15, la clé WireGuard et le jeton passaient par le JavaScript de la WebView (startTunnel({ privateKey })). MO-4 (ESC-570) fait l'appairage par QR code. Deux points de la conception ne tenaient pas tels quels : un code à 8 caractères seul ne porte ni la clé du poste ni son adresse, et @capacitor/barcode-scanner rend le texte du QR au JavaScript.

## Décision

1. Chaque appareil a une portée, choisie à l'appairage et gardée dans remote-devices.json : « poste » (computer : tout ce qui est servi ; un appareil d'avant les portées en est un) ou « téléphone » (phone : la boîte « À toi », répondre aux autorisations et aux questions, lire et écrire dans une conversation, les fichiers d'une demande et d'une conversation, ce qui attend d'un coup d'œil, le Kanban en gestes simples). 2. Chaque commande a sa portée dans la liste unique (remoteapi/list.rs : chaque groupe la dit, la macro refuse un groupe sans elle) ; dispatch la vérifie avant la classe et refuse avec l'erreur nommée « scope ». Les commandes « appareils » (remote_pair_code, phone_pair_qr, remote_devices, remote_device_revoke, phone_*) sont d'un poste : un client de bureau sur un serveur retrouve « Créer un code », et gagne « Appairer un téléphone ». 3. Un appairage par le tunnel du téléphone, ou avec la clé d'un téléphone, donne toujours la portée « téléphone ». 4. Le jeton d'un téléphone n'est jamais envoyé : le natif fait une paire X25519, l'app en donne la moitié publique avec le code, le cœur répond la sienne, et chacun dérive par HKDF-SHA256 le jeton et la clé de l'appairage ; le proxy local du téléphone écrit le jeton dans l'en-tête X-Escouade-Device, la page donne un jeton vide. Le JavaScript ne passe qu'un identifiant d'espace ; clés et jeton restent au Keystore et au Trousseau. Seul le texte du QR traverse la page, une fois, remis tel quel au natif. 5. Le code à 8 caractères, tapé, va avec l'adresse d'un espace joint sans le tunnel (Tailscale) : avec le seul WireGuard, il faut le QR. 6. La révocation retire le jeton, la clé de l'appairage, l'inscription au push et le pair WireGuard, aussitôt. 7. Pas d'appairage sans verrou d'écran ; le verrou avant un geste qui fait agir passe par une seule fonction de l'interface (confirmByLock).

## Conséquences

Un téléphone perdu ou une page piégée dans sa WebView n'ouvre ni terminal, ni fichier, ni secret, ni appareil ; un jeton de téléphone qui fuit ne fait naître aucun appareil, et aucun secret durable ne se lit dans la WebView. Ajouter une commande, c'est aussi choisir sa portée, et une de plus pour le téléphone se décide (un essai liste exactement les siennes ; docs/REMOTE.md les marque « · téléphone »). Un cœur d'avant ESC-570 qui répond le jeton en clair est refusé par l'app (« Mets Escouade à jour »). L'app native embarque maintenant le plugin : la CI de l'app construit d'abord la bibliothèque Rust et les liaisons. MO-5, MO-7, MO-10 et MO-11 étendront la liste du téléphone s'il le faut, par une nouvelle décision. Un téléphone sans appareil photo ne s'appaire au seul tunnel WireGuard d'un poste que par Tailscale.
