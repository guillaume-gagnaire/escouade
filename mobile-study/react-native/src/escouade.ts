// Ce que la maquette React Native reprend d'Escouade tel quel : le catalogue des textes (`t()`), le parseur de diff
// et les types de la conversation (la rune `$state` du catalogue : src/rune-shim.ts).

export { t, setLang } from '../../../src/lib/i18n';
export { parseUnifiedDiff, fileLines, type DiffLine } from '../../../src/lib/diff';
export type { UserItem, TextItem, PermissionItem } from '../../../src/lib/types';
import mobileTexts from '../../shared/texts/mobile.json';

export const mobile = mobileTexts;
