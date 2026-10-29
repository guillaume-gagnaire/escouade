# Les clés publiques du store

Les clés publiques minisign (`*.pub`) qui signent les versions vérifiées et la liste de révocation (`docs/STORE.md`, §4.5 et §4.6) : la clé de tous les jours et la clé de secours, les mêmes que celles que l'app embarque. Elles sont publiques et vivent ici ; le store les lit au démarrage (`/app/keys` dans l'image, `STORE_KEYS_DIR` pour un autre dossier), et la copie de la liste sur escouade.dev aussi (`store/scripts/copy-revocations.ts`).

**Les clés secrètes ne sont jamais ici**, ni dans Coolify, ni dans la CI : sur le poste de l'utilisateur (`~/.escouade-signing/`), chiffrées par mot de passe, et en double hors ligne. Elles se créent avec STORE-8 ; tant que ce dossier n'a pas de `.pub`, le store ne prend aucune liste de révocation et la copie ne copie rien.
