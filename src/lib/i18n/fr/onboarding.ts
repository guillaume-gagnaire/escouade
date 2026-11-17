import type { Tree } from '../types';

// Le premier lancement (ESC-504, docs/ONBOARDING.md, direction C « La ligne ») : une station par question, sur une ligne
// tracée comme le git graph ; et l'icône Aide du rail, d'où il revient.
export default {
  title: 'Premier lancement',
  /** `{count}`: the stations. */
  subtitle: { one: '{count} station · environ 2 minutes', other: '{count} stations · environ 2 minutes' },
  /** The name of the line of the stations, for a screen reader. */
  line: 'Les stations du premier lancement',
  skipAll: 'Tout passer',
  later: 'Plus tard',
  continue: 'Continuer',
  back: 'Retour',
  /** Done again from Aide: it closes, nothing undone. */
  close: 'Fermer',
  /** The keys, beside their caps: « ↑ ↓ choisir · Entrée valider ». */
  choose: 'choisir',
  confirm: 'valider',
  enter: 'Entrée',
  resume: 'Tout se passe et se reprend. Le parcours revient depuis Aide › Premier lancement.',
  then: 'Ensuite : À découvrir',
  thenLanes: 'Planifier · Coder · Relire · Livrer · Piloter',
  /** `{n}`, the station's number; `{name}`, its name in a sentence. */
  order: 'Station {n} · {name}',
  /** Under a station's name in the line. */
  state: {
    now: 'maintenant',
    skipped: 'passé · à reprendre',
    policy: 'imposé',
  },
  stations: {
    harness: { name: 'Harness', inline: 'le harness', todo: 'le programme des agents' },
    account: { name: 'Compte', inline: 'le compte', todo: 'celui de la machine' },
    defaults: { name: 'Réglages de départ', inline: 'les réglages de départ', todo: 'le mode des agents' },
    project: { name: 'Premier projet', inline: 'le premier projet', todo: 'un dossier de ta machine' },
    telemetry: { name: 'Statistiques', inline: 'les statistiques', todo: 'partagées ou non' },
  },
  harness: {
    question: 'Avec quel harness tes agents travaillent-ils ?',
    why: 'Un harness est le programme d’agent de code de ta machine. Escouade le lance et le pilote ; il garde ta connexion et tes réglages.',
    ready: 'Connecté · {email}',
    readyNoEmail: 'Connecté sur cette machine',
    installed: 'Installé, pas connecté',
    installedHint: 'Tu t’y connecteras dans un terminal, juste après.',
    absent: 'Introuvable',
    tooOld: 'Trop ancien',
    unsupported: 'Version non prise en charge',
    copy: 'Copier',
    /** `{harness}`: its name. */
    copyLabel: 'Copier la commande de {harness}',
    copied: 'Commande copiée',
    none: 'Aucun harness n’est installé. Copie la commande de celui que tu veux, lance-la dans un terminal : Escouade le trouvera dès que tu reviens.',
    /** The harness chosen has no plugin installed yet: the seed's, with no network (ESC-373). */
    plugin: 'Escouade installe le plugin de {harness}, depuis l’app, sans réseau.',
    /** Signed in on the machine: the account station is folded into this one (choice O6). */
    machineAccount: 'Ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages › Comptes et harness.',
    /** The harness chosen goes first in « Ordre des harness ». */
    first: 'Le harness choisi passe en tête de l’ordre des harness : c’est celui des nouveaux projets.',
    /** `{harness}`: the only one a policy allows. */
    onlyAllowed: '{harness} est le seul harness permis sur ce poste.',
  },
  account: {
    question: 'Quel compte pour {harness} ?',
    why: 'Les quotas, les statistiques et la connexion sont tenus par compte. Tu pourras en ajouter d’autres (perso, pro) : Escouade passe au suivant quand l’un atteint sa limite.',
    machine: 'Le compte de la machine',
    machineEmail: 'Le compte de la machine · {email}',
    machineHint: 'Son dossier habituel, tel quel : ses réglages, sa connexion et ses conversations.',
    separate: 'Un compte à part',
    separateHint: 'Un dossier à lui sous ~/.escouade, et sa propre connexion.',
    notSignedIn: '{harness} n’est pas encore connecté sur cette machine : tu t’y connectes juste après, dans un terminal.',
    /** The account added already (elsewhere, or by this station). */
    added: 'Compte ajouté : {name}',
    noHarness: 'Choisis d’abord un harness, à la station précédente.',
  },
  defaults: {
    question: 'Quand tes agents doivent-ils te demander ?',
    /** `{model}`, `{effort}`: the account's, as Claude Code's are said. */
    why: 'Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Modèle {model}, effort {effort} : ceux de ton compte.',
    whyNoModel:
      'Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Effort {effort} : celui de ton compte, avec le modèle par défaut de ton harness.',
    bypass:
      'Éteint : l’agent ferait tout sans rien te montrer, même ce qu’un contenu piégé lui dicterait. À réserver à une machine jetable ; il s’allume dans Réglages › Agents.',
    bypassAllowed: 'Permis dans Réglages › Agents : jamais proposé comme mode de départ, il se choisit agent par agent.',
    off: 'Éteint',
    allowed: 'Permis',
    /** ESC-489: a harness whose only mode is Bypass. */
    onlyBypass: '{harness} ne tourne qu’en Bypass : ses agents agiront sans demander.',
    keep: 'Garder {mode}',
  },
  project: {
    question: 'Où travaille ton premier agent ?',
    why: 'Un projet est un dossier, avec ses agents, son Kanban et ses docs. Chaque agent y travaille dans son worktree, sur sa branche : deux agents ne se gênent pas, et rien n’entre dans ta branche sans toi.',
    folder: 'Un dossier de ta machine',
    folderHint: 'Un dépôt que tu as déjà. Escouade le lit sans rien y changer.',
    add: 'Ajouter un projet…',
    opened: 'Projet ouvert : {name}',
  },
  telemetry: {
    stays: 'Tes projets, ton code, tes conversations et tes comptes restent sur ta machine, dans ~/.escouade.',
    imposedOff: 'Éteintes par la stratégie de ce poste.',
    imposedOn: 'Allumées par la stratégie de ce poste.',
    answered: 'Ta réponse : {answer}. Elle se change dans Réglages › Général.',
  },
  end: {
    title: 'Escouade est prêt',
    body: 'Le reste se découvre en le faisant : la voix, le Kanban, les PR…',
    skipped: {
      one: '{count} étape passée : elle t’attend dans Aide › Premier lancement.',
      other: '{count} étapes passées : elles t’attendent dans Aide › Premier lancement.',
    },
    start: 'Commencer',
  },
  /** The rail's Aide (choice O10). */
  help: {
    label: 'Aide',
    tip: 'Le premier lancement, À découvrir, la documentation',
    firstLaunch: 'Premier lancement',
    firstLaunchDetail: 'Revoir le harness, le compte, le mode des agents',
    docs: 'Documentation',
    docsDetail: 'Le guide d’Escouade, sur escouade.dev',
    report: 'Signaler un problème',
    reportDetail: 'Sur GitHub',
  },
  /** Réglages › Général. */
  redo: {
    row: 'Premier lancement',
    rowDesc: 'Revoir le harness, le compte, le mode des nouveaux agents et les statistiques. Rien n’est remis à zéro.',
    button: 'Refaire le premier lancement',
  },
} as const satisfies Tree;
