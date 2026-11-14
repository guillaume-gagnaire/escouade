// Le client de l'API distante, réduit au faux serveur de l'étude (mobile-study/shared/fake-server). Les types sont
// réécrits en Dart : rien de `src/lib/types.ts` d'Escouade ne se reprend tel quel.
import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';

class WaitingAgent {
  WaitingAgent(this.id, this.name, this.project, this.waiting, this.preview);
  factory WaitingAgent.fromJson(Map<String, dynamic> j) =>
      WaitingAgent(j['id'], j['name'], j['project'], j['waiting'], j['preview']);
  final String id, name, project, preview;
  final String? waiting;
}

/// Un message : les champs de `ConvItem` d'Escouade dont la maquette a besoin.
class MobileItem {
  MobileItem(this.json);
  final Map<String, dynamic> json;
  String get id => json['id'];
  String get kind => json['kind'];
  String get text => json['text'] ?? '';
  String get name => json['name'] ?? '';
  Map<String, dynamic> get input => (json['input'] as Map?)?.cast<String, dynamic>() ?? {};
  String get output => json['output'] ?? '';
  String get diff => json['diff'] ?? '';
}

class Pairing {
  Pairing(this.url, this.key, this.name);
  final String url, key, name;
}

const _defaultServer = String.fromEnvironment('MS_SERVER', defaultValue: 'http://10.0.2.2:8445');
const defaultKey = String.fromEnvironment('MS_KEY');

class Server {
  static Pairing pairing = Pairing(_defaultServer, defaultKey, '');

  static Future<void> load() async {
    final p = await SharedPreferences.getInstance();
    final url = p.getString('ms.url');
    if (url != null) pairing = Pairing(url, p.getString('ms.key') ?? '', p.getString('ms.name') ?? '');
  }

  static Future<void> save(Pairing x) async {
    pairing = x;
    final p = await SharedPreferences.getInstance();
    await p.setString('ms.url', x.url);
    await p.setString('ms.key', x.key);
    await p.setString('ms.name', x.name);
  }

  /// Lit `escouade://pair?u=…&k=…&n=…`.
  static Pairing? parse(String uri) {
    if (!uri.startsWith('escouade://pair?')) return null;
    final q = Uri.splitQueryString(uri.substring('escouade://pair?'.length));
    if (q['u'] == null || q['k'] == null) return null;
    return Pairing(q['u']!, q['k']!, q['n'] ?? '');
  }

  static Future<dynamic> get(String path) async {
    final r = await http.get(Uri.parse(pairing.url + path));
    if (r.statusCode != 200) throw Exception('$path : ${r.statusCode}');
    return jsonDecode(utf8.decode(r.bodyBytes));
  }

  static Future<dynamic> post(String path, Object body) async {
    final r = await http.post(Uri.parse(pairing.url + path),
        headers: {'content-type': 'application/json'}, body: jsonEncode(body));
    return jsonDecode(r.body);
  }

  /// Un repère pour le banc, noté par le faux serveur à l'heure de l'hôte.
  static void mark(String ev, [Object? v]) {
    final q = {'app': 'flutter', 'ev': ev, if (v != null) 'v': '$v'};
    http.get(Uri.parse('${pairing.url}/api/mark').replace(queryParameters: q)).catchError((_) => http.Response('', 0));
  }
}
