//! Sharing the docs (ESC-257, `docs/FACTORY.md` § 3): the product owner never sees git, only
//! « N changements à partager » and « Partager ».
//!
//! - **What is left to share**: the files of the docs (`.escouade/spec`, `decisions`, `documents`)
//!   changed in the branch of the docs and not committed, plus those the commits not pushed yet
//!   changed. Nothing else counts: the code of the same branch is not the docs' business.
//! - **« Partager »**, at the user's gesture alone: the remote branch is fetched first, the changes
//!   of the docs (and only them) are committed with a message that tells what they are, what the
//!   others did meanwhile is merged in, and the branch is pushed. The credentials are those git
//!   already has, as for the git panel's push.
//! - **A conflict** (two people touched the same page) is undone and handed to the writer agent
//!   (« L'agent résout » / « Annuler »), never shown as lines of git: the merge is made again in
//!   the folder of the docs for the agent to resolve and commit, and the user shares again once it
//!   has.
//! - Escouade never shares by itself: nothing here runs but at a command of the window (or a
//!   caller of the tool who was told to).

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use serde::Serialize;

use crate::core::{AgentOptions, Core};
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::UiEvent;
use std::sync::Arc;

/// The folders of the docs under the repository's root.
pub const FOLDERS: [&str; 3] = [
    ".escouade/spec",
    ".escouade/decisions",
    ".escouade/documents",
];

/// One « Partager » at a time: two clicks must not commit and push twice.
static SHARING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// True for a file of the docs.
pub fn is_docs_path(path: &str) -> bool {
    FOLDERS
        .iter()
        .any(|f| path.strip_prefix(f).is_some_and(|r| r.starts_with('/')))
}

/// The conflict a « Partager » stopped on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    /// The pages (files) both sides changed.
    pub files: Vec<String>,
    /// The branch that was being merged in.
    pub target: String,
    /// The writer agent resolving it, once « L'agent résout » was clicked.
    pub agent_id: Option<String>,
}

/// What the window was last told of a project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memo {
    pub pending: u32,
    pub conflict: Option<Conflict>,
}

/// What a project's docs have to share now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    /// The files not shared yet.
    pub files: Vec<String>,
    pub count: u32,
    /// The commits not pushed yet that touch more than the docs: « Partager » would publish them
    /// with the docs, so it asks to be confirmed first.
    pub code_commits: u32,
    pub conflict: Option<Conflict>,
}

/// How a « Partager » ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Shared {
    /// Nothing to share: the docs were up to date.
    Nothing,
    /// Shared: the files committed (0 when only others' work came in) and the message; `pulled`
    /// when the others' work was merged in, and `received` the files of the docs it changed
    /// (ESC-534: « 2 changements récupérés »).
    #[serde(rename_all = "camelCase")]
    Done {
        committed: u32,
        message: String,
        pulled: bool,
        received: u32,
    },
    /// Stopped on a conflict, undone: the writer agent can resolve it.
    Conflict { conflict: Conflict },
}

/// What the branch tracks, or the remote branch of the same name when the branch does not track
/// it yet.
pub async fn target_of(dir: &str, st: &git::Status) -> Option<String> {
    if st.branch.is_empty() || st.branch == "(detached)" {
        return None;
    }
    if let Some(up) = st.upstream.as_ref().filter(|_| !st.upstream_gone) {
        return Some(up.clone());
    }
    let remotes = git::remotes(dir).await;
    let remote = git::sync_remote(&remotes, None)?;
    let target = format!("{remote}/{}", st.branch);
    git::ref_exists(dir, &format!("refs/remotes/{target}"))
        .await
        .then_some(target)
}

/// (commits ahead, commits behind) of HEAD against `target`.
async fn divergence(dir: &str, target: &str) -> (u32, u32) {
    let range = format!("HEAD...{target}");
    let out = git::text(dir, &["rev-list", "--left-right", "--count", &range])
        .await
        .unwrap_or_default();
    let mut n = out
        .split_whitespace()
        .map(|s| s.parse::<u32>().unwrap_or(0));
    (n.next().unwrap_or(0), n.next().unwrap_or(0))
}

