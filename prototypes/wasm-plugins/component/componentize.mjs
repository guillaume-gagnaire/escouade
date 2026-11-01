// node componentize.mjs <componentize-js dir> <bundle> <prelude> <glue> <wit> <out>
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const [cjs, bundlePath, prelude, glue, wit, out] = process.argv.slice(2);
const { componentize } = await import(pathToFileURL(cjs + '/node_modules/@bytecodealliance/componentize-js/src/componentize.js'));
let body = '';
for (const line of readFileSync(bundlePath, 'utf8').split('\n')) {
  const m = /^import\s+(.+)\s+from\s+["'](node:[a-z]+)["'];?\s*$/.exec(line.trim());
  if (m) body += `const ${m[1]} = __esc.node(${JSON.stringify(m[2])});\n`;
  else body += line.replaceAll('import.meta.url', '__esc.url') + '\n';
}
const source = `import { send } from 'escouade:plugin/host';\n${readFileSync(prelude, 'utf8')}\nfunction __bundle() {\n${body}}\n${readFileSync(glue, 'utf8')}`;
const t = Date.now();
const { component } = await componentize(source, {
  witWorld: readFileSync(wit, 'utf8'),
  worldName: 'plugin',
  disableFeatures: ['http', 'fetch-event', 'random', 'clocks', 'stdio'],
});
writeFileSync(out, component);
console.log(`componentized in ${Date.now() - t} ms, ${component.length} B`);
