// Les textes : ceux d'Escouade exportés de son catalogue (shared/texts/escouade-texts.json, par export-texts.mjs) et
// ceux du téléphone (shared/texts/mobile.json), les mêmes que les autres maquettes, copiés en assets à la compilation.
import 'dart:convert';

import 'package:flutter/services.dart';

class Texts {
  static Map<String, dynamic> _escouade = {};
  static Map<String, dynamic> _mobile = {};

  static Future<void> load() async {
    _escouade = jsonDecode(await rootBundle.loadString('assets/texts/escouade-texts.json'))['fr'];
    _mobile = jsonDecode(await rootBundle.loadString('assets/texts/mobile.json'));
  }

  /// Un texte du catalogue d'Escouade ; `{engine}` et les autres remplis par `args`.
  static String t(String key, [Map<String, String> args = const {}]) {
    var s = (_escouade[key] ?? key) as String;
    args.forEach((k, v) => s = s.replaceAll('{$k}', v));
    return s;
  }

  static String m(String key) => (_mobile[key] ?? key) as String;
}
