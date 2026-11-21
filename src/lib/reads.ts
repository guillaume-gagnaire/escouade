/**
 * Reads of the same thing asked again and again (the history, the uncommitted files, a diff, read again at each change of
 * the repository), whose answers may come back in any order. An answer is shown when it is newer than the one shown, even
 * if a newer read is still on its way: dropping every answer but the last read asked starved the view on a loaded machine,
 * where the repository changed faster than git answered (« Chargement… » for good, a list never read again, ESC-522). An
 * older answer never replaces a newer one.
 */
export function newestReads() {
  let asked = 0;
  let shown = 0;
  return {
    /** A read is asked: its number. */
    ask(): number {
      return ++asked;
    },
    /** Whether the answer of read `n` is to be shown (newer than the one shown); it is then the one shown. */
    take(n: number): boolean {
      if (n <= shown) return false;
      shown = n;
      return true;
    },
  };
}
