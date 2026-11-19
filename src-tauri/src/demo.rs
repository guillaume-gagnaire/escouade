//! The demonstration project of the first launch (ESC-505, ON-2, `docs/ONBOARDING.md` § 4.4, choice O3): a small
//! throwaway git repository the app writes on this machine, without the network, to try an agent, a permission, a diff
//! and a branch without any risk for a real repository.
//!
//! It lives in `<data>/demo/bac-a-sable` (`~/.escouade/demo/bac-a-sable`) and nowhere else: its agents' worktrees are
//! inside it (`.claude/worktrees`, the default), its history too. It holds a to-do list in TypeScript and two tests that
//! fail on purpose, run by `node --test` with nothing to install. It is never removed by itself: « Supprimer » removes
//! it, its project and its folder, and nothing outside that folder.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// The folder's name, under `<data>/demo`.
pub const FOLDER: &str = "bac-a-sable";

/// Where the demonstration lives for the data folder `root`.
pub fn dir(root: &Path) -> PathBuf {
    root.join("demo").join(FOLDER)
}

/// The same folder, whatever the spelling (`\` or `/`, a trailing separator, the case under Windows).
pub fn same(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    norm(a) == norm(b)
}

/// Whether `path` is the demonstration of the data folder `root`.
pub fn is_demo(root: &Path, path: &Path) -> bool {
    same(&dir(root), path)
}

/// The files of the demonstration: a to-do list, its tests (two fail on purpose), how to run them.
pub fn files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("README.md", README),
        ("package.json", PACKAGE),
        (".gitignore", "node_modules/\n"),
        ("src/todo.ts", TODO),
        ("test/todo.test.ts", TESTS),
    ]
}

const README: &str = "# bac-a-sable

Le projet de démonstration d’Escouade : une petite liste de tâches en TypeScript, écrite sur ta machine, sans réseau.
Rien ne sort d’ici. Deux tests échouent exprès : demande à un agent de les faire passer.

- `npm test` (ou `node --test test/todo.test.ts`) lance les tests, sans rien à installer (Node 22.18 ou plus récent).
- « Supprimer », sur la bande du projet dans Escouade, retire ce dossier et son projet.

---

Escouade’s demo project: a small to-do list in TypeScript, written on your machine, offline. Nothing leaves it. Two
tests fail on purpose: ask an agent to make them pass. `npm test` runs them, with nothing to install (Node 22.18 or
later). “Delete”, on the project’s band in Escouade, removes this folder and its project.
";

const PACKAGE: &str = r#"{
  "name": "bac-a-sable",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "description": "Le projet de démonstration d'Escouade : une liste de tâches, deux tests qui échouent exprès.",
  "scripts": {
    "test": "node --test test/todo.test.ts"
  }
}
"#;

const TODO: &str = "export interface Todo {
  id: number;
  title: string;
  done: boolean;
}

export class TodoList {
  private items: Todo[] = [];
  private next = 1;

  add(title: string): Todo {
    const todo: Todo = { id: this.next++, title: title.trim(), done: false };
    this.items.push(todo);
    return todo;
  }

  toggle(id: number): void {
    const todo = this.items.find((t) => t.id === id);
    if (todo) todo.done = !todo.done;
  }

  /** The tasks still to do. */
  remaining(): Todo[] {
    return this.items.filter((t) => t.done);
  }

  /** Removes the tasks done; how many were removed. */
  clearDone(): number {
    const before = this.items.length;
    this.items = this.items.filter((t) => !t.done);
    return this.items.length;
  }

  all(): Todo[] {
    return [...this.items];
  }
}
";

const TESTS: &str = "import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TodoList } from '../src/todo.ts';

test('adds a task, trimmed, not done', () => {
  const list = new TodoList();
  const todo = list.add('  Écrire le README  ');
  assert.deepEqual(todo, { id: 1, title: 'Écrire le README', done: false });
});

test('toggles a task', () => {
  const list = new TodoList();
  const { id } = list.add('Relire');
  list.toggle(id);
  assert.equal(list.all()[0].done, true);
});

test('lists the tasks still to do', () => {
  const list = new TodoList();
  list.add('Coder');
  const done = list.add('Tester');
  list.toggle(done.id);
  assert.deepEqual(
    list.remaining().map((t) => t.title),
    ['Coder'],
  );
});

test('says how many tasks were cleared', () => {
  const list = new TodoList();
  list.add('Coder');
  const done = list.add('Tester');
  list.toggle(done.id);
  assert.equal(list.clearDone(), 1);
});
";

/// A git command in `cwd` with Escouade's own identity, the user's hooks and signing left out: the demonstration's
/// commit is the app's, whatever the machine's git says (a new machine may have no `user.name`).
async fn git(cwd: &Path, args: &[&str]) -> Result<()> {
    let cwd = cwd.to_string_lossy().to_string();
    let mut all: Vec<&str> = vec![
        "-c",
        "user.name=Escouade",
        "-c",
        "user.email=demo@escouade.invalid",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "core.autocrlf=false",
    ];
    all.extend_from_slice(args);
    crate::git::run(&cwd, &all).await.map(|_| ())
}

