// Les jetons du design d'Escouade (src/app.css, thème sombre), recopiés en Dart : rien ne se reprend tel quel.
import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';

class Esc {
  static const frame = Color(0xFF0A0B0C);
  static const panel = Color(0xFF101113);
  static const raise = Color(0xFF17181C);
  static const well = Color(0xFF0B0C0E);
  static const line = Color(0x13FFFFFF);
  static const text = Color(0xFFF7F8F8);
  static const soft = Color(0xFFD0D3D8);
  static const muted = Color(0xFF989DA6);
  static const dim = Color(0xFF8F949E);
  static const textAi = Color(0xFFE9EBEE);
  static const brand = Color(0xFF35C1BD);
  static const pInk = Color(0xFFF0B98C);
  static const pSoft = Color(0x24D9915A);
  static const run = Color(0xFF5DD39A);
  static const wait = Color(0xFFF2C85B);
  static const waitBg = Color(0x12F2C85B);
  static const waitLine = Color(0x52F2C85B);
  static const add = Color(0xFF7FD6A0);
  static const del = Color(0xFFF08A80);
  static const addBg = Color(0x1A7FD6A0);
  static const delBg = Color(0x1AF08A80);
  static const hunk = Color(0x14F0B98C);

  static const mono = TextStyle(fontFamily: 'monospace', fontSize: 12, height: 1.5, color: text);

  static final markdown = MarkdownStyleSheet(
    p: const TextStyle(color: textAi, fontSize: 15, height: 1.45),
    h2: const TextStyle(color: text, fontSize: 18, fontWeight: FontWeight.w600),
    code: const TextStyle(fontFamily: 'monospace', fontSize: 13, color: pInk, backgroundColor: raise),
    listBullet: const TextStyle(color: textAi, fontSize: 15),
    tableBody: const TextStyle(color: textAi, fontSize: 14),
    tableHead: const TextStyle(color: text, fontSize: 14, fontWeight: FontWeight.w600),
    tableBorder: TableBorder.all(color: line),
    blockquoteDecoration: const BoxDecoration(border: Border(left: BorderSide(color: line, width: 3))),
    blockquote: const TextStyle(color: soft, fontSize: 15),
    codeblockDecoration: BoxDecoration(color: well, borderRadius: BorderRadius.circular(8)),
  );

  /// Les couleurs du code (les classes de highlight.js).
  static const syntax = <String?, TextStyle>{
    'keyword': TextStyle(color: Color(0xFFC792EA)),
    'built_in': TextStyle(color: Color(0xFF82AAFF)),
    'type': TextStyle(color: Color(0xFFFFCB6B)),
    'string': TextStyle(color: Color(0xFFC3E88D)),
    'number': TextStyle(color: Color(0xFFF78C6C)),
    'comment': TextStyle(color: Color(0xFF697098), fontStyle: FontStyle.italic),
    'title': TextStyle(color: Color(0xFF82AAFF)),
    'function': TextStyle(color: Color(0xFF82AAFF)),
    'literal': TextStyle(color: Color(0xFFF78C6C)),
    'meta': TextStyle(color: Color(0xFF89DDFF)),
  };
}