/// The files of the docs changed and not shared yet in the branch checked out in `dir`: those
/// changed in the working tree, and those of the commits not pushed.
pub async fn pending_files(dir: &str) -> Result<Vec<String>> {
    let st = git::status(dir).await?;
    let mut files: BTreeSet<String> = st
        .entries
        .iter()
        .filter(|e| is_docs_path(&e.path))
        .map(|e| e.path.clone())
        .collect();
    if let Some(target) = target_of(dir, &st).await {
        let range = format!("{target}...HEAD");
        let mut args = vec![
            "-c",
            "core.quotepath=off",
            "--literal-pathspecs",
            "diff",
            "--name-only",
            "--no-renames",
            &range,
            "--",
        ];
        args.extend(FOLDERS);
        for f in git::text(dir, &args).await?.lines() {
            if is_docs_path(f) {
                files.insert(f.to_string());
            }
        }
    }
    Ok(files.into_iter().collect())
}

/// The commits not pushed yet that touch something else than the docs: those HEAD has and the
/// remote branch (or, for a branch not published, any remote) does not.
pub async fn code_commits(dir: &str) -> Result<u32> {
    let st = git::status(dir).await?;
    if st.branch.is_empty() || st.branch == "(detached)" {
        return Ok(0);
    }
    let mut args: Vec<String> = ["rev-list".into(), "--count".into()].into();
    match target_of(dir, &st).await {
        Some(target) => args.push(format!("{target}..HEAD")),
        None => args.extend(["HEAD".into(), "--not".into(), "--remotes".into()]),
    }
    args.push("--".into());
    args.push(".".into());
    args.extend(FOLDERS.iter().map(|f| format!(":(exclude){f}")));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    Ok(git::text(dir, &args).await?.trim().parse().unwrap_or(0))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The message of the commit of `files`: what they are, in the user's language.
pub fn commit_message(lang: Lang, files: &[String]) -> String {
    let count = |folder: &str| {
        files
            .iter()
            .filter(|f| f.strip_prefix(folder).is_some_and(|r| r.starts_with('/')))
            .count()
    };
    let (pages, decisions, documents) = (count(FOLDERS[0]), count(FOLDERS[1]), count(FOLDERS[2]));
    let mut parts = Vec::new();
    let fr = matches!(lang, Lang::Fr);
    if pages > 0 {
        parts.push(if fr {
            plural(pages, "page de la spec", "pages de la spec")
        } else {
            plural(pages, "spec page", "spec pages")
        });
    }
    if decisions > 0 {
        parts.push(if fr {
            plural(decisions, "décision", "décisions")
        } else {
            plural(decisions, "decision", "decisions")
        });
    }
    if documents > 0 {
        parts.push(plural(documents, "document", "documents"));
    }
    if parts.is_empty() {
        return tr_in!(lang, "Docs : mise à jour", "Docs: update");
    }
    let list = parts.join(", ");
    tr_in!(lang, "Docs : {list}", "Docs: {list}")
}

/// A failure of git told in a simple sentence: the user knows nothing of git.
pub fn plain(lang: Lang, what: &str, e: &anyhow::Error) -> anyhow::Error {
    let raw = format!("{e:#}");
    let low = raw.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| low.contains(w));
    let sentence = if has(&[
        "authentication failed",
        "could not read username",
        "could not read password",
        "permission denied",
        "access denied",
        "terminal prompts disabled",
        "invalid credentials",
        "returned error: 403",
        "returned error: 401",
        "repository not found",
        "authentification",
    ]) {
        tr_in!(
            lang,
            "Escouade n’a pas pu s’identifier auprès du dépôt distant. Vérifie que ton compte git (ou celui de l’hébergeur ajouté à Escouade) a accès à ce dépôt, puis recommence.",
            "Escouade couldn’t sign in to the remote repository. Check that your git account (or the host’s account added to Escouade) can reach this repository, then try again."
        )
    } else if has(&[
        "could not resolve host",
        "unable to access",
        "timed out",
        "connection refused",
        "network is unreachable",
    ]) {
        tr_in!(
            lang,
            "Le dépôt distant ne répond pas : vérifie ta connexion, puis recommence.",
            "The remote repository doesn’t answer: check your connection, then try again."
        )
    } else {
        tr_in!(
            lang,
            "{what} n’a pas abouti : {raw}",
            "{what} didn’t work: {raw}"
        )
    };
    anyhow::anyhow!(sentence)
}

