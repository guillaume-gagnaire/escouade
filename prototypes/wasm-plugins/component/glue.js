// The prelude's Javy.IO over the Component Model: the turn's input is the standard input, each line written goes to
// the host's `send` at once.
const __dec2 = new TextDecoder();
const __enc2 = new TextEncoder();
globalThis.Javy = {
  IO: {
    readSync(fd, buf) {
      const b = globalThis.__in;
      if (!b || !b.length) return 0;
      const n = Math.min(b.length, buf.length);
      buf.set(b.subarray(0, n));
      globalThis.__in = b.subarray(n);
      return n;
    },
    writeSync(fd, bytes) {
      if (fd === 1) send(__dec2.decode(bytes));
      return bytes.length;
    },
  },
};
export function turn(input) {
  globalThis.__in = __enc2.encode(input);
  if (!__esc.booted) __esc.boot(__bundle);
  else __esc.pump();
}
