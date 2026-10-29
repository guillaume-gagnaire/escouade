// The texts the agents and their tools write, cleaned as they come into a conversation, whatever the harness (ESC-290):
// a tool that read a binary file (audio, an archive) gives NUL, control characters, terminal escapes, broken UTF-16,
// which the page's text engine may refuse, its process aborting with the window. What is not text is shown, never drawn
// raw: a control character by its picture (␀, ␛…), a lone half of a UTF-16 pair by �, a terminal's colours left out.

/** A terminal's escape sequences: CSI (colours, cursor), OSC (title, links) and the two-character ones. */
const ANSI = /\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)?|[@-Z\\-_])/g;
/** Control characters other than the tab, the line feed and the carriage return. */
const CONTROL = /[\x00-\x08\x0b\x0c\x0e-\x1a\x1c-\x1f\x7f]/g;
/** A half of a UTF-16 pair without its other half. */
const LONE = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g;
/** What calls for a cleaning: anything else is let through untouched, at the cost of one search. */
const SUSPECT = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\ud800-\udfff]/;

/** The picture of the control character `c` (U+2400 to U+241F, ␡ for DEL). */
const picture = (c: string) => {
  const code = c.charCodeAt(0);
  return String.fromCharCode(code === 0x7f ? 0x2421 : 0x2400 + code);
};

/** `text` without what the page should not draw raw. */
export function cleanText(text: string): string {
  if (!SUSPECT.test(text)) return text;
  return text.replace(ANSI, '').replace(LONE, '�').replace(CONTROL, picture).replace(/\x1b/g, '␛');
}

/** `value` with each of its texts cleaned, however deep: an item of a conversation, a patch of one. */
export function cleanDeep<T>(value: T): T {
  if (typeof value === 'string') return cleanText(value) as T;
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) value[i] = cleanDeep(value[i]);
    return value;
  }
  if (value && typeof value === 'object') {
    const o = value as Record<string, unknown>;
    for (const k of Object.keys(o)) o[k] = cleanDeep(o[k]);
  }
  return value;
}
