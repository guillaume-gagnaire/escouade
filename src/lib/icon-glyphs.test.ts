// No character of Unicode serves as an icon in the components the 2.0 design has redrawn: they draw the icons of
// lib/icons.ts (Icon.svelte). Each lot of the redesign adds its components here (docs/TODO-2.0.md).

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { en } from './i18n/en';
import { fr } from './i18n/fr';

/** The components redrawn, by lot. */
const REDRAWN = {
  'lot 1 (ESC-71)': [
    'src/components/Icon.svelte',
    'src/components/IconSprite.svelte',
    'src/components/Level.svelte',
    'src/components/EngineMark.svelte',
    'src/components/Star.svelte',
    'src/components/PrivateMark.svelte',
    'src/components/ContextMenu.svelte',
    'src/components/Dropdown.svelte',
    'src/components/settings/Chips.svelte',
    'src/components/settings/Switch.svelte',
    'src/components/settings/SharedMark.svelte',
    'src/components/settings/Row.svelte',
    'src/components/settings/Group.svelte',
    'src/components/settings/GeneralPage.svelte',
  ],
  'lot 2 (ESC-72)': [
    'src/App.svelte',
    'src/components/TitleBar.svelte',
    'src/components/ProjectRail.svelte',
    'src/components/ProjectTile.svelte',
    'src/components/StatusBar.svelte',
    'src/components/QuotaMeter.svelte',
    'src/components/AccountsPanel.svelte',
    'src/components/Toasts.svelte',
    'src/components/Welcome.svelte',
  ],
  'lot 3 (ESC-73)': ['src/components/Sidebar.svelte', 'src/components/RunsSection.svelte', 'src/components/StatusDot.svelte'],
  'lot 4 (ESC-76)': [
    'src/components/Conversation.svelte',
    'src/components/Composer.svelte',
    'src/components/conv/TurnCard.svelte',
    'src/components/conv/ToolRow.svelte',
    'src/components/conv/PatchView.svelte',
    'src/components/conv/UserMessage.svelte',
    'src/components/conv/Thinking.svelte',
    'src/components/conv/Markdown.svelte',
    'src/components/conv/EventMessage.svelte',
    'src/components/conv/Notice.svelte',
    'src/components/conv/SetupProgress.svelte',
    'src/components/conv/CriteriaReport.svelte',
    'src/components/conv/PermissionCard.svelte',
    'src/components/conv/QuestionCard.svelte',
    'src/components/conv/Handover.svelte',
    'src/components/conv/NoAccount.svelte',
    'src/components/conv/Via.svelte',
    'src/components/DictationBand.svelte',
    'src/components/AttachmentChip.svelte',
    'src/components/MicButton.svelte',
    'src/components/ConversationButton.svelte',
    'src/components/ConversationOverlay.svelte',
    'src/components/SubagentView.svelte',
    'src/components/TaskView.svelte',
  ],
  'lot 5 (ESC-77)': ['src/components/PlanBanner.svelte'],
  'lot 6 (ESC-78)': [
    'src/components/board/Board.svelte',
    'src/components/board/BoardColumn.svelte',
    'src/components/board/TicketCard.svelte',
    'src/components/board/TicketForm.svelte',
    'src/components/board/RejectForm.svelte',
    'src/components/modals/ImportModal.svelte',
  ],
  'lot 7 (ESC-79)': [
    'src/components/SidePanel.svelte',
    'src/components/FilesPanel.svelte',
    'src/components/FileDiff.svelte',
    'src/components/DiffView.svelte',
    'src/components/DiffModal.svelte',
    'src/components/GitGraph.svelte',
    'src/components/WorkflowPanel.svelte',
    'src/components/branches/BranchPicker.svelte',
    'src/components/branches/NewBranchModal.svelte',
    'src/components/branches/MergedBranchesModal.svelte',
    'src/components/modals/CommitModal.svelte',
  ],
  'lot 8 (ESC-80)': [
    'src/components/TerminalView.svelte',
    'src/components/RunView.svelte',
    'src/components/AgentTerminal.svelte',
    'src/components/modals/TestLaunchModal.svelte',
    'src/components/modals/ConfirmModal.svelte',
  ],
  'lot 9 (ESC-81)': [
    'src/components/editor/EditorView.svelte',
    'src/components/editor/CodeEditor.svelte',
    'src/components/editor/EditorTabs.svelte',
    'src/components/editor/FileTree.svelte',
    'src/components/editor/FileIcon.svelte',
    'src/components/editor/NewFileField.svelte',
    'src/components/editor/SearchPanel.svelte',
    'src/components/editor/SourcePicker.svelte',
    'src/components/editor/TargetPicker.svelte',
    'src/components/QuickOpen.svelte',
    'src/components/ConvSearch.svelte',
    'src/lib/editor/find.ts',
    'src/lib/editor/theme.ts',
  ],
  'lot 10 (ESC-82)': ['src/components/Overview.svelte', 'src/components/Stats.svelte'],
  'lot 11a (ESC-83)': [
    'src/components/modals/Modal.svelte',
    'src/components/modals/UpdateModal.svelte',
    'src/components/modals/NewProjectModal.svelte',
    'src/components/modals/AccountModal.svelte',
    'src/components/modals/PrModal.svelte',
    'src/components/modals/TelemetryModal.svelte',
    'src/components/wizard/AccountStep.svelte',
    'src/components/wizard/Checks.svelte',
    'src/components/wizard/CopiedStep.svelte',
    'src/components/wizard/IntegrationsStep.svelte',
    'src/components/wizard/KanbanStep.svelte',
    'src/components/wizard/LaunchStep.svelte',
    'src/components/wizard/SummaryStep.svelte',
    'src/components/wizard/WorktreesStep.svelte',
    'src/components/Splitter.svelte',
  ],
  'lot 11b (ESC-152)': [
    'src/components/modals/SettingsModal.svelte',
    'src/components/settings/AccountsTab.svelte',
    'src/components/settings/AboutPage.svelte',
    'src/components/settings/AdvancedPage.svelte',
    'src/components/settings/AgentsPage.svelte',
    'src/components/settings/McpActivity.svelte',
    'src/components/settings/NotificationsPage.svelte',
    'src/components/settings/BoardTab.svelte',
    'src/components/settings/ClosedProjects.svelte',
    'src/components/settings/ForgesSection.svelte',
    'src/components/settings/McpSection.svelte',
    'src/components/settings/ModelFavorites.svelte',
    'src/components/settings/NewProjects.svelte',
    'src/components/settings/OnlineKey.svelte',
    'src/components/settings/OnlineModel.svelte',
    'src/components/settings/OnlineVoiceField.svelte',
    'src/components/settings/ProjectPage.svelte',
    'src/components/settings/GitPage.svelte',
    'src/components/settings/LaunchPage.svelte',
    'src/components/settings/TeamPage.svelte',
    'src/components/settings/PrivacyPage.svelte',
    'src/components/settings/Proposal.svelte',
    'src/components/settings/SharedSection.svelte',
    'src/components/settings/SpeechSettings.svelte',
    'src/components/settings/StepList.svelte',
    'src/components/settings/VoiceTab.svelte',
    'src/components/settings/WorktreeBaseField.svelte',
  ],
};

