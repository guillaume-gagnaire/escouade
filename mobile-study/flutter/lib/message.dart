// Un message de la conversation, réécrit en Dart : le Markdown (flutter_markdown_plus), les blocs de code colorés
// (highlight), le diff (un parseur de diff unifié minimal, l'équivalent de src/lib/diff.ts), la sortie d'une commande,
// l'autorisation avec la commande entière.
import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:highlight/highlight.dart' show highlight, Node;
import 'package:markdown/markdown.dart' as md;

import 'server.dart';
import 'texts.dart';
import 'theme.dart';

class MessageView extends StatelessWidget {
  const MessageView({super.key, required this.item, this.onAnswer});
  final MobileItem item;
  final void Function(String decision)? onAnswer;

  @override
  Widget build(BuildContext context) {
    switch (item.kind) {
      case 'user':
        return Align(
          alignment: Alignment.centerRight,
          child: Container(
            margin: const EdgeInsets.only(left: 48),
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(color: Esc.pSoft, borderRadius: BorderRadius.circular(14)),
            child: Text(item.text, style: const TextStyle(color: Esc.text, fontSize: 15)),
          ),
        );
      case 'text':
        return MarkdownBody(
          data: item.text,
          styleSheet: Esc.markdown,
          builders: {'code': CodeBlockBuilder()},
        );
      case 'tool':
        return _ToolCard(item: item);
      case 'permission':
        return _PermissionCard(item: item, onAnswer: onAnswer);
    }
    return const SizedBox.shrink();
  }
}

class _ToolCard extends StatelessWidget {
  const _ToolCard({required this.item});
  final MobileItem item;

  @override
  Widget build(BuildContext context) {
    final head = item.name == 'Bash' ? item.input['command'] : item.input['file_path'];
    return Container(
      decoration: BoxDecoration(color: Esc.well, border: Border.all(color: Esc.line), borderRadius: BorderRadius.circular(10)),
      clipBehavior: Clip.antiAlias,
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: Text.rich(
            TextSpan(children: [
              TextSpan(text: '${item.name} ', style: const TextStyle(color: Esc.pInk, fontWeight: FontWeight.w600)),
              TextSpan(text: '$head', style: Esc.mono),
            ]),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        const Divider(height: 1, color: Esc.line),
        if (item.name == 'Bash')
          Padding(padding: const EdgeInsets.all(8), child: Text(item.output, style: Esc.mono.copyWith(color: Esc.soft)))
        else
          DiffBlock(diff: item.diff),
      ]),
    );
  }
}

/// Le diff : une ligne par ligne du diff unifié, ses numéros, ajouts en vert, retraits en rouge.
class DiffBlock extends StatelessWidget {
  const DiffBlock({super.key, required this.diff});
  final String diff;

  @override
  Widget build(BuildContext context) {
    final rows = <Widget>[];
    var oldNo = 0, newNo = 0;
    var inHunk = false;
    for (final line in diff.split('\n')) {
      final h = RegExp(r'^@@ -(\d+)(?:,\d+)? \+(\d+)').firstMatch(line);
      if (h != null) {
        oldNo = int.parse(h.group(1)!);
        newNo = int.parse(h.group(2)!);
        inHunk = true;
        rows.add(Container(color: Esc.hunk, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2), child: Text(line, style: Esc.mono.copyWith(color: Esc.pInk))));
        continue;
      }
      if (!inHunk) continue;
      final kind = line.isEmpty ? ' ' : line[0];
      final n = kind == '+' ? newNo++ : (kind == '-' ? oldNo++ : (oldNo++, newNo++).$2);
      rows.add(Container(
        color: kind == '+' ? Esc.addBg : (kind == '-' ? Esc.delBg : null),
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          SizedBox(width: 34, child: Text('$n', style: Esc.mono.copyWith(color: Esc.dim))),
          Text(kind == ' ' ? ' ' : kind, style: Esc.mono.copyWith(color: kind == '+' ? Esc.add : Esc.del)),
          const SizedBox(width: 6),
          Expanded(child: Text(line.isEmpty ? '' : line.substring(1), style: Esc.mono, softWrap: false, overflow: TextOverflow.clip)),
        ]),
      ));
    }
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: rows);
  }
}

class _PermissionCard extends StatelessWidget {
  const _PermissionCard({required this.item, this.onAnswer});
  final MobileItem item;
  final void Function(String decision)? onAnswer;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: Esc.waitBg, border: Border.all(color: Esc.waitLine), borderRadius: BorderRadius.circular(12)),
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Text(Texts.t('composer.permission.title', {'engine': 'Claude Code'}), style: const TextStyle(color: Esc.text, fontWeight: FontWeight.w600, fontSize: 15)),
        const SizedBox(height: 4),
        Text('${item.input['description']}', style: const TextStyle(color: Esc.soft)),
        const SizedBox(height: 10),
        Text(Texts.m('wholeCommand'), style: const TextStyle(color: Esc.muted, fontSize: 12)),
        Container(
          margin: const EdgeInsets.symmetric(vertical: 6),
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(color: Esc.well, borderRadius: BorderRadius.circular(8)),
          child: SelectableText('${item.input['command']}', style: Esc.mono),
        ),
        if (onAnswer != null)
          Row(children: [
            Expanded(child: OutlinedButton(onPressed: () => onAnswer!('deny'), child: Text(Texts.t('composer.permission.deny')))),
            const SizedBox(width: 10),
            Expanded(
              child: FilledButton(
                style: FilledButton.styleFrom(backgroundColor: Esc.brand, foregroundColor: const Color(0xFF04110F)),
                onPressed: () => onAnswer!('allow'),
                child: Text(Texts.t('composer.permission.allow')),
              ),
            ),
          ]),
      ]),
    );
  }
}

/// Les blocs de code du Markdown, colorés par `highlight` (le pendant de shiki dans Escouade).
class CodeBlockBuilder extends MarkdownElementBuilder {
  @override
  Widget? visitElementAfter(md.Element element, TextStyle? preferredStyle) {
    final cls = element.attributes['class'];
    if (cls == null || !cls.startsWith('language-')) return null; // le code en ligne : le rendu par défaut
    final lang = cls.substring('language-'.length);
    final code = element.textContent.replaceAll(RegExp(r'\n$'), '');
    List<TextSpan> spans;
    try {
      spans = _spans(highlight.parse(code, language: lang == 'ts' ? 'typescript' : lang).nodes ?? []);
    } catch (_) {
      spans = [TextSpan(text: code)];
    }
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(color: Esc.well, borderRadius: BorderRadius.circular(8), border: Border.all(color: Esc.line)),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Text.rich(TextSpan(style: Esc.mono, children: spans)),
      ),
    );
  }

  List<TextSpan> _spans(List<Node> nodes) => [
        for (final n in nodes)
          n.value != null
              ? TextSpan(text: n.value, style: Esc.syntax[n.className])
              : TextSpan(style: Esc.syntax[n.className], children: _spans(n.children ?? [])),
      ];
}
