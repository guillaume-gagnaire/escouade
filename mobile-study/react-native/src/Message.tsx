// Un message de la conversation, réécrit en React Native : le Markdown par react-native-marked (le même marked
// qu'Escouade), les blocs de code colorés (react-native-code-highlighter), le diff par `parseUnifiedDiff` et
// `fileLines` d'Escouade (src/lib/diff.ts, repris tels quels), la sortie d'une commande, l'autorisation.
import { memo, type ReactNode } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { Renderer, useMarkdown, type RendererInterface } from 'react-native-marked';
import CodeHighlighter from 'react-native-code-highlighter';
import { atomOneDark } from 'react-syntax-highlighter/dist/esm/styles/hljs';
import { fileLines, mobile, parseUnifiedDiff, t } from './escouade';
import type { MobileItem } from './server';
import { C } from './theme';

class CodeRenderer extends Renderer implements RendererInterface {
  code(text: string, language?: string): ReactNode {
    return (
      <View key={this.getKey()} style={s.codeBlock}>
        <CodeHighlighter hljsStyle={atomOneDark} language={language || 'text'} textStyle={s.mono} scrollViewProps={{ contentContainerStyle: { backgroundColor: C.well } }}>
          {text}
        </CodeHighlighter>
      </View>
    );
  }
}
const renderer = new CodeRenderer();

const mdTheme = { colors: { text: C.textAi, link: C.brand, code: C.well, border: C.line } };

const mdStyles = { text: { color: C.textAi, fontSize: 15, lineHeight: 21 }, codespan: { color: C.pInk, backgroundColor: C.raise } };

/** Le Markdown sans liste à défiler à lui (useMarkdown) : il est déjà dans celle de la conversation. */
function MarkdownText({ text }: { text: string }) {
  const elements = useMarkdown(text, { renderer, theme: mdTheme, styles: mdStyles, colorScheme: 'dark' });
  return <View>{elements}</View>;
}

function Diff({ diff }: { diff: string }) {
  return (
    <View>
      {parseUnifiedDiff(diff).map((file) => (
        <ScrollView horizontal key={file.path}>
          <View>
            {fileLines(file).map((l, i) => (
              <View key={i} style={[s.diffRow, l.kind === 'add' && s.add, l.kind === 'del' && s.del, l.kind === 'meta' && s.meta]}>
                <Text style={[s.mono, s.no]}>{l.kind === 'del' ? l.oldNo : (l.newNo ?? '')}</Text>
                <Text style={[s.mono, { color: l.kind === 'add' ? C.add : l.kind === 'del' ? C.del : C.text }]}>
                  {l.kind === 'add' ? '+ ' : l.kind === 'del' ? '- ' : '  '}
                  {l.text}
                </Text>
              </View>
            ))}
          </View>
        </ScrollView>
      ))}
    </View>
  );
}

export const Message = memo(function Message({ item, onAnswer }: { item: MobileItem; onAnswer?: (d: 'allow' | 'deny') => void }) {
  switch (item.kind) {
    case 'user':
      return <Text style={s.user}>{item.text}</Text>;
    case 'text':
      return <MarkdownText text={item.text} />;
    case 'tool':
      return (
        <View style={s.tool}>
          <Text numberOfLines={1} style={s.toolHead}>
            <Text style={s.toolName}>{item.name} </Text>
            <Text style={s.mono}>{item.name === 'Bash' ? item.input.command : item.input.file_path}</Text>
          </Text>
          {item.name === 'Bash' ? <Text style={[s.mono, s.out]}>{item.output}</Text> : <Diff diff={item.diff ?? ''} />}
        </View>
      );
    case 'permission':
      return (
        <View style={s.perm}>
          <Text style={s.permTitle}>{t('composer.permission.title', { engine: 'Claude Code' })}</Text>
          <Text style={s.permDesc}>{item.input.description}</Text>
          <Text style={s.permLabel}>{mobile.wholeCommand}</Text>
          <Text selectable style={[s.mono, s.permCmd]}>
            {item.input.command}
          </Text>
          {onAnswer && (
            <View style={s.actions}>
              <Pressable style={s.deny} onPress={() => onAnswer('deny')}>
                <Text style={s.btnText}>{t('composer.permission.deny')}</Text>
              </Pressable>
              <Pressable style={s.allow} onPress={() => onAnswer('allow')}>
                <Text style={[s.btnText, { color: '#04110f' }]}>{t('composer.permission.allow')}</Text>
              </Pressable>
            </View>
          )}
        </View>
      );
  }
  return null;
});

const s = StyleSheet.create({
  mono: { fontFamily: 'monospace', fontSize: 12, lineHeight: 18, color: C.text },
  user: { marginLeft: 48, padding: 10, backgroundColor: C.pSoft, borderRadius: 14, color: C.text, fontSize: 15 },
  codeBlock: { marginVertical: 6, borderRadius: 8, overflow: 'hidden', borderWidth: 1, borderColor: C.line },
  tool: { borderWidth: 1, borderColor: C.line, borderRadius: 10, overflow: 'hidden', backgroundColor: C.well },
  toolHead: { padding: 8, borderBottomWidth: 1, borderBottomColor: C.line, color: C.text },
  toolName: { color: C.pInk, fontWeight: '600' },
  out: { padding: 8, color: C.soft },
  diffRow: { flexDirection: 'row', paddingHorizontal: 8 },
  add: { backgroundColor: 'rgba(127,214,160,0.1)' },
  del: { backgroundColor: 'rgba(240,138,128,0.1)' },
  meta: { backgroundColor: 'rgba(240,185,140,0.08)' },
  no: { width: 34, color: C.dim },
  perm: { borderWidth: 1, borderColor: C.waitLine, backgroundColor: C.waitBg, borderRadius: 12, padding: 12 },
  permTitle: { color: C.text, fontWeight: '600', fontSize: 15 },
  permDesc: { color: C.soft, marginTop: 4, marginBottom: 10 },
  permLabel: { color: C.muted, fontSize: 12 },
  permCmd: { backgroundColor: C.well, borderRadius: 8, padding: 10, marginVertical: 6 },
  actions: { flexDirection: 'row', gap: 10 },
  deny: { flex: 1, padding: 12, borderRadius: 10, borderWidth: 1, borderColor: C.line, alignItems: 'center' },
  allow: { flex: 1, padding: 12, borderRadius: 10, backgroundColor: C.brand, alignItems: 'center' },
  btnText: { color: C.text, fontWeight: '600' },
});