/**
 * What draws an icon with a character: arrows, mathematical and technical signs, geometric shapes, dingbats,
 * pictographs, and the few others the 1.x interface used (× ✓ ✦ ⧉ ⋯ •).
 */
const ICON_GLYPH = /[×•←-⇿⋯⌀-⏿■-◿☀-➿⟀-⟿⤀-⥿⧉⬀-⯿]|[🌀-🫿]/u;

/** The lines of a component that hold one, outside its comments. */
function glyphs(file: string): string[] {
  const text = readFileSync(file, 'utf8')
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '');
  return text
    .split('\n')
    .map((line, i) => [line.replace(/(^|[^:])\/\/.*$/, '$1'), i + 1] as const)
    .filter(([line]) => ICON_GLYPH.test(line))
    .map(([line, n]) => `${file}:${n}: ${line.trim()}`);
}

describe('the icons of the redrawn components', () => {
  it('knows a glyph used as an icon from a letter', () => {
    expect(['×', '✓', '▾', '▶', '⌕', '⟳', '⚙', '⧉', '✦', '↑', '■'].filter((g) => !ICON_GLYPH.test(g))).toEqual([]);
    expect(['é', '«', '»', '’', '…', '·', '—', '%', '$', '€'].filter((g) => ICON_GLYPH.test(g))).toEqual([]);
  });

  for (const [lot, files] of Object.entries(REDRAWN))
    it(`are drawn, not written, in ${lot}`, () => {
      expect(files.flatMap(glyphs)).toEqual([]);
    });
});

/** The texts of a zone of a catalog, by their dotted path. */
function texts(tree: unknown, path: string): [string, string][] {
  if (typeof tree === 'string') return [[path, tree]];
  if (!tree || typeof tree !== 'object') return [];
  return Object.entries(tree).flatMap(([k, v]) => texts(v, path ? `${path}.${k}` : k));
}

describe('the texts of the windows (lot 11a, ESC-83)', () => {
  // Their buttons draw their icon: « ✦ Remplir automatiquement » and « + Ajouter une commande » are no more. The arrow
  // between two branches (« escouade/x → main ») is a word of the sentence, as the preview writes it.
  const ZONES = (c: typeof fr | typeof en) => [
    ...texts(c.wizard, 'wizard'),
    ...texts(c.accounts.modal, 'accounts.modal'),
    ...texts(c.prs.modal, 'prs.modal'),
    ...texts(c.shell.update, 'shell.update'),
    ...texts(c.shell.telemetry, 'shell.telemetry'),
  ];
  for (const [lang, catalog] of [
    ['fr', fr],
    ['en', en],
  ] as const)
    it(`draw no icon with a character, in ${lang}`, () => {
      expect(
        ZONES(catalog)
          .filter(([, text]) => ICON_GLYPH.test(text.replace(/ → /g, ' ')) || /^[+] /.test(text))
          .map(([path]) => path),
      ).toEqual([]);
    });
});