/// Shares the docs of the branch checked out in `dir`; see the module's comment.
/// Unless `with_code`, it refuses to push commits of code with the docs (the docs are committed
/// first, locally: they stay safe, nothing leaves).
pub async fn share(dir: &str, message: Option<&str>, with_code: bool) -> Result<Shared> {
    let lang = i18n::ui();
    if let Some(op) = git::operation_in_progress(dir).await {
        bail!(tr_in!(
            lang,
            "Un {op} des docs est déjà en cours : l’agent rédacteur n’a pas fini de résoudre le conflit. Attends qu’il ait terminé, ou annule.",
            "A {op} of the docs is already under way: the writer agent hasn’t finished resolving the conflict. Wait for it to finish, or cancel."
        ));
    }
    let st = git::status(dir).await?;
    if st.branch.is_empty() || st.branch == "(detached)" {
        bail!(tr_in!(
            lang,
            "Les docs ne sont sur aucune branche : il n’y a rien à partager d’ici.",
            "The docs aren’t on any branch: there is nothing to share from here."
        ));
    }
    if git::remotes(dir).await.is_empty() {
        bail!(tr_in!(
            lang,
            "Ce projet n’a pas de dépôt distant : les docs n’ont nulle part où être partagées.",
            "This project has no remote repository: the docs have nowhere to be shared to."
        ));
    }
    // The remote branch first.
    git::fetch(dir, false)
        .await
        .map_err(|e| plain(lang, &tr_in!(lang, "La récupération", "Fetching"), &e))?;
    // Then the changes of the docs, and only them.
    let files: Vec<String> = git::status(dir)
        .await?
        .entries
        .into_iter()
        .filter(|e| is_docs_path(&e.path))
        .map(|e| e.path)
        .collect();
    let text = message
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| commit_message(lang, &files));
    if !files.is_empty() {
        git::commit_paths(dir, &files, &text)
            .await
            .map_err(|e| plain(lang, &tr_in!(lang, "L’enregistrement", "Saving"), &e))?;
    }
    // Code not pushed yet would leave with the docs: only when the user says so.
    let code = code_commits(dir).await?;
    if code > 0 && !with_code {
        bail!(tr_in!(
            lang,
            "Cette branche a aussi {code} commits de code pas encore partagés : « Partager » les publierait avec les docs. Confirme pour tout partager, ou partage le code depuis le panneau Git.",
            "This branch also has {code} commits of code not shared yet: “Share” would publish them with the docs. Confirm to share everything, or share the code from the Git panel."
        ));
    }
    // What the others did meanwhile.
    let st = git::status(dir).await?;
    let target = target_of(dir, &st).await;
    let mut pulled = false;
    let mut received = 0;
    if let Some(target) = &target {
        match merge_in(lang, dir, target).await? {
            Merged::Up => {}
            Merged::In(n) => (pulled, received) = (true, n),
            Merged::Conflict(conflict) => return Ok(Shared::Conflict { conflict }),
        }
    }
    let to_push = match &target {
        Some(t) => divergence(dir, t).await.0 > 0,
        // Git failed to read it: its error, nothing pushed on it.
        None => git::has_commit(dir).await?,
    };
    if to_push {
        git::push(dir)
            .await
            .map_err(|e| plain(lang, &tr_in!(lang, "Le partage", "Sharing"), &e))?;
    }
    if files.is_empty() && !to_push && !pulled {
        return Ok(Shared::Nothing);
    }
    Ok(Shared::Done {
        committed: files.len() as u32,
        message: if files.is_empty() {
            String::new()
        } else {
            text
        },
        pulled,
        received,
    })
}

