// The program (run.js in escouade-plugin.json, once bundled): speaks the protocol to Escouade, which runs it inside the
// app in WASM; under Node too (the bench of escouade-plugin, a test), on stdin and stdout. start() finds which.
import { plugin } from './plugin.ts';

void plugin.start();
