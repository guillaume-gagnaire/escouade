import { defineZone } from '../types';

// The settings window and the settings of a project.
export default defineZone('settings', {
  tabs: {
    app: { label: 'Application', desc: 'Language of the interface and of texts written by Claude' },
  },
  app: {
    language: 'Language',
    uiLanguage: 'Interface language',
    system: 'System ({lang})',
    claudeLanguage: 'Language of texts written by Claude',
    sameAsUi: 'Same as the interface (default)',
    claudeLanguageHelp:
      'Proposed commit messages and pull request descriptions, comments posted to Jira, Trello, and GitHub, instructions given to agents.',
    langName: { fr: 'French', en: 'English' },
  },
});