/// How the others' work was brought in.
enum Merged {
    /// Nothing came: HEAD already has the remote branch.
    Up,
    /// Merged (in fast-forward when HEAD had nothing of its own): the files of the docs it changed.
    In(u32),
    /// Two sides touched the same files: the merge was undone.
    Conflict(Conflict),
}

/// Merges `target` into HEAD when HEAD is behind it: in fast-forward when it can, else a merge
/// commit. A conflict is undone and returned; another failure (changes of the code in the way) is
/// told in a simple sentence.
async fn merge_in(lang: Lang, dir: &str, target: &str) -> Result<Merged> {
    let (_, behind) = divergence(dir, target).await;
    if behind == 0 {
        return Ok(Merged::Up);
    }
    let before = git::text(dir, &["rev-parse", "HEAD"]).await?;
    if let Err(e) = git::run_hooks(dir, &["merge", "--no-edit", target]).await {
        let unmerged = git::unmerged(dir).await;
        let _ = git::run(dir, &["merge", "--abort"]).await;
        if unmerged.is_empty() {
            bail!(tr_in!(
                lang,
                "Ce que les autres ont partagé n’a pas pu être récupéré : des modifications du code, pas encore enregistrées, gênent. Enregistre-les ou mets-les de côté, puis recommence. ({e:#})",
                "What the others shared couldn’t be brought in: changes to the code, not saved yet, are in the way. Save them or set them aside, then try again. ({e:#})"
            ));
        }
        return Ok(Merged::Conflict(Conflict {
            files: unmerged,
            target: target.to_string(),
            agent_id: None,
        }));
    }
    Ok(Merged::In(docs_changed_since(dir, before.trim()).await))
}

/// The files of the docs that changed between `from` and HEAD.
async fn docs_changed_since(dir: &str, from: &str) -> u32 {
    let range = format!("{from}..HEAD");
    let mut args = vec![
        "-c",
        "core.quotepath=off",
        "--literal-pathspecs",
        "diff",
        "--name-only",
        "--no-renames",
        &range,
        "--",
    ];
    args.extend(FOLDERS);
    git::text(dir, &args)
        .await
        .map(|out| out.lines().filter(|f| is_docs_path(f)).count() as u32)
        .unwrap_or(0)
}

/// The synchronisation of the docs when none of them is left to share (ESC-534): the remote branch
/// fetched, then what the others shared there merged in (in fast-forward when it can); nothing is
/// committed nor pushed, the code is not published. `Nothing` when the docs were up to date, `Done`
/// with `received` the files of the docs that came in, or the conflict, undone, as « Partager »
/// gives it.
pub async fn pull(dir: &str) -> Result<Shared> {
    let lang = i18n::ui();
    if let Some(op) = git::operation_in_progress(dir).await {
        bail!(tr_in!(
            lang,
            "Un {op} des docs est déjà en cours : l’agent rédacteur n’a pas fini de résoudre le conflit. Attends qu’il ait terminé, ou annule.",
            "A {op} of the docs is already under way: the writer agent hasn’t finished resolving the conflict. Wait for it to finish, or cancel."
        ));
    }
    let st = git::status(dir).await?;
    if st.branch.is_empty() || st.branch == "(detached)" {
        bail!(tr_in!(
            lang,
            "Les docs ne sont sur aucune branche : il n’y a rien à récupérer ici.",
            "The docs aren’t on any branch: there is nothing to bring in here."
        ));
    }
    if git::remotes(dir).await.is_empty() {
        bail!(tr_in!(
            lang,
            "Ce projet n’a pas de dépôt distant : il n’y a rien à récupérer des autres.",
            "This project has no remote repository: there is nothing to bring in from the others."
        ));
    }
    git::fetch(dir, false)
        .await
        .map_err(|e| plain(lang, &tr_in!(lang, "La récupération", "Fetching"), &e))?;
    let st = git::status(dir).await?;
    let Some(target) = target_of(dir, &st).await else {
        // The branch is not published yet: nobody else wrote on it.
        return Ok(Shared::Nothing);
    };
    Ok(match merge_in(lang, dir, &target).await? {
        Merged::Up => Shared::Nothing,
        Merged::In(received) => Shared::Done {
            committed: 0,
            message: String::new(),
            pulled: true,
            received,
        },
        Merged::Conflict(conflict) => Shared::Conflict { conflict },
    })
}

