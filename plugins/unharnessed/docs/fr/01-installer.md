# Installer

Cinq étapes : Unharnessed sur ta machine, un fournisseur de modèles pour lui, le plugin ACP, ce plugin, puis un compte. Avant tout, lis l’onglet **Permissions** : Unharnessed ne demande jamais rien.

## 1. Obtenir Unharnessed

Unharnessed n’a pas de téléchargement officiel hors de son extension VS Code, et aucune commande officielle ne l’installe seule : ce plugin n’en propose donc aucune. Deux façons de l’avoir :

**L’extension VS Code (macOS Apple Silicon seulement).** Au 16 novembre 2026, l’extension « Unharnessed Agent » (`Unharnessed.unharnessed-agent`, 0.0.4) n’est publiée sur le Marketplace de VS Code que pour macOS Apple Silicon. Elle porte le programme dans son dossier :

```text
~/.vscode/extensions/unharnessed.unharnessed-agent-<version>-darwin-arm64/bin/unharnessed
```

à côté de son noyau Python (`bin/unharnessed-runtime`) et de ses skills (`bin/skills`), qu’il trouve à côté de lui. Le nom du dossier change à chaque version : Escouade ne le trouve pas seul. Donne son chemin dans Réglages › Comptes et harness (le chemin du programme), ou mets un lien vers lui dans un dossier du `PATH`, et refais-le après chaque mise à jour de l’extension. Le programme de l’extension n’a pas été essayé hors de VS Code.

**La construction depuis la source (Windows, Linux, macOS).** Il faut Rust 1.98.1 (le dépôt le fixe), [uv](https://docs.astral.sh/uv/) et Git :

```sh
git clone https://github.com/unharnessed-agent/Unharnessed
cd Unharnessed
cargo build --release -p uh-cli --bin unharnessed
./target/release/unharnessed --unharnessed-bootstrap
```

Puis mets `target/release` dans le `PATH` : Escouade cherche `unharnessed` dans le `PATH`. `unharnessed --version` doit répondre `0.9.8` ou plus. Le plugin a été essayé avec Unharnessed 0.9.8, construit ainsi sous Linux (commit `fe2cf28` du 15 novembre 2026) ; la construction sous Windows et macOS n’a pas été essayée.

Dans les deux cas, il faut `uv` dans le `PATH` : Unharnessed construit son noyau Python (un venv, `~/.unharnessed/kernel-venv`) à son premier lancement.

## 2. Un fournisseur de modèles

Unharnessed parle à ses fournisseurs lui-même : Escouade ne lui donne aucune clé. Configure-le dans un terminal, comme le dit sa documentation : une connexion à un abonnement, une clé dans les variables habituelles d’un fournisseur, ou un modèle local (Ollama, LM Studio, llama.cpp) déclaré dans `~/.unharnessed/models.json`. Par exemple, pour Ollama :

```json
{
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434/v1",
      "apiKey": "local",
      "api": "openai-completions",
      "models": [{ "id": "qwen3-coder:30b", "name": "Qwen3 Coder 30B", "contextWindow": 32768 }]
    }
  }
}
```

`unharnessed model list` liste les modèles qu’il peut prendre.

## 3. Le plugin ACP

Unharnessed est servi par le plugin ACP (`dev.escouade.acp`). Dans la vue Plugins, vérifie qu’il est dans **Installés** et allumé ; sinon, installe-le depuis **Store**. Coupé, les agents Unharnessed restent, marqués « harness absent », avec leurs conversations.

## 4. Installer le plugin

1. Dans la vue Plugins, ouvre **Store** et cherche « Unharnessed ».
2. Sur sa page, clique sur **Installer**. La fenêtre montre le programme que ses agents lanceront, `unharnessed --mode acp`, avec tes droits. Confirme.
3. Autorise le mode Bypass : Réglages › Agents, « Autoriser le mode Bypass ». C’est le seul mode d’Unharnessed.
4. Choisis « Unharnessed » dans le menu « Harness » à côté de « Nouvel agent ».

## 5. Brancher un compte

Ouvre Réglages › Comptes et harness, **Ajouter un compte…**, harness « Unharnessed » :

- **Le compte de la machine** : le dossier habituel d’Unharnessed, `~/.unharnessed`, tel quel.
- **Un nouveau compte** : un dossier à lui, donné à Unharnessed par `UNHARNESSED_CODING_AGENT_DIR` : ses identifiants (`auth.json`), ses réglages (`settings.json`, `models.json`) et ses sessions y vivent. Configure son fournisseur pour ce dossier. Le noyau Python reste partagé, dans `~/.unharnessed/kernel-venv`.

Dans la fiche du compte, mets le mode des nouveaux agents sur **Bypass** (voir l’onglet Permissions).

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Les identifiants, les réglages et les sessions d’Unharnessed** restent les siens, dans son dossier. Escouade n’en lit aucun.
