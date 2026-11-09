# Installer

Trois étapes : OpenCode sur ta machine, le plugin dans Escouade, puis le compte.

## 1. Installer OpenCode

```sh
npm install -g opencode-ai
```

C’est OpenCode 1, le seul que pilote Escouade : OpenCode 2 (`@opencode/cli`) est reconnu et dit « Non prise en charge ». Réglages › Comptes et harness montre l’exécutable trouvé, sa version et son état. Le plugin a été essayé avec OpenCode 1.19.3. `npm install -g opencode-ai@latest` le met à jour.

Puis connecte un fournisseur : `opencode auth login`, ou « Se connecter… » dans Réglages › Comptes et harness, qui le lance. Les modèles locaux d’Ollama marchent aussi, déclarés dans la config d’OpenCode.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « OpenCode ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé » et « Parle au serveur local du harness, lancé par Escouade ». Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent d’OpenCode ne démarre ; leurs conversations restent.

## 3. Brancher le compte

OpenCode a un seul compte dans Escouade : ta config habituelle d’OpenCode et les fournisseurs que tu y as connectés.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « OpenCode ».
2. **Le compte de la machine** est proposé dès qu’OpenCode est installé (« OpenCode est installé sur cette machine »). **Ajouter ce compte** l’ajoute.
3. Dans sa fiche, règle le modèle, l’effort et le mode des nouveaux agents, et si tu veux un **Budget par jour ($)**.

## Ce qu’Escouade garde du compte

- **La liste des comptes** (nom, ordre, réglages, budget) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Les fournisseurs et leurs clés** restent ceux d’OpenCode, dans sa config et son dossier. Escouade et le plugin ne les lisent pas.

OpenCode n’a pas de quota : le coût d’un tour est celui que donnent tes fournisseurs, 0 avec un modèle local. La barre d’état montre le coût du jour ; une fois le budget du jour atteint, le pilote auto se met en pause jusqu’à minuit.
