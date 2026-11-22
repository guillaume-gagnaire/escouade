import type { Tree } from '../types';

// The features switched on or off as a whole (F-1, ESC-550): Réglages › Fonctionnalités.
export default {
  /** The essential, never switched off. */
  essential: {
    name: 'Agents et conversation',
    desc: 'L’essentiel, toujours là : les agents et leur conversation, le panneau Git, les terminaux et les lancements.',
    always: 'Ne se coupe pas.',
  },
  kanban: {
    name: 'Kanban',
    desc: 'Les tickets et leurs critères, le pilote auto, les sources de tickets.',
    takes:
      'Coupé : son onglet, ses colonnes, ses réglages et ses outils MCP disparaissent, le pilote auto ne lance plus rien. Ses tickets restent.',
  },
  editor: {
    name: 'Éditeur',
    desc: 'Ouvrir, modifier et rechercher dans les fichiers du projet.',
    takes: 'Coupé : la vue de l’éditeur, Ctrl P et ses serveurs de langage s’arrêtent. Ses réglages restent.',
  },
  docs: {
    name: 'Espace Docs',
    desc: 'La spécification, les décisions, les documents et les discussions.',
    takes: 'Coupé : son onglet et ses outils MCP disparaissent. Les pages, décisions et documents restent.',
  },
  cost: {
    name: 'Coût',
    desc: 'Le coût des tours, des agents et des tickets ; la température du cache.',
    takes: 'Coupé : les montants et la température disparaissent partout. Les coûts restent comptés.',
  },
  stats: {
    name: 'Statistiques',
    desc: 'L’activité et les dépenses dans le temps.',
    takes: 'Coupé : sa vue disparaît du rail. Les chiffres restent comptés.',
  },
  workflows: {
    name: 'Workflows',
    desc: 'Les enchaînements d’agents et de skills d’un projet.',
    takes: 'Coupé : son onglet et son outil MCP disparaissent, plus rien n’est lu. Leurs fichiers restent.',
  },
  scheduled: {
    name: 'Tâches programmées',
    desc: 'Des tickets qui reviennent à heure fixe.',
    takes: 'Coupé : leur section et leurs outils MCP disparaissent, aucune tâche ne part. Les tâches restent.',
  },
  overseer: {
    name: 'Overseer',
    desc: 'L’agent qui pilote tous les projets, à qui tu parles.',
    takes: 'Coupé : son écran, son raccourci et ses outils MCP disparaissent, il s’arrête. Son journal reste.',
  },
  voice: {
    name: 'Voix',
    desc: 'La dictée et la conversation à voix haute.',
    takes: 'Coupé : le micro et son raccourci disparaissent, plus rien n’écoute. Ses réglages et ses modèles restent.',
  },
  /** The assistant (F-2, ESC-559): one screen per feature, what it does in three points, its switch. */
  screens: {
    kanban: {
      title: 'Le Kanban',
      lead: 'Des tickets avec leurs critères, que des agents prennent l’un après l’autre jusqu’à ce que tout soit vert, puis te rendent à valider.',
      p1: 'Le pilote auto lance les tickets prêts dès qu’une place se libère.',
      p2: 'Les tickets peuvent venir de Jira, GitHub, Linear, Trello…',
      p3: 'Rien n’entre dans ta branche sans ta validation.',
      activate: 'Activer le Kanban',
      kept: 'Tu l’utilises déjà : il reste allumé.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : il reste allumé.',
        other: 'Tu l’utilises déjà sur {count} projets : il reste allumé.',
      },
    },
    editor: {
      title: 'L’éditeur',
      lead: 'Ouvre un fichier du projet ou du worktree d’un agent, modifie-le et vois ce que l’agent a changé, sans quitter Escouade.',
      p1: 'Ctrl P ouvre n’importe quel fichier du projet.',
      p2: 'La coloration, la recherche et les serveurs de langage de VS Code.',
      p3: 'Les fichiers cités dans une conversation s’y ouvrent d’un clic.',
      activate: 'Activer l’éditeur',
      kept: 'Tu l’utilises déjà : il reste allumé.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : il reste allumé.',
        other: 'Tu l’utilises déjà sur {count} projets : il reste allumé.',
      },
    },
    docs: {
      title: 'L’espace Docs',
      lead: 'La spécification de ton projet, ses décisions et ses documents, tenus à jour par les agents des tickets.',
      p1: 'La spec en pages, que les agents lisent avant d’écrire du code.',
      p2: 'Les décisions, proposées par les agents, acceptées par toi.',
      p3: 'Les documents du projet, cherchés par le sens.',
      activate: 'Activer l’espace Docs',
      kept: 'Tu l’utilises déjà : il reste allumé.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : il reste allumé.',
        other: 'Tu l’utilises déjà sur {count} projets : il reste allumé.',
      },
    },
    cost: {
      title: 'Le coût',
      lead: 'Ce que coûtent chaque tour, chaque agent et chaque ticket, et la température du cache de chaque agent.',
      p1: 'Le coût du jour, dans la barre d’état.',
      p2: 'La température du cache : un agent chaud coûte bien moins cher à relancer.',
      p3: 'Ce que le cache t’a fait économiser.',
      activate: 'Activer le coût',
      kept: 'Tu l’utilises déjà : il reste allumé.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : il reste allumé.',
        other: 'Tu l’utilises déjà sur {count} projets : il reste allumé.',
      },
    },
    stats: {
      title: 'Les statistiques',
      lead: 'L’activité de tes agents et tes dépenses dans le temps, projet par projet.',
      p1: 'Les tours, les tokens et le coût, jour après jour.',
      p2: 'Par projet, par compte et par harness.',
      p3: 'Tout reste sur ta machine.',
      activate: 'Activer les statistiques',
      kept: 'Tu les utilises déjà : elles restent allumées.',
      keptOn: {
        one: 'Tu les utilises déjà sur {count} projet : elles restent allumées.',
        other: 'Tu les utilises déjà sur {count} projets : elles restent allumées.',
      },
    },
    workflows: {
      title: 'Les workflows',
      lead: 'Les enchaînements d’agents, de skills et de commandes de ton projet, lus dans ses dossiers et suivis étape par étape.',
      p1: 'Lus dans les dossiers .claude et .agents de chaque projet.',
      p2: 'L’étape de chaque ticket, sur sa carte.',
      p3: 'Un onglet à côté des fichiers, dans la barre du projet.',
      activate: 'Activer les workflows',
      kept: 'Tu les utilises déjà : ils restent allumés.',
      keptOn: {
        one: 'Tu les utilises déjà sur {count} projet : ils restent allumés.',
        other: 'Tu les utilises déjà sur {count} projets : ils restent allumés.',
      },
    },
    scheduled: {
      title: 'Les tâches programmées',
      lead: 'Des tickets qui reviennent à heure fixe : une revue chaque matin, la mise à jour des dépendances chaque lundi.',
      p1: 'Un rythme par jour, par semaine, ou en cron.',
      p2: 'Chaque passage est un ticket, qu’un agent fait avancer.',
      p3: 'Elles partent avec le Kanban.',
      activate: 'Activer les tâches programmées',
      kept: 'Tu les utilises déjà : elles restent allumées.',
      keptOn: {
        one: 'Tu les utilises déjà sur {count} projet : elles restent allumées.',
        other: 'Tu les utilises déjà sur {count} projets : elles restent allumées.',
      },
    },
    overseer: {
      title: 'L’Overseer',
      lead: 'Un agent qui pilote tous tes projets : tu lui parles, il lance, relance et répond aux agents qui attendent.',
      p1: 'Il suit tous tes projets, sauf les privés.',
      p2: 'Tu lui donnes des consignes ; il tient le journal de ce qu’il fait.',
      p3: 'Il ne tourne que si tu l’allumes dans ses réglages.',
      activate: 'Activer l’Overseer',
      kept: 'Tu l’utilises déjà : il reste allumé.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : il reste allumé.',
        other: 'Tu l’utilises déjà sur {count} projets : il reste allumé.',
      },
    },
    voice: {
      title: 'La voix',
      lead: 'Dicte tes messages, ou parle à tes agents à voix haute.',
      p1: 'Whisper sur ta machine : rien de ce que tu dis n’en sort.',
      p2: 'Ou un service en ligne, avec ta clé.',
      p3: 'Un micro dans chaque champ, et son raccourci.',
      activate: 'Activer la voix',
      kept: 'Tu l’utilises déjà : elle reste allumée.',
      keptOn: {
        one: 'Tu l’utilises déjà sur {count} projet : elle reste allumée.',
        other: 'Tu l’utilises déjà sur {count} projets : elle reste allumée.',
      },
    },
  },
  tour: {
    /** The station of the first launch. */
    question: 'Que veux-tu voir autour de tes agents ?',
    why: 'Une fonctionnalité par écran : allume celles qui te servent. L’essentiel est toujours là : les agents, le panneau Git, les terminaux et les lancements.',
    /** The window that opens by itself after the update that brings the choice. */
    updateTitle: 'Choisis tes fonctionnalités',
    updateLead: 'Escouade te laisse maintenant choisir ce qu’il te montre. Ce que tu utilises reste allumé ; le reste attend ton geste.',
    /** « Revoir la présentation ». */
    reviewTitle: 'Les fonctionnalités d’Escouade',
    list: 'Les fonctionnalités',
    /** `{n}` of `{total}`. */
    of: '{n} / {total}',
    ofLong: 'Fonctionnalité {n} sur {total}',
    shot: 'Capture d’Escouade : {name}',
    foot: 'Tout se change plus tard dans Réglages › Fonctionnalités.',
    skip: 'Passer : garder l’essentiel',
    previous: 'Précédent',
    next: 'Suivant',
    finish: 'Terminer',
    close: 'Fermer la présentation',
    /** Under the station, on the line. */
    on: { one: '{count} allumée', other: '{count} allumées' },
    none: 'l’essentiel seul',
    review: 'Revoir la présentation',
    reviewDesc: 'Chaque fonctionnalité, écran par écran, avec son interrupteur.',
    /** The window of the novelties: a feature an update brought. */
    newTitle: { one: 'Nouvelle fonctionnalité', other: 'Nouvelles fonctionnalités' },
    imposed: 'Toutes les fonctionnalités sont imposées sur ce poste : elles se montrent ici, sans se changer.',
  },
  page: {
    used: 'Utilisée',
    usedOn: { one: 'Utilisée sur {count} projet', other: 'Utilisée sur {count} projets' },
    needs: 'Part avec « {need} » : rallume-le d’abord.',
    foot: 'Une fonctionnalité coupée disparaît vraiment, ce qu’elle faisait en fond s’arrête ; ses données restent et reviennent quand tu la rallumes.',
  },
  /** A proposal at the right moment (F-3, ESC-558): the small window at the bottom right. */
  offer: {
    label: 'Proposition : {name}',
    kicker: 'Fonctionnalité coupée',
    noticed: 'Ce qu’Escouade a remarqué',
    would: 'Ce qu’elle ferait',
    private: 'Ce qu’Escouade remarque reste sur ta machine.',
    enable: 'Activer',
    later: 'Pas maintenant',
    never: 'Ne plus proposer',
    neverHint: 'Tu pourras la reproposer dans Réglages › Fonctionnalités.',
    signals: {
      mcpTickets: {
        one: '{count} ticket demandé au serveur MCP cette semaine a été refusé : le Kanban est coupé.',
        other: '{count} tickets demandés au serveur MCP cette semaine ont été refusés : le Kanban est coupé.',
      },
      longPrompts: {
        one: 'Tu as lancé {count} agent à la main cette semaine, sur une longue consigne.',
        other: 'Tu as lancé {count} agents à la main cette semaine, chacun sur une longue consigne.',
      },
      specEdits: {
        one: 'La spec de « {project} » (SPEC.md, docs/) a changé {count} jour ces deux dernières semaines.',
        other: 'La spec de « {project} » (SPEC.md, docs/) a changé sur {count} jours ces deux dernières semaines.',
      },
      documents: {
        one: 'Tu as glissé {count} document dans une conversation ces deux dernières semaines.',
        other: 'Tu as glissé {count} documents dans les conversations ces deux dernières semaines.',
      },
      usageLimit: {
        one: 'Un agent a atteint sa limite d’usage cette semaine.',
        other: 'Des agents ont atteint leur limite d’usage {count} fois cette semaine.',
      },
      agentFiles: {
        one: '« {project} » a {count} skill, commande ou agent (.claude/, .agents/).',
        other: '« {project} » a {count} skills, commandes ou agents (.claude/, .agents/).',
      },
      sameMessage: {
        one: 'Tu envoies la même consigne à la main, vers la même heure.',
        other: 'Tu envoies la même consigne à la main vers la même heure, {count} jours de suite.',
      },
      waitingProjects: {
        one: 'Des agents attendaient une réponse sur {count} projet.',
        other: 'Des agents attendaient ta réponse sur {count} projets à la fois.',
      },
    },
    does: {
      kanban: 'Le Kanban en ferait des tickets : chacun son agent, son worktree et ses critères, lancés par le pilote auto.',
      editor: 'L’éditeur ouvrirait les fichiers du projet à côté des agents.',
      docs: 'L’Espace Docs tiendrait la spec, les décisions et les documents du projet, que les agents lisent et citent.',
      cost: 'Le Coût montrerait ce que coûtent les tours, les agents et les tickets, et la température du cache, pour voir venir la limite.',
      stats: 'Les Statistiques montreraient l’activité et les dépenses dans le temps.',
      workflows: 'Les Workflows liraient ces enchaînements et montreraient l’étape où en est chaque agent.',
      scheduled: 'Une tâche programmée l’enverrait à ta place, à heure fixe, en ticket.',
      overseer: 'L’Overseer suivrait tous les projets à la fois et te dirait qui t’attend.',
      voice: 'La voix te laisserait dicter tes messages et parler aux agents.',
    },
  },
  /** « Ne plus proposer », given back (F-3): Réglages › Fonctionnalités. */
  declined: {
    title: 'Propositions écartées',
    desc: 'Ce que tu as choisi de ne plus te voir proposer quand ça servirait.',
    none: 'Aucune : Escouade peut te proposer une fonctionnalité coupée au moment où elle servirait, une à la fois, au plus une par jour.',
    restore: 'Reproposer',
    restoreLabel: 'Reproposer « {name} »',
  },
} as const satisfies Tree;
