// Metro voit aussi le code d'Escouade (src/lib : diff.ts, le catalogue des textes) et le socle de l'étude
// (mobile-study/shared), repris tels quels ; leurs dépendances (aucune pour ceux-là) viennent du node_modules de l'app.
const path = require('path');
const { getDefaultConfig } = require('expo/metro-config');

const config = getDefaultConfig(__dirname);
const repo = path.resolve(__dirname, '../..');
config.watchFolders = [path.join(repo, 'src/lib'), path.resolve(__dirname, '../shared')];
config.resolver.nodeModulesPaths = [path.resolve(__dirname, 'node_modules')];

module.exports = config;