/// The merge of `target` made again in `dir`, its conflicts left in the files for the writer agent.
/// The files in conflict; none when the merge went through (the others changed nothing since).
async fn merge_for_agent(dir: &str, target: &str) -> Result<Vec<String>> {
    match git::run_hooks(dir, &["merge", "--no-edit", target]).await {
        Ok(_) => Ok(Vec::new()),
        Err(e) => {
            let files = git::unmerged(dir).await;
            if files.is_empty() {
                let _ = git::run(dir, &["merge", "--abort"]).await;
                return Err(e);
            }
            Ok(files)
        }
    }
}

/// What the writer agent is asked when it resolves a conflict of the docs.
pub fn conflict_message(lang: Lang, target: &str, files: &[String]) -> String {
    tr_in!(
        lang,
        "Deux personnes ont modifié les mêmes pages des docs. Le merge de {target} dans la branche des docs a des conflits sur : {files}. Résous-les en gardant ce que chacun a voulu dire (les deux versions de la spec, des décisions, des documents), retire les marques de conflit, commite le merge, puis dis simplement que c’est fait. Ne touche à rien d’autre, ne pousse pas : l’utilisateur partagera.",
        "Two people changed the same pages of the docs. The merge of {target} into the docs branch has conflicts in: {files}. Resolve them keeping what each meant (both versions of the spec, the decisions, the documents), remove the conflict markers, commit the merge, then just say it is done. Touch nothing else and don’t push: the user will share.",
        files = files.join(", ")
    )
}

impl Core {
    /// Where the docs of `project_id` are, as git sees them.
    async fn docs_dir(&self, project_id: &str) -> Result<String> {
        Ok(self
            .docs_space(project_id, None)
            .await?
            .root
            .to_string_lossy()
            .to_string())
    }

    /// What the project's docs have to share, and the conflict being handled.
    pub async fn docs_pending(&self, project_id: &str) -> Result<Pending> {
        // On the team server (TS-9), the docs leave by themselves: nothing to share by git.
        if self.team_docs_on(&self.project(project_id)?) {
            return Ok(Pending {
                count: 0,
                files: Vec::new(),
                code_commits: 0,
                conflict: None,
            });
        }
        let dir = self.docs_dir(project_id).await?;
        let files = pending_files(&dir).await?;
        let code_commits = code_commits(&dir).await.unwrap_or(0);
        Ok(Pending {
            count: files.len() as u32,
            files,
            code_commits,
            conflict: self
                .docs_shares
                .lock()
                .get(project_id)
                .and_then(|m| m.conflict.clone()),
        })
    }

    /// Tells the window what the project's docs have to share when it changed since it was last
    /// told. A read that fails says nothing.
    pub async fn docs_changed(&self, project_id: &str) {
        // On the team server (TS-9): to its journal, and sent soon.
        self.team_docs_changed(project_id).await;
        let Ok(p) = self.docs_pending(project_id).await else {
            return;
        };
        let memo = Memo {
            pending: p.count,
            conflict: p.conflict.clone(),
        };
        {
            let mut shares = self.docs_shares.lock();
            let slot = shares.entry(project_id.to_string()).or_default();
            if *slot == memo {
                return;
            }
            *slot = memo;
        }
        self.hub.emit(UiEvent::DocsShare {
            project_id: project_id.to_string(),
            pending: p.count,
            conflict: p.conflict,
        });
    }

