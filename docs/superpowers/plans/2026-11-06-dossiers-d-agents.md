# Dossiers d'agents — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Context

Sur Nexus, une grosse feature tient un orchestrateur et une dizaine d'agents qu'il a créés par le MCP, mêlés dans une liste plate (du plus ancien au plus récent) aux agents d'autres features. L'utilisateur veut des **dossiers d'agents** par projet : nommés, repliables, avec un agent principal (★), créés à la main ou automatiquement quand un agent en crée d'autres par le MCP, avec glisser-déposer et parité MCP. Le design approuvé fait foi : `docs/superpowers/specs/2026-11-06-dossiers-d-agents-design.md` (commit `000274e0`).

**Goal:** Ranger les agents d'un projet dans des dossiers d'agents, à la main et automatiquement, dans la fenêtre et par le serveur MCP.

**Architecture:** Les dossiers et l'ordre de la liste vivent sur `Project` (cœur Rust, sauvegardés et gardés à la fermeture du projet gratuitement via `ClosedProject.project`). Un module pur `src-tauri/src/folders.rs` tient les règles (ordre rangé, ordre affiché, déplacements) et les méthodes `Core` qui les appliquent, émettent `UiEvent::Project` et sauvegardent ; la fenêtre et le MCP passent par elles. La fenêtre recalcule l'ordre affiché en TS (`src/lib/sidebar.ts`), avec les mêmes cas chiffrés que Rust dans un fixture JSON partagé.

**Tech Stack:** Rust (Tauri 2, rmcp `#[tool]`, serde), Svelte 5, Vitest, Playwright (e2e Windows en CI), cargo-nextest.

**Spec:** `docs/superpowers/specs/2026-11-06-dossiers-d-agents-design.md` — à lire avec ce plan.

**Forme du plan :** exigences, interfaces et cas de test, sans le code complet. L'implémenteur lit le code cité, écrit d'abord les tests (rouge vérifié), puis le code. Première étape à l'exécution : copier ce plan dans `docs/superpowers/plans/2026-11-06-dossiers-d-agents.md` et le commiter.

## Global Constraints

