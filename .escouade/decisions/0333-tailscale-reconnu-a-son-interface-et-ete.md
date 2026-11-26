---
number: 333
id: "tailscale-reconnu-a-son-interface-et-ete"
title: "Tailscale reconnu à son interface et éteint par défaut, STUN dit et coupable, le proxy du téléphone à délai"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-641"
pages: ["a-distance/tunnel-du-telephone","a-distance/app-mobile"]
tickets: ["ESC-641","ESC-624"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/tunnel-du-telephone","added":8,"removed":5},{"page":"a-distance/app-mobile","added":4,"removed":0}]
accepted_at: "2026-11-26T09:38:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624) a relevé trois constats sur l'accès du téléphone. SR1-05 : sans la commande tailscale, toute adresse de 100.64.0.0/10 passait pour Tailscale, et c'est aussi le CGNAT d'un opérateur (clé 4G, partage de connexion) : l'API pouvait écouter sur une adresse d'opérateur, et Tailscale était servi par défaut. SR1-08 : le proxy local du téléphone n'avait aucun délai avant la ligne de requête ; 64 connexions muettes d'une autre app bloquaient la WebView. SR1-11 : l'accès allumé, le poste disait son adresse publique à Cloudflare et Google (STUN) toutes les 5 minutes sans le dire.

## Décision

1) La commande tailscale seule fait foi quand elle est là ; sans elle, l'adresse de la route vers 100.100.100.100 ne compte que si elle est celle de l'interface de Tailscale (tailscale0, l'adaptateur « Tailscale » de Windows, un utun dont 100.100.100.100 répond au DNS comme MagicDNS). « Servir aussi sur l'adresse Tailscale » est éteint par défaut, et un phone.json qui n'en dit rien le prend éteint. 2) Le proxy du téléphone ferme une connexion qui n'a pas passé la porte en 5 s ; au plafond de 64, il ferme la plus ancienne connexion encore à la porte plutôt que la nouvelle. 3) Réglages › Téléphone dit STUN (qui, quand, quoi) et a l'interrupteur « Demander l'adresse publique à STUN », allumé par défaut ; éteint, rien ne part vers les serveurs STUN, la box (UPnP, NAT-PMP) et l'adresse donnée à la main servent encore, et l'adresse que la box dit avoir sur Internet sert d'adresse publique quand elle en est une. La page de confidentialité du site le dit.

## Conséquences

Un utilisateur de Tailscale doit l'allumer une fois dans Réglages › Téléphone. Un Tailscale sans sa commande et sur une interface renommée n'est plus reconnu (il faut sa commande). Une connexion du WebView lente à envoyer sa première ligne (plus de 5 s) est fermée et rouverte. STUN éteint, l'adresse publique derrière une box qui ne la dit pas reste inconnue. Pas encore essayé sur un vrai Mac ni derrière une vraie clé 4G (gestes de l'utilisateur).
