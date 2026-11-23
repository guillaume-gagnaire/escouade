# Les clés publiques du store

Les clés publiques minisign (`*.pub`) qui signent les versions vérifiées et la liste de révocation (`docs/STORE.md`, §4.5 et §4.6). Chacune a ses rôles, donnés par le nom de son fichier (ESC-592, audit SEC-03) :

| Fichier | Clé | Ce qu'elle signe |
| --- | --- | --- |
| `store.pub` | la clé de la première main (id E0516468FFDCBDD5), sans mot de passe | les plugins `dev.escouade.*` seulement |
| `third-party.pub` | la clé des tiers et de la révocation, à mot de passe (à créer : `npm run plugin-review -- create-key third-party`) | tout autre plugin, et la liste de révocation |
| `backup.pub` | la clé de secours (id 177096D57E2EC0C8), hors ligne | tout |

Un fichier d'un autre nom n'est pris par personne. Ce sont les mêmes clés, avec les mêmes rôles, que celles que l'app embarque (`STORE_KEYS` de `crates/escouade-plugins/src/store_keys.rs`, qu'un test compare à ce dossier ; `ROLES_BY_FILE` de `store/server/utils/minisign.ts` pour le store). Elles sont publiques et vivent ici ; le store les lit au démarrage (`/app/keys` dans l'image, `STORE_KEYS_DIR` pour un autre dossier), le script de la revue avant de déposer une signature ou une liste (`npm run plugin-review -- sign`, `revoke`), la copie de la liste sur escouade.dev aussi (`store/scripts/copy-revocations.ts`), et l'amorce de l'app (`store/scripts/seed.ts`).

**Les clés secrètes ne sont jamais ici**, ni dans Coolify, ni dans la CI : sur le poste de l'utilisateur (`~/.escouade-signing/store.key`, `~/.escouade-signing/third-party.key`), en double hors ligne ; la clé de secours seulement hors ligne. L'utilisateur les crée lui-même (`docs/STORE.md` §4.5). **Tant que ce dossier n'a pas de `.pub`**, le store ne prend ni signature ni liste de révocation (`/review` le dit), la copie ne copie rien et l'app refuse tout badge.
