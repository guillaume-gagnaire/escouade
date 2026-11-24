---
number: null
id: "sous-macos-et-linux-l-arbre-d-un-agent-c"
title: "Sous macOS et Linux, l'arbre d'un agent compte et s'arrête au-delà de son groupe de process"
date: "2026-11-23"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-656"
pages: ["harness/process-des-agents"]
tickets: ["ESC-656"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/process-des-agents","added":11,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Sous macOS, l'utilisateur voyait la RAM et le CPU d'un agent sous-comptés. Sous Unix, le job d'un agent ne comptait (ps) et ne tuait (killpg) que le groupe de process que mène son process : tout ce que l'agent lance dans une session ou un groupe à lui (spawn détaché de Node, commande de l'outil Bash de Claude Code, serveur MCP, sous-agent hors process) n'était ni compté ni arrêté, alors que le Job de Windows tient toute la descendance. Un test (job.rs, perl qui prend sa session) l'a montré sur le code d'avant : pids ne voyait que sh, et perl survivait à terminate. Sous macOS, rss diffère aussi de la « Mémoire » du Moniteur d'activité.

## Décision

L'arbre d'un agent sous Unix = son groupe, la descendance de ses process par le ppid, les groupes que ces descendants mènent, et ce qui en a été vu et vit encore (pid et heure de début identiques). usage, pids et terminate s'appliquent à cet arbre ; un échantillon des ressources lit ps une seule fois pour tous les agents (job::Sample, EngineProcess::usage_in). terminate relit l'arbre, vérifie l'heure de début de chaque process (/proc sous Linux, proc_pidinfo sous macOS) juste avant SIGKILL, et ne touche jamais un pid repris ni le groupe d'un meneur dont le pid a été repris. La mémoire est l'empreinte physique sous macOS (proc_pid_rusage, RUSAGE_INFO_V2, ri_phys_footprint, par libc, sans nouvelle crate), rss sous Linux.

## Conséquences

Les chiffres d'un agent sous macOS et Linux montent (sa vraie descendance) et se rapprochent de Windows et du Moniteur d'activité. Sous Linux, la somme des rss compte encore une fois par process les bibliothèques partagées (le PSS de /proc/<pid>/smaps_rollup le corrigerait, au prix d'une lecture par process). Un arrêt d'agent coûte une lecture de ps de plus (quelques au plus) ; le drop d'un job dont le meneur a fini et qui n'a rien vu hors de son groupe (une commande git) n'en lit aucune. Restent hors d'atteinte : un process lancé à part dont le parent a fini avant tout échantillon, et ce que les groupes laissés par une app tuée avaient lancé à part.