/// Whether `dir` is a demonstration written already (its repository and its first commit there).
pub fn written(dir: &Path) -> bool {
    dir.join(".git").join("HEAD").is_file() && dir.join("package.json").is_file()
}

/// The demonstration written in `<root>/demo/bac-a-sable`, a git repository with one commit on `main`; the folder. One
/// written already is kept as it is (« Créer » again opens it). A folder there that is no demonstration is left alone,
/// and refused.
pub async fn create(root: &Path) -> Result<PathBuf> {
    let dir = dir(root);
    if written(&dir) {
        return Ok(dir);
    }
    if dir.exists()
        && std::fs::read_dir(&dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        let p = dir.display();
        bail!(tr!(
            "{p} existe déjà et n’est pas le projet de démonstration : déplace-le, puis réessaie.",
            "{p} already exists and isn’t the demo project: move it, then try again."
        ));
    }
    std::fs::create_dir_all(&dir).with_context(|| dir.display().to_string())?;
    for (rel, body) in files() {
        let file = dir.join(rel);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, body).with_context(|| file.display().to_string())?;
    }
    let made = async {
        git(&dir, &["init", "-q", "--initial-branch=main"]).await?;
        git(&dir, &["add", "-A"]).await?;
        git(
            &dir,
            &[
                "commit",
                "-q",
                "--no-verify",
                "-m",
                "Bac à sable : une liste de tâches, deux tests qui échouent exprès",
            ],
        )
        .await
    }
    .await;
    if let Err(e) = made {
        // Half made, it goes: « Réessayer » finds the place free.
        let _ = remove_tree(&dir);
        return Err(e);
    }
    Ok(dir)
}

/// The demonstration's folder removed, with all it holds (its worktrees, its history). Refused for any other folder:
/// nothing outside `<root>/demo/bac-a-sable` is ever touched.
pub fn remove(root: &Path, path: &Path) -> Result<()> {
    if !is_demo(root, path) {
        bail!(tr!(
            "Ce dossier n’est pas le projet de démonstration : Escouade ne le supprime pas.",
            "This folder isn’t the demo project: Escouade won’t delete it."
        ));
    }
    let dir = dir(root);
    if dir.exists() {
        remove_tree(&dir)?;
    }
    // `demo` itself goes once empty.
    let _ = std::fs::remove_dir(root.join("demo"));
    Ok(())
}

/// `dir` removed whole; a file git made read-only (its objects, under Windows) is made writable first.
fn remove_tree(dir: &Path) -> Result<()> {
    if std::fs::remove_dir_all(dir).is_ok() {
        return Ok(());
    }
    fn writable(p: &Path) {
        if let Ok(meta) = std::fs::symlink_metadata(p) {
            if meta.is_dir() {
                if let Ok(entries) = std::fs::read_dir(p) {
                    for e in entries.flatten() {
                        writable(&e.path());
                    }
                }
            } else {
                let mut perms = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(p, perms);
            }
        }
    }
    writable(dir);
    std::fs::remove_dir_all(dir).with_context(|| dir.display().to_string())
}

impl crate::core::Core {
    /// « Un projet de démonstration »: written if need be, then added as a project (an agent's worktree in it, the
    /// default), or the project it is already.
    pub async fn demo_create(
        self: &std::sync::Arc<Self>,
        color: &str,
    ) -> Result<crate::model::Project> {
        let root = self.data.root().to_path_buf();
        let at = dir(&root);
        let open = self
            .projects
            .read()
            .iter()
            .find(|p| same(Path::new(&p.path), &at))
            .cloned();
        if let Some(p) = open {
            return Ok(p);
        }
        let made = create(&root).await?;
        let (project, _) = self
            .add_project(
                &made.to_string_lossy(),
                FOLDER,
                color,
                true,
                None,
                false,
                None,
            )
            .await?;
        Ok(project)
    }

    /// « Supprimer » of the demonstration: its project closed and erased (its agents stopped, their history gone), then
    /// its folder removed. Any other project is refused.
    pub async fn demo_remove(self: &std::sync::Arc<Self>, id: &str) -> Result<()> {
        let project = self.project(id)?;
        let root = self.data.root().to_path_buf();
        if !is_demo(&root, Path::new(&project.path)) {
            bail!(tr!(
                "Ce projet n’est pas le projet de démonstration : Escouade ne le supprime pas.",
                "This project isn’t the demo project: Escouade won’t delete it."
            ));
        }
        self.remove_project(id, true).await?;
        let path = PathBuf::from(&project.path);
        // Its agents' processes just ended may hold a file a moment (Windows): tried again a few times.
        let mut last = None;
        for _ in 0..10 {
            let (r, p) = (root.clone(), path.clone());
            match tokio::task::spawn_blocking(move || remove(&r, &p)).await? {
                Ok(()) => return Ok(()),
                Err(e) => last = Some(e),
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        Err(last.expect("tried"))
    }
}

#[cfg(test)]
#[path = "demo_tests.rs"]
mod tests;