    fn docs_set_conflict(&self, project_id: &str, conflict: Option<Conflict>) {
        let mut shares = self.docs_shares.lock();
        let slot = shares.entry(project_id.to_string()).or_default();
        slot.conflict = conflict;
        // Whatever it was told, it is told again.
        slot.pending = u32::MAX;
    }

    /// « Partager »: see the module's comment. A conflict is kept for « L'agent résout » and
    /// « Annuler ».
    pub async fn docs_share(
        &self,
        project_id: &str,
        message: Option<&str>,
        with_code: bool,
    ) -> Result<Shared> {
        if self.team_docs_on(&self.project(project_id)?) {
            bail!(tr!(
                "Les docs de ce projet sont sur le serveur d’équipe : elles partent toutes seules.",
                "This project’s docs are on the team server: they leave by themselves."
            ));
        }
        let _one = SHARING.lock().await;
        let waiting = self
            .docs_shares
            .lock()
            .get(project_id)
            .and_then(|m| m.conflict.clone());
        if let Some(c) = waiting.filter(|c| c.agent_id.is_none()) {
            bail!(tr!(
                "Un conflit entre deux versions des docs attend ({n} pages) : demande à l’agent de le résoudre, ou annule.",
                "A conflict between two versions of the docs is waiting ({n} pages): ask the agent to resolve it, or cancel.",
                n = c.files.len()
            ));
        }
        let dir = self.docs_dir(project_id).await?;
        // The agent resolved it and committed the merge: nothing waits any more (if it did not
        // finish, `share` says so).
        if git::operation_in_progress(&dir).await.is_none() {
            self.docs_set_conflict(project_id, None);
        }
        let out = share(&dir, message, with_code).await;
        if let Ok(Shared::Conflict { conflict }) = &out {
            self.docs_set_conflict(project_id, Some(conflict.clone()));
        }
        self.docs_changed(project_id).await;
        out
    }

    /// The synchronisation of the docs with none of them left to share (ESC-534): what the others
    /// pushed on the docs branch brought in, see `pull`. One at a time with « Partager »; a conflict
    /// is kept for « L'agent résout » and « Annuler », as « Partager » keeps it.
    pub async fn docs_pull(&self, project_id: &str) -> Result<Shared> {
        let _one = SHARING.lock().await;
        let waiting = self
            .docs_shares
            .lock()
            .get(project_id)
            .and_then(|m| m.conflict.clone());
        if let Some(c) = waiting.filter(|c| c.agent_id.is_none()) {
            bail!(tr!(
                "Un conflit entre deux versions des docs attend ({n} pages) : demande à l’agent de le résoudre, ou annule.",
                "A conflict between two versions of the docs is waiting ({n} pages): ask the agent to resolve it, or cancel.",
                n = c.files.len()
            ));
        }
        let dir = self.docs_dir(project_id).await?;
        if git::operation_in_progress(&dir).await.is_none() {
            self.docs_set_conflict(project_id, None);
        }
        let out = pull(&dir).await;
        if let Ok(Shared::Conflict { conflict }) = &out {
            self.docs_set_conflict(project_id, Some(conflict.clone()));
        }
        self.docs_changed(project_id).await;
        out
    }

