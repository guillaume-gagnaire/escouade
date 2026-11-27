#!/usr/bin/env python3
# whats-up garde v2
"""Refuse a production release (push or PR to the prod branch) that brings no newer What's up ? note.

  python3 .whatsup/garde.py pre-push <remote> <url>   git pre-push hook: stdin = git's ref lines; exit 3 = refuse
  python3 .whatsup/garde.py claude                    Claude PreToolUse hook: stdin = hook JSON; exit 2 = refuse

Copied into each repo at install time: a git hook cannot point at the plugin cache, whose path moves with each version.
"""
import fnmatch
import json
import os
import re
import subprocess
import sys

MESSAGE = "Pas de fiche What's up ? pour cette mise en prod : lance /whats-up, ou écris la raison."
ZERO = re.compile(r"^0+$")
ECHAPPE = re.compile(r"^\s*Sans nouveautés :[ \t]*\S", re.M)
CORPS = re.compile(r"""(?<!\S)(?:--body|-b)(?:\s+|=)(?:"((?:[^"\\]|\\.)*)"|'([^']*)'|(\S+))""")
CORPS_FICHIER = re.compile(r"""(?<!\S)(?:--body-file|-F)(?:\s+|=)["']?([^\s"']+)""")
# A heredoc body (`--body "$(cat <<'EOF' … EOF)"`, the usual form from Claude) may hold quotes and flag-like text.
HEREDOC = re.compile(r"""<<-?\s*(['"]?)(\w+)\1[^\n]*\n.*?^\s*\2\s*$""", re.S | re.M)
REFUS = 3  # pre-push: its own code, so a hook can tell a refusal from a crash


class Outillage(Exception):
    """git or config failed: never block a release on a tooling failure."""


def cle(id_):
    date, _, suffixe = id_[:10], id_[10:11], id_[11:]
    return (date, int(suffixe) if suffixe.isdigit() else 1)


def derniere(ids):
    return max(ids, key=cle) if ids else ""


def config(repo):
    try:
        with open(os.path.join(repo, ".whatsup", "config.json"), encoding="utf-8") as f:
            c = json.load(f)
        return c if isinstance(c, dict) and c.get("prod") and c.get("dossier") else None
    except (OSError, ValueError):
        return None


def _git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)


def ids_sur(repo, ref, dossier):
    """Note ids of index.json at `ref`; None when absent; ValueError when malformed; Outillage when `ref` is unknown."""
    if _git(repo, "cat-file", "-e", f"{ref}^{{commit}}").returncode:
        raise Outillage(f"{ref} introuvable")
    r = _git(repo, "show", f"{ref}:{dossier.strip('/')}/index.json")
    if r.returncode:
        return None
    index = json.loads(r.stdout)
    ids = index.get("ids") if isinstance(index, dict) else None
    if not isinstance(ids, list) or not all(isinstance(i, str) for i in ids):
        raise ValueError("ids")
    return ids


def decider(part, prod):
    if prod is None:  # first release after install: it ships the component itself
        return True
    if part is None:  # the index was lost on what ships
        return False
    return cle(derniere(part) or "0") > cle(derniere(prod) or "0")


def _sans_corps(cmd):
    return CORPS.sub(" ", HEREDOC.sub(" ", cmd))


def pr_vers_prod(cmd, prod):
    """Head branch of a `gh pr create` to prod ("" = current branch); None when the command is not one."""
    sans = _sans_corps(cmd)
    if not re.search(r"\bgh\s+pr\s+create\b", sans):
        return None
    base = re.search(r"""(?<!\S)(?:--base|-B)(?:\s+|=)["']?([^\s"']+)""", sans)
    if not base or base.group(1) != prod:
        return None
    head = re.search(r"""(?<!\S)(?:--head|-H)(?:\s+|=)["']?([^\s"']+)""", sans)
    return head.group(1) if head else ""


def echappe(cmd, corps_fichier=""):
    # The whole command: a body may be a heredoc whose quotes defeat any body parsing.
    return bool(ECHAPPE.search(cmd.replace("\\n", "\n")) or ECHAPPE.search(corps_fichier))


def _motif_tag(prod):
    """`tag:v*` → `v*` (a release is a pushed tag); None when prod is a branch."""
    return prod[4:] if prod.startswith("tag:") else None


