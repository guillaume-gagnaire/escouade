// The program Escouade starts (run in escouade-plugin.json): speaks the protocol on stdin and stdout.
import { slack } from './plugin.ts';

void slack().start();
