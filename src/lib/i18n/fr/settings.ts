import type { Tree } from '../types';

// The settings window and the settings of a project.
export default {
  tabs: {
    app: { label: 'Application', desc: 'Langue de l’interface et des textes rédigés par Claude' },
  },
  // The « Application » tab.
  app: {
    language: 'Langue',
    uiLanguage: 'Langue de l’interface',
    /** The system's language, named in the interface's: « Système (Français) ». */
    system: 'Système ({lang})',
    claudeLanguage: 'Langue des textes rédigés par Claude',
    sameAsUi: 'Comme l’interface (par défaut)',
    claudeLanguageHelp:
      'Messages de commit et descriptions de pull request proposés, commentaires publiés dans Jira, Trello et GitHub, consignes données aux agents.',
    /** A language named in the interface's (the choices name each in itself: `LANG_NAMES`). */
    langName: { fr: 'Français', en: 'Anglais' },
  },
} as const satisfies Tree;