def poussees(lignes, prod):
    """(local sha, remote sha) of each pushed line that releases to prod (deletions excluded)."""
    motif = _motif_tag(prod)
    out = []
    for ligne in lignes.splitlines():
        champs = ligne.split()
        if len(champs) != 4 or ZERO.match(champs[1]):
            continue
        ref = champs[2]
        if (ref.startswith("refs/tags/") and fnmatch.fnmatchcase(ref[10:], motif)) if motif else ref == f"refs/heads/{prod}":
            out.append((champs[1], champs[3]))
    return out


def _tag_precedent(repo, sha, motif):
    """The release before `sha`: the newest matching tag reachable from its parent; None for the first release."""
    r = _git(repo, "describe", "--tags", "--abbrev=0", f"--match={motif}", f"{sha}^{{commit}}^")
    return r.stdout.strip() or None if not r.returncode else None


def _juger(repo, c, part_ref, prod_ref):
    try:
        part = ids_sur(repo, part_ref, c["dossier"])
    except ValueError:
        part = None
    try:
        prod = ids_sur(repo, prod_ref, c["dossier"]) if prod_ref else None
    except ValueError:
        prod = None
    return decider(part, prod)


def _avertir(pourquoi):
    sys.stderr.write(f"⚠ What's up ? : vérification impossible ({pourquoi}), laissé passer.\n")


def mode_pre_push(repo, entree):
    c = config(repo)
    if not c:
        return 0
    lignes = poussees(entree, c["prod"])
    if not lignes:
        return 0
    if os.environ.get("WHATSUP_SANS", "").strip():
        return 0
    try:
        motif = _motif_tag(c["prod"])
        for local, distant in lignes:
            prod_ref = _tag_precedent(repo, local, motif) if motif else (None if ZERO.match(distant) else distant)
            if not _juger(repo, c, f"{local}^{{commit}}", prod_ref):
                sys.stderr.write(MESSAGE + "\n")
                return REFUS
    except Outillage as e:
        _avertir(e)
    return 0


def _dossier_de_travail(defaut, donnees, cmd):
    """Where the command runs: the hook's cwd, then a leading `cd <dir> &&`, then the repo top-level."""
    d = donnees.get("cwd") if isinstance(donnees.get("cwd"), str) else defaut
    m = re.match(r"""\s*cd\s+(?:"([^"]+)"|'([^']+)'|(\S+))\s*&&""", cmd)
    if m:
        d = os.path.join(d, os.path.expanduser(next(g for g in m.groups() if g)))
    top = _git(d, "rev-parse", "--show-toplevel") if os.path.isdir(d) else None
    return top.stdout.strip() if top and not top.returncode else d


def mode_claude(repo, entree):
    try:
        donnees = json.loads(entree)
        cmd = donnees["tool_input"]["command"]
    except (ValueError, KeyError, TypeError):
        return 0
    if not isinstance(cmd, str) or not isinstance(donnees, dict):
        return 0
    repo = _dossier_de_travail(repo, donnees, cmd)
    c = config(repo)
    if not c:
        return 0
    head = pr_vers_prod(cmd, c["prod"])
    if head is None:
        return 0
    f = CORPS_FICHIER.search(cmd)
    texte = ""
    if f:
        try:
            with open(os.path.join(repo, f.group(1)), encoding="utf-8") as fh:
                texte = fh.read()
        except OSError:
            pass
    if echappe(cmd, texte):
        return 0
    try:
        if _git(repo, "fetch", "-q", "origin", c["prod"]).returncode:
            raise Outillage("git fetch origin")
        head = head or _git(repo, "branch", "--show-current").stdout.strip()
        _git(repo, "fetch", "-q", "origin", head)
        part = f"origin/{head}" if not _git(repo, "rev-parse", "-q", "--verify", f"origin/{head}").returncode else head
        if not _juger(repo, c, part, f"origin/{c['prod']}"):
            sys.stderr.write(MESSAGE + "\n")
            return 2
    except Outillage as e:
        _avertir(e)
    return 0


if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    try:
        entree = sys.stdin.read()
        racine = os.getcwd()
        sys.exit({"pre-push": mode_pre_push, "claude": mode_claude}.get(mode, lambda *_: 0)(racine, entree))
    except Exception as e:  # never block a release on a crash of the guard itself
        _avertir(type(e).__name__)
        sys.exit(0)
