---
number: 201
id: "les-extensions-de-l-editeur-open-vsx-par"
title: "Les extensions de l'éditeur : Open VSX par le cœur, VSIX signé, déclaratives seulement, liste permise en quatre couches"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-481"
pages: ["editeur/extensions","editeur/editeur","projets/escouade-yml","fenetre/reglages-imposes"]
tickets: ["ESC-481"]
documents: ["docs/EDITOR-STUDY.md"]
supersedes: []
superseded_by: null
changes: [{"page":"editeur/extensions","added":65,"removed":0},{"page":"editeur/editeur","added":5,"removed":3},{"page":"projets/escouade-yml","added":17,"removed":3},{"page":"fenetre/reglages-imposes","added":3,"removed":2}]
accepted_at: "2026-11-17T19:04:19+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

A5 de l'étude de l'éditeur (docs/EDITOR-STUDY.md, §3, §6, §7) : la vue VS Code doit pouvoir chercher, installer, mettre à jour, couper et retirer des extensions. L'utilisateur a choisi Open VSX seulement (le marketplace de Microsoft est exclu) et pas d'hôte Node d'abord. La maquette chargeait les fichiers depuis le point `unpkg` d'Open VSX (JavaScript en text/plain, gros fichiers refusés), sans rien vérifier, et avait dû ajouter `unsafe-eval` et les domaines d'Open VSX à la CSP. Il fallait trancher : qui parle à Open VSX, ce que prouve la signature et que faire d'une version sans, ce qui tourne sans hôte d'extensions, quel thème gagne, comment la liste permise se règle et s'impose, et ce qu'un projet privé laisse partir.

## Décision

1. **Tout par le cœur** (crate `escouade-vsx`, module `vsx.rs`) : recherche, fiches, icônes, README, téléchargement, vérification, magasin, fichiers. La page ne demande jamais rien à Open VSX ni au disque. Le client est celui de l'app (proxy, autorités) ; toute adresse donnée par Open VSX doit être sur son origine.
2. **Somme et signature** : le SHA-256 du VSIX contre le `.sha256` d'Open VSX, puis la signature Ed25519 du `.sigzip` (`.signature.sig`, sur les octets du VSIX entier) avec la clé publique qu'Open VSX publie. Elle prouve que le fichier est celui qu'Open VSX a publié, pas que l'extension est sûre ni que son auteur est qui il dit (c'est l'espace de noms vérifié, montré sur la fiche), ni rien contre Open VSX lui-même. **Une version sans signature n'est pas installée.**
3. **Déballage borné** : la partie `extension/` seule, l'archive entière refusée pour `..`, chemin absolu ou de lecteur, lien, nom interdit sous Windows, doublon de casse, plus de 20 000 fichiers ou 512 Mo décompressés (tailles annoncées non crues). Le `package.json` doit être l'extension et la version de la fiche.
4. **Ce qui tourne** : les extensions déclaratives seules (thèmes, grammaires, extraits, langages, thèmes d'icônes), posées dans la vue sans aucun code. **Une extension à code s'installe et reste « en attente de l'hôte d'extensions »** (A6) ; une extension Node dit « Demande l'hôte Node, pas encore dans Escouade ».
5. **La CSP de la fenêtre n'est pas élargie** : les fichiers arrivent à la page par une commande du cœur, en `blob:` et `data:` (déjà permis). Le protocole du cœur `escouade-ext:` (lecture seule, types MIME, `nosniff`, une extension installée seulement, rien d'autre du disque) existe pour l'hôte d'extensions d'une origine à part (A6), avec sa propre CSP.
6. **Le thème** : celui d'Escouade reste celui de la vue par défaut ; le thème d'une extension se choisit exprès (« Utiliser … »), passe alors devant, et se quitte d'un geste (« Revenir au thème d'Escouade ») ; couper ou retirer l'extension ramène celui d'Escouade.
7. **La liste permise** `extensions.allowed`, la règle de VS Code (par éditeur ou par extension, `true`, `false`, `stable`, versions), tenue par le cœur en quatre couches qui doivent toutes permettre l'extension : le poste géré et l'équipe (stratégie `extensions.allowed` de la liste unique, sorte « restreindre », forme JSON, chaque rang gardé et croisé), les réglages du poste (Général › Avancé, grisé quand c'est imposé) et le `.escouade.yml` de chaque projet ouvert dans la vue (`editor.extensions_allowed`, clé du fichier seul). Une extension refusée n'est même pas demandée à Open VSX ; une extension installée qu'une couche exclut n'est plus chargée.
8. **La question de confiance par éditeur** est gardée, avant tout téléchargement, avec l'espace de noms vérifié ou non.
9. **Les mises à jour** sont proposées sur demande (« Rechercher des mises à jour »), jamais installées seules, et repassent par toute la vérification.
10. **Un projet privé** ouvert dans la vue : rien n'est demandé à Open VSX depuis elle (ni recherche, ni fiche, ni icône, ni installation, ni mise à jour) ; les extensions installées marchent. Ouvrir la vue ne demande jamais rien à Open VSX.

## Conséquences

Les thèmes, grammaires et extraits d'Open VSX marchent dans la vue sans `unsafe-eval` ni domaine distant dans la CSP ; Vim, ESLint ou Prettier s'installent mais attendent A6 ou N2. Une ancienne version sans signature d'Open VSX ne s'installe pas (la plupart sont signées depuis 2023). Les extensions sont celles du poste pour toute la vue, sans dépendances ni packs. La stratégie `extensions.allowed` rejoint l'ADMX, les profils et la page « Stratégies du poste » ; `editor.extensions_allowed` rejoint le schéma du `.escouade.yml`. Poids : +24 Ko brotli de frontend (4 Ko dans le morceau principal), aucune crate tierce nouvelle (Ed25519, zip et SHA-256 étaient déjà dans l'app). A6 servira l'hôte d'extensions web depuis `escouade-ext:` sur une origine à part ; il devra dire chaque directive de CSP qu'il ajoute.
