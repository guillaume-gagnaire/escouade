#!/usr/bin/env node
// The command of `npm create escouade-plugin` (index.mjs).
import { main } from './index.mjs';

process.exitCode = await main(process.argv.slice(2));