- Branche `escouade/gestion-dossiers-agents` (worktree actuel). Commits atomiques, messages en français au format du dépôt (`feat(portée): …`), terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Pas de push sans que l'utilisateur le dise (circuit : fork `Metalzoid/escouade` d'abord).
- Deux langues partout : `t()` + `src/lib/i18n/{fr,en}/<zone>.ts` côté fenêtre (français source, anglais vérifié par `npm run check`), `tr!("fr", "en")` côté Rust.
- Aucune dépendance nouvelle. Glisser-déposer HTML5 natif, comme `ProjectRail.svelte` (seuil de mouvement de l'OS, ≈ 4 px : décision 5 du design).
- Un état d'avant cette version se lit sans dossier (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), et `blank_project()` (`src-tauri/src/core.rs:1567`) reçoit les champs neufs.
- Verrous : jamais `agents` et `projects` tenus ensemble ; lire les agents (relâcher), puis écrire `projects`.
- Une opération qui vise un agent ou un dossier disparu est refusée avec sa raison (`anyhow::bail!(tr!(…))`), jamais à moitié faite.
- Le cadre du dossier n'a jamais la classe `agent` (sélecteurs `.agent[data-agent]` du Sidebar et des e2e).
- Spec du projet : mise à jour par `write_spec_page` (`fenetre/barre-laterale`, `harness/serveur-mcp`), jamais `docs/SPEC.md`.
- Tests Rust sur Mac : `cargo nextest run --profile ci --test-threads 3` dans `src-tauri`, environnement sans `CLAUDE_*` (script bash : `for v in $(env | grep -o '^CLAUDE[A-Za-z_]*'); do unset "$v"; done`), `CLAUDE_CODE_ENABLE_TODO_TOOLS=1` ; `cargo clippy --all-targets -- -D warnings` ; `cargo fmt --check`. Front avec Node 24 (`PATH=$HOME/.nvm/versions/node/v24.22.2/bin:$PATH`) : `npm test`, `npm run check`, `npm run format:check`. Jamais `cargo test --lib` pour juger.

## Review Focus

1. **Agents d'avant la version** (aucun dans `agent_order`) puis un nouvel agent : le nouveau doit s'afficher en bas, pas au-dessus des anciens → test `settle_lists_old_agents_before_a_new_one` (tâche 1) et cas `old-agents-then-new` du fixture.
2. **Agent principal archivé puis restauré** : le dossier s'affiche sans principal pendant l'archivage, et le retrouve à la restauration si personne d'autre ne l'a pris → test `an_archived_principal_comes_back_as_principal` (tâche 2).
3. **Deux `create_agent` simultanés du même orchestrateur sans dossier** : un seul dossier automatique, pas deux → test `two_creations_at_once_make_one_folder` (tâche 5), garanti par `core.mcp.acting`.
4. **Glisser un agent qui attend (remonté)** : posé où on l'a lâché dans l'ordre rangé, toujours affiché en tête jusqu'à sa réponse → cas Vitest `a waiting agent dropped keeps its lift` (tâche 7).
5. **Dossier supprimé pendant qu'un ticket le retient** : l'agent du ticket naît hors dossier, sans erreur → test `a_ticket_whose_folder_went_starts_its_agent_loose` (tâche 5).

---

## Fichiers

- Créer `src-tauri/src/folders.rs` (règles pures + `impl Core` des dossiers) et `src-tauri/src/folders_tests.rs` (déclarés dans `lib.rs` comme les autres `*_tests`).
- Créer `tests/fixtures/sidebar-order.json` (cas partagés Rust/TS).
- Créer `src/lib/agent-folders.ts` (+ `.test.ts`) : dépôt, pas clavier, menus des dossiers.
- Créer `tests/e2e/agent-folders.spec.ts`.
- Modifier : `src-tauri/src/model.rs` (`AgentFolder`, `Project`, `Ticket.folder`), `core.rs` (`AgentOptions.folder`, hooks), `commands.rs`, `lib.rs`, `windowthread.rs`, `tickets.rs`, `mcp/{tools,act,read,resolve,tests,private_tests,tools_tests,act_tests}.rs`, `agent.rs:1821`, `src/lib/{types,ipc,sidebar,state.svelte,tools}.ts`, `src/lib/i18n/{fr,en}/{nav,mcp}.ts`, `src/components/Sidebar.svelte`, `website/content/docs/{fr,en}/mcp.md`, `CHANGELOG.md`, `CHANGELOG.en.md`.

---

### Task 1 : modèle et règles pures

**Files:** Modify `src-tauri/src/model.rs` (après `Project` ~:795 ; `blank_project` dans `core.rs:1567`) · Create `src-tauri/src/folders.rs`, `src-tauri/src/folders_tests.rs`, `tests/fixtures/sidebar-order.json`

**Interfaces — Produces:**
```rust
// model.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentFolder {
    pub id: String,                 // new_id() (model.rs:2744)
    pub name: String,
    pub named: bool,                // renommé (main ou update_folder) : ne suit plus son principal
    pub principal: Option<String>,  // toujours membre de `agents`
    pub agents: Vec<String>,        // membres dans l'ordre rangé, archivés compris
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub folded: bool,
}
// Project : agent_folders: Vec<AgentFolder>, agent_order: Vec<String> (ids d'agents hors dossier et de dossiers)

// folders.rs (pur)
pub struct Member<'a> { pub id: &'a str, pub created_at: i64, pub waiting: bool, pub archived: bool }
pub enum Shown { Agent(String), Folder { id: String, agents: Vec<String> } } // agents : principal d'abord
pub fn settle(p: &mut Project, members: &[Member]) -> bool;
pub fn folder_of<'a>(p: &'a Project, agent: &str) -> Option<&'a AgentFolder>;
pub fn add_agent(p: &mut Project, agent: &str, folder: Option<&str>); // en bas du dossier, ou de la liste
pub fn place_after(p: &mut Project, agent: &str, after: &str);       // même niveau, juste après
pub fn remove_agent(p: &mut Project, agent: &str);                    // dossiers, principal, ordre
pub fn move_item(p: &mut Project, item: &str, folder: Option<&str>, index: Option<usize>) -> anyhow::Result<()>;
pub fn insert_folder(p: &mut Project, f: AgentFolder, at: Option<&str>); // à la place de l'agent `at`, sinon en bas ; `f.agents` quittent leur place
pub fn remove_folder(p: &mut Project, folder: &str) -> anyhow::Result<()>; // membres à la place du dossier, dans leur ordre
pub fn shown(p: &Project, members: &[Member]) -> Vec<Shown>;
pub fn shown_agents(p: &Project, members: &[Member]) -> Vec<String>;
```
Règles : `settle` retire les ids inconnus et les doublons, puis ajoute en bas d'`agent_order`, par `created_at`, chaque membre rangé nulle part. `shown` : archivés exclus ; non listés traités comme rangés en bas par `created_at` ; hors dossier, d'abord les agents `waiting` (dans leur ordre rangé), puis les éléments dans l'ordre rangé ; dans un dossier, principal (s'il n'est pas archivé), puis `waiting`, puis les autres. `move_item` refuse un dossier vers un dossier, un item ou un dossier cible inconnu ; un principal sorti de son dossier perd l'étoile ; `index` est le rang dans le niveau cible une fois l'item retiré, `None` = en bas.

- [ ] **Step 1 : tests rouges** dans `folders_tests.rs` : `settle_lists_old_agents_before_a_new_one` (3 agents non listés + `add_agent` d'un 4ᵉ → `agent_order == [a1,a2,a3,a4]`), `move_item_refuses_a_folder_into_a_folder`, `a_principal_moved_out_loses_its_star`, `removing_a_folder_puts_its_agents_in_its_place`, `remove_agent_clears_order_folder_and_principal`, `old_state_reads_without_folders` (`serde_json::from_value::<PersistedState>` d'un projet sans les champs → vides), et `shown_follows_the_shared_cases` qui lit `include_str!("../../tests/fixtures/sidebar-order.json")`.
- [ ] **Step 2 : le fixture** `tests/fixtures/sidebar-order.json` : `[{ "name", "project": { "agentFolders", "agentOrder" }, "agents": [{ "id", "createdAt", "status", "archived" }], "shown": [ids à plat], "items": [["agent", id] | ["folder", id, [ids]]] }]`, cas au moins : `flat-oldest-first`, `old-agents-then-new`, `waiting-loose-lifts-to-top`, `waiting-in-folder-under-principal`, `principal-archived`, `unknown-ids-ignored`, `empty-folder`.
- [ ] **Step 3 : vérifier l'échec** — `cargo nextest run --profile ci folders_tests` : FAIL (module absent).
- [ ] **Step 4 : implémenter** les types et `folders.rs` (fonctions ci-dessus).
- [ ] **Step 5 : vérifier** — même commande : PASS ; `cargo clippy --all-targets -- -D warnings` propre.
- [ ] **Step 6 : commit** `feat(dossiers): les dossiers d'agents sur le projet et leurs règles d'ordre`.

### Task 2 : le cœur applique, la fenêtre demande

**Files:** Modify `src-tauri/src/folders.rs`, `core.rs` (`AgentOptions` :741, `create_agent_with` :6203, `duplicate_agent` :6262, `rename_agent` :7337, `apply_name_to` :7287, `delete_agent` :7584), `tickets.rs:485` (`emit_project` → `pub(crate)`), `commands.rs`, `lib.rs` (handler ~:684), `windowthread.rs` (`FAMILIES`), `src/lib/types.ts` (`Project` :711), `src/lib/ipc.ts` · Test `src-tauri/src/folders_tests.rs`

**Interfaces — Consumes:** tâche 1. **Produces:**
```rust
impl<R: Runtime> Core<R> {
    fn edit_layout<T>(&self, project_id: &str, f: impl FnOnce(&mut Project) -> Result<T>) -> Result<T>; // members lus, settle, f, request_save, emit_project
    pub fn folder_create(&self, project_id: &str, name: Option<&str>, agents: &[String], principal: Option<&str>, at: Option<&str>) -> Result<AgentFolder>; // name None : « Dossier », « Dossier 2 »… (tr!), named = true
    pub fn folder_rename(&self, project_id: &str, folder: &str, name: &str) -> Result<()>;   // named = true ; nom vide refusé
    pub fn folder_delete(&self, project_id: &str, folder: &str) -> Result<()>;
    pub fn folder_principal(&self, project_id: &str, folder: &str, agent: Option<&str>) -> Result<()>; // agent hors du dossier refusé
    pub fn folder_fold(&self, project_id: &str, folder: &str, folded: bool) -> Result<()>;
    pub fn item_move(&self, project_id: &str, item: &str, folder: Option<&str>, index: Option<usize>) -> Result<()>;
    pub fn shown_agents_of(&self, project_id: &str) -> Vec<String>;
}
// AgentOptions { …, folder: Option<String> } — create_agent_with appelle add_agent (dossier disparu : hors dossier)
```
Hooks : `duplicate_agent` → `place_after(copie, original)` ; `delete_agent` → `remove_agent` ; `rename_agent` et `apply_name_to` → chaque dossier de son projet dont il est principal et `!named` prend son nom (puis `emit_project`). Archiver ne touche pas aux dossiers (le principal archivé reste noté, `shown` le cache) ; restaurer (`archive_agent(id, false)`, :7453) le remet en bas de son niveau (son dossier s'il en est encore membre, sinon la liste) par `move_item(…, None)`.
Choix du plan : l'état replié est sur `AgentFolder.folded` (pas dans `UiState`, que la fenêtre réécrit en entier) ; il suit le projet fermé puis rouvert.
Commandes Tauri, famille `("agent_folder_", "agent_folder")` dans `FAMILIES` : `agent_folder_create(project_id, agent_id: Option<String>) -> AgentFolder` (avec un agent : il entre, à sa place, sans devenir principal), `agent_folder_rename`, `agent_folder_delete`, `agent_folder_principal(project_id, folder_id, agent_id: Option<String>)`, `agent_folder_fold(project_id, folder_id, folded)`, `agent_folder_move(project_id, item_id, folder_id: Option<String>, index: Option<usize>)`.
TS : `interface AgentFolder { id; name; named; principal: string | null; agents: string[]; folded?: boolean }`, `Project.agentFolders?`, `Project.agentOrder?` ; `api.agentFolderCreate/Rename/Delete/Principal/Fold/Move` sur le modèle de `reorderProjects` (`ipc.ts:291`).

- [ ] **Step 1 : tests rouges** (harness `core_tests::harness`, `h.project(false)`, `h.core.create_agent`) : `a_new_agent_goes_to_the_bottom_or_into_its_folder`, `the_copy_follows_its_original_in_its_folder`, `a_deleted_agent_leaves_its_folder`, `an_archived_principal_comes_back_as_principal`, `a_restored_agent_comes_back_at_the_bottom_of_its_folder`, `an_auto_named_folder_follows_its_principal_until_renamed`, `folders_survive_a_restart` (`save_now` + `Core::load`), `folders_come_back_with_a_closed_project` (`closed_tests::close` / `add_again`), `each_change_tells_the_window` (un événement `project` dans `h.events`), `a_move_to_a_folder_that_went_is_refused`.
- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run --profile ci folders_tests` : FAIL.
- [ ] **Step 3 : implémenter** méthodes, hooks, commandes, liaisons TS.
- [ ] **Step 4 : vérifier** — `cargo nextest run --profile ci folders_tests closed_tests core_tests` : PASS ; `npm run check` propre.
- [ ] **Step 5 : commit** `feat(dossiers): le cœur range les agents, la fenêtre le lui demande`.

### Task 3 : MCP en lecture — `list_agents`, `list_folders`

**Files:** Modify `mcp/read.rs` (`AgentRow` :260, `agents` :290), `mcp/resolve.rs` (`folder`, sur `named`/`listed` :332-355), `mcp/tools.rs` (`ListFoldersArgs`, outil, description de `list_agents` :1096), `mcp/tests.rs` (`EXPOSED`, `READING`), `mcp/tools_tests.rs` · Test `mcp/tools_tests.rs`

**Interfaces — Produces:** `#[derive(Serialize)] struct FolderRef { id: String, name: String }` (dans `read.rs`, repris par la tâche 5) ; `resolve::folder(core, caller, asked: &str, project: &Project) -> Result<AgentFolder, ToolError>` (id exact, puis nom sans casse ; mêmes mots qu'un nom inconnu pour ce que l'appelant ne voit pas). `AgentRow` gagne `folder: Option<FolderRef { id, name }>` et `principal: bool`. `list_agents` : projets dans l'ordre de l'app (comme `read::projects` :190), agents dans `shown_agents_of`. `list_folders { project: Option<String> }` → `[{ id, name, principal: {id,name}|null, agents: [{id,name,status}] }]` dans l'ordre affiché, archivés exclus. Description de `list_agents` : « oldest first » devient l'ordre de la barre latérale.

- [ ] **Step 1 : tests rouges** : adapter `tools_tests.rs:338-426` (JSON de `all[0]` avec `"folder": null, "principal": false`), ajouter `list_agents_follows_the_sidebar_order_with_folders` et `list_folders_gives_each_folder_with_its_principal_and_agents` (et `last_entry == (…, "list_folders", "project: DEMO", "ok")`) ; `list_folders` dans `READING` et dans la boucle « chaque argument décrit » (`tools_tests.rs:225-247`).
- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run --profile ci mcp::` : FAIL.
- [ ] **Step 3 : implémenter.**
- [ ] **Step 4 : vérifier** — `cargo nextest run --profile ci mcp::` : PASS.
- [ ] **Step 5 : commit** `feat(mcp): list_folders, et list_agents dans l'ordre de la barre`.

### Task 4 : MCP en écriture — `create_folder`, `update_folder`, `delete_folder`, `move_agent`

**Files:** Modify `mcp/tools.rs`, `mcp/act.rs`, `mcp/tests.rs` (`EXPOSED`, `ACTING`, `NAMED_FOR_IT` :154 + renommer le test `…only_two_are_named_for_approving_or_deleting` :324), `mcp/private_tests.rs` (`naming_calls` :78-140), `mcp/act_tests.rs` (`required` :149-170), `src/lib/tools.ts` (`ESCOUADE_TOOLS` :19-74, args principaux :81), `src-tauri/src/agent.rs:1821` (`ESCOUADE_MAIN_ARGS` : ajouter `"folder"` en dernier, dans les deux fichiers), `src/lib/i18n/{fr,en}/mcp.ts` (bloc `tools`) · Test `mcp/act_tests.rs`, `src/lib/tools.test.ts`

**Interfaces — Consumes:** tâches 2-3. **Produces** (args en `Option` sauf mention, `project` facultatif pour un agent comme ailleurs, `may_act_on` sur le projet) :
- `create_folder { project, name: String, agents: Option<Vec<String>>, principal }` → le dossier comme `list_folders` ;
- `update_folder { project, folder: String, name, principal (""= aucun), position: Option<u32> }` ;
- `delete_folder { project, folder: String }` — `destructive_hint = false` (aucun agent n'est touché), dans `NAMED_FOR_IT` avec `"delete"` ;
- `move_agent { agent: String, folder (""= hors dossier), position, project }`.
Réponses : `reply_changed` avec un « ce qui a changé » bilingue (« Dossier « X » créé avec 3 agents. »).

- [ ] **Step 1 : tests rouges** : `a_folder_is_made_renamed_starred_and_deleted_through_mcp`, `move_agent_puts_an_agent_in_and_out_of_a_folder_at_a_position`, `a_private_project_s_folders_are_named_as_unknown` (dans `private_tests`), `required("create_folder") == ["name"]`, `required("move_agent") == ["agent"]` ; libellés dans `tools.test.ts`.
- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run --profile ci mcp::` ; `npx vitest run src/lib/tools.test.ts` : FAIL.
- [ ] **Step 3 : implémenter.**
- [ ] **Step 4 : vérifier** — mêmes commandes : PASS.
- [ ] **Step 5 : commit** `feat(mcp): les agents rangent leurs dossiers`.

### Task 5 : création automatique — `create_agent`, `start_ticket`

**Files:** Modify `mcp/tools.rs` (`CreateAgentArgs` :999, `StartTicketArgs` :740 : `folder: Option<String>`), `mcp/act.rs` (`create_agent` :1682 après l'étape 8 sous `acting` ; `start_ticket` :697 avant `ticket_start_on`), `folders.rs`, `model.rs` (`Ticket.folder: Option<String>`, `serde(default, skip_serializing_if)`), `tickets.rs` (`launch_ticket_agent` :2069 : `AgentOptions.folder = t.folder`) · Test `mcp/act_tests.rs`, `folders_tests.rs`

**Interfaces — Produces:** `Core::auto_folder(&self, project_id: &str, caller: &Caller, asked: Option<&str>) -> Result<Option<String>>` :
- `asked == Some("")` → `None` ;
- `asked == Some(x)` → dossier existant (id ou nom) ; sinon créé sous ce nom, `named = true`, l'appelant principal s'il est un agent de ce projet sans dossier ;
- `asked == None` et l'appelant est un agent **de ce projet** → son dossier ; sans dossier, un dossier créé à sa place, `name = son nom`, `named = false`, lui principal ;
- sinon (autre projet, `Caller::External`) → `None`.
`create_agent` passe le résultat dans `AgentOptions.folder` et répond `{ id, name, folder: {id,name}|null }` (adapter le test `act_tests.rs:962`, qui compte 2 clés). `start_ticket` le retient sur `Ticket.folder` (`edit_ticket`) et le dit dans sa réponse ; `launch_ticket_agent` le prend (dossier disparu → hors dossier) ; le pilote auto et la fenêtre ne posent jamais `Ticket.folder`.

- [ ] **Step 1 : tests rouges** : `an_agent_creating_agents_gets_a_folder_starred_with_them`, `the_second_creation_goes_in_the_same_folder`, `two_creations_at_once_make_one_folder` (deux `create_agent` en `tokio::join!`), `a_named_folder_is_made_or_reused`, `an_empty_folder_keeps_the_new_agent_loose`, `no_folder_in_another_project_nor_from_outside`, `a_ticket_started_by_an_agent_brings_its_agents_to_its_folder` (dont la relance après `ticket_resume` d'un agent disparu), `a_ticket_whose_folder_went_starts_its_agent_loose`.
- [ ] **Step 2 : vérifier l'échec** — `cargo nextest run --profile ci mcp:: folders_tests tickets_tests` : FAIL.
- [ ] **Step 3 : implémenter.**
- [ ] **Step 4 : vérifier** — même commande : PASS ; puis la suite entière `cargo nextest run --profile ci --test-threads 3` : PASS.
- [ ] **Step 5 : commit** `feat(mcp): les agents créés par un agent se rangent dans son dossier`.

### Task 6 : l'ordre affiché dans la fenêtre

**Files:** Modify `src/lib/sidebar.ts` (`sidebarOrder` :14 remplacé), `src/lib/sidebar.test.ts` (:21-31), `src/lib/state.svelte.ts` (`projectAgents` :411) · Test `src/lib/sidebar.test.ts`

**Interfaces — Produces:**
```ts
export type SidebarItem =
  | { kind: 'agent'; agent: Agent; lifted: boolean }
  | { kind: 'folder'; folder: AgentFolder; principal: Agent | null; agents: Agent[] /* sans le principal, ordre affiché */; running: number; waiting: number };
export function sidebarItems(project: Project, agents: Agent[] /* agents non archivés du projet */): SidebarItem[];
export function shownAgents(items: SidebarItem[]): Agent[];          // à plat, principal compris
export function foldedCards(item: Extract<SidebarItem, { kind: 'folder' }>, selectedId: string | undefined): Agent[]; // en attente + agent ouvert
```
`state.svelte.ts` : `projectItems = $derived(…sidebarItems…)`, `projectAgents = $derived(shownAgents(this.projectItems))`. Ainsi `Ctrl+Tab` (`shortcuts.ts:134`), `app.agent` (repli :433), `worktreeAgents` (Sidebar :622), `RunsSection.svelte:66` et `EditorView.svelte:756` suivent l'ordre affiché sans autre changement. `running` compte `status === 'running'`, `waiting` compte `status === 'waiting'`, principal compris.

- [ ] **Step 1 : tests rouges** : `sidebarItems follows the shared cases` (import de `tests/fixtures/sidebar-order.json`, compare `items` et `shown`), `foldedCards keeps the waiting ones and the open agent`, `the counts of a folder include its principal`.
- [ ] **Step 2 : vérifier l'échec** — `npx vitest run src/lib/sidebar.test.ts` : FAIL.
- [ ] **Step 3 : implémenter** ; supprimer `sidebarOrder` (Sidebar :139 passe à `app.projectItems`).
- [ ] **Step 4 : vérifier** — `npm test` et `npm run check` : PASS.
- [ ] **Step 5 : commit** `feat(barre): l'ordre affiché suit les dossiers`.

### Task 7 : dépôt et clavier (logique pure)

**Files:** Create `src/lib/agent-folders.ts`, `src/lib/agent-folders.test.ts`

**Interfaces — Produces:**
```ts
export type Place = { folderId: string | null; index: number | null }; // index : rang rangé une fois l'item retiré ; null = en bas
export function dropPlace(project: Project, items: SidebarItem[], dragged: string, level: string | null, beforeShown: string | null): Place | null; // null : dépôt interdit (dossier dans un dossier) ou sans effet
export function stepPlace(project: Project, items: SidebarItem[], item: string, by: -1 | 1): Place | null; // null aux bouts
```
`dropPlace` prend comme voisin le premier élément affiché **non remonté** à partir de `beforeShown` dans ce niveau. `stepPlace` : sur l'ordre rangé ; un agent au bord d'un dossier en sort (au-dessus ou au-dessous du dossier) ; un agent hors dossier qui arrive sur un dossier y entre (en haut en descendant, en bas en montant) ; un dossier saute l'élément voisin entier.

- [ ] **Step 1 : tests rouges** : `a waiting agent dropped keeps its lift` (le `Place` vise le voisin non remonté), `a folder never drops into a folder`, `dropping where it already is does nothing`, `Alt+Down at the bottom of a folder takes the agent out below it`, `Alt+Down onto a folder enters it at the top`, `Alt+Up onto a folder enters it at the bottom`, `a folder steps over a whole folder`.
- [ ] **Step 2 : vérifier l'échec** — `npx vitest run src/lib/agent-folders.test.ts` : FAIL.
- [ ] **Step 3 : implémenter.**
- [ ] **Step 4 : vérifier** — même commande : PASS.
- [ ] **Step 5 : commit** `feat(barre): où tombe un agent glissé ou déplacé au clavier`.

### Task 8 : la boîte dans la barre latérale

**Files:** Modify `src/components/Sidebar.svelte` (liste :797-1176, `agentMenu` :311-373, `newAgentMenu` :537-619, `cardKey` :261, renommage :114-308), `src/lib/i18n/{fr,en}/nav.ts` (`sidebar.folders.*`), `src/components/Sidebar.test.ts` · Test `src/components/Sidebar.test.ts`

**Exigences** (maquette A `.superpowers/brainstorm/40755-1793986207/content/dossiers-layout-v2.html`, `DESIGN.md`) :
- La carte d'agent devient un `{#snippet card(a)}` rendu hors dossier et dans un dossier, inchangé (`class="agent card"`, `data-agent`).
- Cadre `section.folder[data-folder]` : en-tête avec `button.chev` (`aria-expanded`, `aria-label` « Replier/Déplier <nom> »), nom (double-clic : renommage en ligne, même état `renaming` que les agents), nombre, pastilles vertes/ambres masquées à zéro, et un libellé pour les lecteurs d'écran (« 2 en cours, 1 en attente »). Corps décalé sous un filet ; principal en tête avec `★` couleur projet, `title`/`aria-label` « Agent principal ». Replié : seules les cartes de `foldedCards`. Vide : « Glisse un agent ici ».
- Glisser : `draggable` sur cartes et en-têtes (pas pendant un renommage), trait `.drop` comme `ProjectRail.svelte:396`, en-tête survolé surligné ; au dépôt `dropPlace` → `api.agentFolderMove`. Clavier : `Alt+↑/↓` sur carte (dans `cardKey`) et sur `button.chev` → `stepPlace` → `api.agentFolderMove`, focus gardé (`queueMicrotask`, `[data-agent]` / `[data-folder] .chev`).
- Menu d'agent, après Renommer : « Déplacer vers… » (second menu au même point, motif `launchMenu` :394 : chaque dossier, « Nouveau dossier », « Hors dossier »), « Nouveau dossier avec cet agent » (hors dossier), « Faire l'agent principal » / « Retirer l'étoile » (dans un dossier), « Monter »/« Descendre » avec `hint` `nav.rail.upKey`/`downKey`, désactivés aux bouts (`stepPlace` null).
- Menu de l'en-tête (clic droit) : Renommer, Monter, Descendre, séparateur, « Supprimer le dossier » (`danger`, sans confirmation).
- Menu de « Nouvel agent » : « Nouveau dossier » → `api.agentFolderCreate(project.id, null)` puis renommage en ligne du dossier rendu.

- [ ] **Step 1 : tests rouges** (`resetApp` avec un projet portant `agentFolders`/`agentOrder`, `fakeBackend`, `render(Sidebar)`) : `a folder shows its principal first with a star`, `a folded folder still shows the agent that waits`, `the folder header says its counts to screen readers`, `Déplacer vers puts the agent in a folder` (`backend.called('agent_folder_move')`), `Alt+ArrowDown moves the focused card` (motif `ProjectRail.test.ts:166-195`), `dragging a card onto a folder asks for the move`, `Nouveau dossier in the new agent menu creates one`, `Supprimer le dossier asks for its deletion`. Avant de toucher un menu : `grep -rn "<libellé>" tests/e2e`.
- [ ] **Step 2 : vérifier l'échec** — `npx vitest run src/components/Sidebar.test.ts` : FAIL.
- [ ] **Step 3 : implémenter** (FR et EN).
- [ ] **Step 4 : vérifier** — `npm test`, `npm run check`, `npm run format:check` : PASS.
- [ ] **Step 5 : commit** `feat(barre): les dossiers d'agents dans la barre latérale`.

### Task 9 : de bout en bout, docs et spec

**Files:** Create `tests/e2e/agent-folders.spec.ts` · Modify `website/content/docs/{fr,en}/mcp.md` (tables Lire/Agir, ligne `list_agents`), `CHANGELOG.md`, `CHANGELOG.en.md` ; spec par `write_spec_page`

- [ ] **Step 1 : e2e** sur le modèle de `sidebar.spec.ts` et `mcp.spec.ts` (fixture `addProject`, `Control+N`, `send`, `side = page.locator('aside.side')`) :
  - `a folder made by hand takes an agent dragged in and out, and a click moves nothing` : trois agents, « Nouveau dossier » → nom « Front », `card.dragTo(header)`, clic sur une autre carte → ordre inchangé, glisser hors du dossier, `app.restart()` → disposition gardée ;
  - `a folded folder keeps showing its agent that waits for an answer` : un agent du dossier reçoit `question`, autre agent ouvert, replier → sa carte et ses choix visibles, répondre → la carte rentre ;
  - `an agent that creates agents through MCP gets a folder with them, starred` : `test.use({ preStart: { run: () => ({ mcpEnabled: true }) } })`, jeton de l'agent par `agentServer` (`mcp.spec.ts:233`), deux `tools/call create_agent` → un dossier au nom de l'agent, lui en tête avec « Agent principal », les deux dedans ; `list_folders` les rend ; `create_agent` avec `folder: ""` → hors dossier.
- [ ] **Step 2 : `npx tsc --noEmit -p tests/e2e`** (ou `npm run check` s'il les couvre) : propre. Les e2e tournent en CI Windows (voir vérification).
- [ ] **Step 3 : docs** : nouvelles lignes d'outils et paramètre `folder` dans `mcp.md` FR/EN ; une entrée « Dossiers d'agents » dans les deux CHANGELOG, dans leur format.
- [ ] **Step 4 : spec** : `write_spec_page` sur `fenetre/barre-laterale` (section « Dossiers d'agents » sous « Agents », menus de l'agent) et `harness/serveur-mcp` (outils, `folder`, ordre de `list_agents`), `sources: ["docs/superpowers/specs/2026-11-06-dossiers-d-agents-design.md"]` ; page `harness/serveur-mcp` : relire d'abord par `read_spec_page`.
- [ ] **Step 5 : commit** `test(e2e): les dossiers d'agents de bout en bout` puis `docs: les dossiers d'agents`.

---

## Vérification

1. **Local Mac** : `npm run format:check && npm run check && npm test` (Node 24) ; dans `src-tauri` : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --profile ci --test-threads 3` sans `CLAUDE_*` → tout vert.
2. **Vraie app sur le Mac** (mémoire `captures-app-sur-mac`) : `ESCOUADE_DATA_DIR=<jetable> npx tauri dev` avec un `state.json` minimal (un projet, quatre agents) ; vérifier à la main : créer un dossier, glisser un agent dedans puis dehors, un simple clic n'en déplace aucun, replier avec un agent en `question`, `Ctrl+Tab` dans l'ordre affiché, ★ sur le principal ; puis activer le serveur MCP et appeler `create_agent` / `list_folders` en `curl` avec le jeton d'un agent → dossier automatique. Captures `screencapture -x -o -l <id>` par PID, envoyées à l'utilisateur.
3. **CI Windows (e2e)** : quand l'utilisateur le dit, pousser sur le fork `Metalzoid/escouade` et suivre la CI (`gh run watch`) ; les 1 à 4 tests instables par shard connus (mémoire `e2e-windows-instables-ci`) ne comptent pas, `agent-folders.spec.ts` doit passer.
4. Revue finale de la branche (superpowers:requesting-code-review) avant de proposer la PR.

## Exécution (à choisir en approuvant)

- **Recommandé : sous-agents (superpowers:subagent-driven-development)**, en mode économe comme les plans précédents (implémenteurs et relecteurs Sonnet, une relecture par paire de tâches, revue finale Opus). Raison : 9 tâches enchaînées par leurs interfaces (Rust → MCP → TS), et une erreur d'ordre ou d'outil MCP livrée se paie chez chaque utilisateur.
- **Autre choix : en natif (superpowers:executing-plans)**, toutes les tâches dans cette session puis une seule revue finale. Moins cher, utile si le quota de 5 h serre de nouveau.