    /// « L'agent résout »: the merge is made again in the docs' folder, its conflicts left for the
    /// writer agent, which is asked to resolve them. If the others changed nothing since, the merge
    /// goes through and the conflict is gone.
    pub async fn docs_share_resolve(self: &Arc<Self>, project_id: &str) -> Result<Conflict> {
        let _one = SHARING.lock().await;
        let conflict = self
            .docs_shares
            .lock()
            .get(project_id)
            .and_then(|m| m.conflict.clone())
            .ok_or_else(|| {
                anyhow::anyhow!(tr!(
                    "Il n’y a pas de conflit des docs à résoudre.",
                    "There is no docs conflict to resolve."
                ))
            })?;
        if conflict.agent_id.is_some() {
            bail!(tr!(
                "L’agent rédacteur s’occupe déjà de ce conflit.",
                "The writer agent is already handling this conflict."
            ));
        }
        let dir = self.docs_dir(project_id).await?;
        let files = merge_for_agent(&dir, &conflict.target).await?;
        if files.is_empty() {
            self.docs_set_conflict(project_id, None);
            self.docs_changed(project_id).await;
            return Ok(Conflict {
                files: Vec::new(),
                ..conflict
            });
        }
        let lang = self.lang().claude;
        let created = self
            .create_agent_with(
                project_id,
                AgentOptions {
                    name: Some(tr!("redacteur-docs", "docs-writer")),
                    isolated: Some(false),
                    cwd: Some(dir.clone()),
                    select: true,
                    ..Default::default()
                },
            )
            .await;
        let agent = match created {
            Ok(a) => a,
            Err(e) => {
                let _ = git::run(&dir, &["merge", "--abort"]).await;
                return Err(e);
            }
        };
        let handed = Conflict {
            files: files.clone(),
            agent_id: Some(agent.meta.id.clone()),
            ..conflict
        };
        self.docs_set_conflict(project_id, Some(handed.clone()));
        if let Err(e) = self
            .send_message(
                &agent.meta.id,
                conflict_message(lang, &handed.target, &files),
                Vec::new(),
            )
            .await
        {
            log::warn!("docs conflict: the writer agent was not asked: {e:#}");
        }
        self.docs_changed(project_id).await;
        Ok(handed)
    }

    /// « Annuler »: the merge under way (if the agent was started) is undone and the conflict
    /// forgotten; the docs stay as the user had them, ready to share again.
    pub async fn docs_share_cancel(&self, project_id: &str) -> Result<()> {
        let _one = SHARING.lock().await;
        let dir = self.docs_dir(project_id).await?;
        if git::operation_in_progress(&dir).await == Some("merge") {
            git::run(&dir, &["merge", "--abort"]).await.map_err(|e| {
                anyhow::anyhow!(tr!(
                    "L’annulation n’a pas abouti : {e}",
                    "Cancelling didn’t work: {e}"
                ))
            })?;
        }
        self.docs_set_conflict(project_id, None);
        self.docs_changed(project_id).await;
        Ok(())
    }
}

/// A change of the spec already committed (ESC-258, « Ce qui a bougé » of the spec's map): who,
/// when, what it said and the pages it touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    /// Seconds since the epoch.
    pub when: i64,
    pub author: String,
    pub message: String,
    /// The pages (`subject/page`) it touched.
    pub pages: Vec<String>,
}

/// The last `limit` commits that touched the spec, newest first. A folder with no commit yet has none.
pub async fn recent(dir: &str, limit: usize) -> Vec<Moved> {
    let n = limit.to_string();
    let Ok(out) = git::run(
        dir,
        &[
            "log",
            "-n",
            &n,
            "--format=%x1e%at%x1f%an%x1f%s",
            "--name-only",
            "--",
            ".escouade/spec",
        ],
    )
    .await
    else {
        return Vec::new();
    };
    parse_recent(&String::from_utf8_lossy(&out))
}

/// `recent`'s reading of `git log`.
pub fn parse_recent(text: &str) -> Vec<Moved> {
    text.split('\u{1e}')
        .filter_map(|rec| {
            let mut lines = rec.lines();
            let head = lines.next()?;
            let mut f = head.split('\u{1f}');
            let when = f.next()?.trim().parse().ok()?;
            let author = f.next().unwrap_or_default().to_string();
            let message = f.next().unwrap_or_default().to_string();
            let pages = lines
                .filter_map(|l| l.trim().strip_prefix(".escouade/spec/"))
                .filter_map(|l| l.strip_suffix(".md"))
                .map(str::to_string)
                .collect();
            Some(Moved {
                when,
                author,
                message,
                pages,
            })
        })
        .collect()
}

impl Core {
    /// The last changes of the project's spec that were committed.
    pub async fn docs_recent(&self, project_id: &str) -> Result<Vec<Moved>> {
        Ok(recent(&self.docs_dir(project_id).await?, 6).await)
    }
}
