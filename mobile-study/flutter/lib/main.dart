// La maquette Flutter de l'app mobile (ESC-445) : la liste des agents qui attendent, la conversation longue,
// l'autorisation sous verrou biométrique (local_auth), l'appairage par QR code (mobile_scanner), le push (FCM par
// firebase_messaging, APNs sous iOS) et le Rust commun par flutter_rust_bridge.
import 'dart:async';
import 'dart:io' show Platform;

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:local_auth/local_auth.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import 'message.dart';
import 'server.dart';
import 'src/rust/api/core.dart' as core;
import 'src/rust/frb_generated.dart';
import 'texts.dart';
import 'theme.dart';

/// Un push « données » reçu app fermée : FCM réveille un isolat Dart, qui ouvre le contenu scellé par le Rust.
@pragma('vm:entry-point')
Future<void> onBackgroundPush(RemoteMessage m) async {
  await RustLib.init();
  await Server.load();
  final sealed = m.data['sealed'];
  var r = 'plain';
  if (sealed != null) {
    try {
      core.open(key: Server.pairing.key, sealed: sealed);
      r = 'decrypted';
    } catch (e) {
      r = 'error';
    }
  }
  Server.mark('push-bg', r);
  await Future.delayed(const Duration(milliseconds: 500));
}

bool firebaseReady = false;

const fakeIosOptions = FirebaseOptions(
  apiKey: 'AIzaSyFAKE-esc445-not-a-real-key-000000',
  appId: '1:000000000000:ios:0000000000000445',
  messagingSenderId: '000000000000',
  projectId: 'esc445-factice-sans-projet',
);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await Future.wait([Texts.load(), Server.load()]);
  try {
    // Sous iOS, des options factices (aucun projet Firebase) : le push arrive par APNs, que simctl push simule.
    await Firebase.initializeApp(options: Platform.isIOS ? fakeIosOptions : null);
    FirebaseMessaging.onBackgroundMessage(onBackgroundPush);
    firebaseReady = true;
  } catch (e) {
    debugPrint('Firebase : $e');
  }
  runApp(const MaquetteApp());
}

class MaquetteApp extends StatelessWidget {
  const MaquetteApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: 'Escouade (Flutter)',
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
          brightness: Brightness.dark,
          scaffoldBackgroundColor: Esc.panel,
          colorScheme: const ColorScheme.dark(primary: Esc.brand, surface: Esc.panel),
          appBarTheme: const AppBarTheme(backgroundColor: Esc.frame, foregroundColor: Esc.text),
        ),
        home: const AgentsScreen(),
      );
}

void say(BuildContext context, String s) =>
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(s), duration: const Duration(seconds: 4)));

Future<void> openPush(BuildContext context, RemoteMessage m) async {
  final sealed = m.data['sealed'];
  var text = '(sans contenu scellé)';
  if (sealed != null) {
    try {
      text = core.open(key: Server.pairing.key, sealed: sealed);
    } catch (e) {
      text = "ne s'ouvre pas : $e";
    }
  }
  if (context.mounted) say(context, '${Texts.m('pushOpened')} : $text');
  Server.mark('push-open', sealed != null ? 'decrypted' : 'plain');
}

class AgentsScreen extends StatefulWidget {
  const AgentsScreen({super.key});
  @override
  State<AgentsScreen> createState() => _AgentsScreenState();
}

class _AgentsScreenState extends State<AgentsScreen> {
  List<WaitingAgent> agents = [];
  String error = '';

  @override
  void initState() {
    super.initState();
    _load();
    if (firebaseReady) {
      FirebaseMessaging.instance.getInitialMessage().then((m) {
        if (m != null && mounted) openPush(context, m);
      });
      FirebaseMessaging.onMessageOpenedApp.listen((m) => openPush(context, m));
    }
    Timer(const Duration(milliseconds: 1500), measureBridge);
  }

  Future<void> _load() async {
    try {
      final list = (await Server.get('/api/agents')) as List;
      setState(() => agents = list.map((j) => WaitingAgent.fromJson(j)).toList());
      SchedulerBinding.instance.addPostFrameCallback((_) => Server.mark('ready', agents.length));
    } catch (e) {
      setState(() => error = '$e');
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(Texts.t('nav.titleBar.waiting'), overflow: TextOverflow.ellipsis),
          actions: [
            TextButton(onPressed: () => _push(const PairScreen()), child: Text(Texts.m('pair'))),
            TextButton(onPressed: () => _push(const StatusScreen()), child: Text(Texts.m('status'))),
          ],
        ),
        body: ListView(padding: const EdgeInsets.all(8), children: [
          if (error.isNotEmpty) Text(error, style: const TextStyle(color: Esc.del)),
          for (final a in agents)
            Card(
              color: Esc.raise,
              child: ListTile(
                leading: Icon(Icons.circle, size: 12, color: a.waiting != null ? Esc.wait : Esc.run),
                title: Text.rich(TextSpan(children: [
                  TextSpan(text: a.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                  TextSpan(text: '  ${a.project}', style: const TextStyle(color: Esc.muted, fontSize: 13)),
                ])),
                subtitle: Text(a.preview, style: const TextStyle(color: Esc.soft)),
                trailing: Text(
                  a.waiting == 'permission' ? Texts.m('permission') : a.waiting == 'question' ? Texts.m('question') : Texts.m('noWait'),
                  style: const TextStyle(color: Esc.wait, fontSize: 12),
                ),
                onTap: () => _push(ConversationScreen(agent: a)),
              ),
            ),
        ]),
      );

  void _push(Widget w) => Navigator.of(context).push(MaterialPageRoute(builder: (_) => w));
}

String bridgeResult = '';

/// Le pont : 1 000 ouvertures appelées depuis Dart, et le même travail fait dans le Rust, pour lire le coût du pont.
void measureBridge() {
  try {
    final key = Server.pairing.key;
    final sealed = core.seal(key: key, plain: 'x' * 256);
    const n = 1000;
    final sw = Stopwatch()..start();
    for (var i = 0; i < n; i++) {
      core.open(key: key, sealed: sealed);
    }
    final perCall = sw.elapsedMicroseconds / n;
    final inRust = core.benchInRust(n: n, size: 256).toInt() / n;
    bridgeResult = '${perCall.toStringAsFixed(1)} µs par appel (dont ${inRust.toStringAsFixed(1)} µs de Rust pour seal + open)';
    Server.mark('bridge-us', perCall.toStringAsFixed(1));
  } catch (e) {
    bridgeResult = 'erreur : $e';
  }
}

class ConversationScreen extends StatefulWidget {
  const ConversationScreen({super.key, required this.agent});
  final WaitingAgent agent;
  @override
  State<ConversationScreen> createState() => _ConversationScreenState();
}

class _ConversationScreenState extends State<ConversationScreen> {
  List<MobileItem> items = [];

  @override
  void initState() {
    super.initState();
    Server.get('/api/agents/${widget.agent.id}/conversation').then((list) {
      setState(() => items = (list as List).map((j) => MobileItem(j)).toList());
      SchedulerBinding.instance.addPostFrameCallback((_) => Server.mark('conv-ready', items.length));
    });
  }

  Future<void> answer(String decision) async {
    final auth = LocalAuthentication();
    var detail = 'empreinte';
    var ok = false;
    try {
      ok = await auth.authenticate(localizedReason: Texts.m('lockReason'));
    } catch (e) {
      detail = '$e';
    }
    if (!ok) {
      Server.mark('perm-locked', detail);
      if (mounted) say(context, '${Texts.m('lockFailed')} ($detail)');
      return;
    }
    await Server.post('/api/agents/${widget.agent.id}/permission', {'decision': decision, 'app': 'flutter', 'biometric': detail});
    Server.mark('perm-sent', decision);
    if (mounted) {
      say(context, '${Texts.m('sent')} : ${Texts.t(decision == 'allow' ? 'composer.permission.allowed' : 'composer.permission.denied')}');
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: Text(widget.agent.name)),
        // Une liste paresseuse, à l'envers : le dernier message (l'autorisation) en bas, construit le premier.
        body: ListView.builder(
          reverse: true,
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 24),
          itemCount: items.length,
          itemBuilder: (_, i) {
            final it = items[items.length - 1 - i];
            return Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: MessageView(item: it, onAnswer: it.kind == 'permission' ? answer : null),
            );
          },
        ),
      );
}

class PairScreen extends StatefulWidget {
  const PairScreen({super.key});
  @override
  State<PairScreen> createState() => _PairScreenState();
}

class _PairScreenState extends State<PairScreen> {
  String result = '';

  Future<void> scan() async {
    final text = await Navigator.of(context).push<String>(MaterialPageRoute(builder: (_) => const ScannerScreen()));
    if (text == null) return;
    final p = Server.parse(text);
    if (p == null) {
      setState(() => result = 'QR inconnu : $text');
      return;
    }
    await Server.save(p);
    // Le serveur scelle un texte avec la clé du QR code : l'ouvrir par le Rust prouve l'appairage et le pont.
    final sealed = (await Server.get('/api/challenge'))['sealed'] as String;
    final plain = core.open(key: p.key, sealed: sealed);
    setState(() => result = plain);
    Server.mark('paired', plain.length);
  }

  @override
  Widget build(BuildContext context) {
    final p = Server.pairing;
    return Scaffold(
      appBar: AppBar(title: Text(Texts.m('pairTitle'))),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(Texts.m('pairHelp')),
          const SizedBox(height: 12),
          FilledButton(onPressed: scan, child: Text(Texts.m('scan'))),
          const SizedBox(height: 12),
          Text(p.name.isNotEmpty ? '${Texts.m('paired')} : ${p.name} (${p.url})' : Texts.m('notPaired')),
          if (result.isNotEmpty) Text(result, style: const TextStyle(color: Esc.add)),
        ]),
      ),
    );
  }
}

class ScannerScreen extends StatefulWidget {
  const ScannerScreen({super.key});
  @override
  State<ScannerScreen> createState() => _ScannerScreenState();
}

class _ScannerScreenState extends State<ScannerScreen> {
  final controller = MobileScannerController(formats: const [BarcodeFormat.qrCode]);
  bool done = false;

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: Text(Texts.m('scan'))),
        body: MobileScanner(
          controller: controller,
          onDetect: (capture) {
            final v = capture.barcodes.firstOrNull?.rawValue;
            if (v != null && !done) {
              done = true;
              Navigator.of(context).pop(v);
            }
          },
        ),
      );
}

class StatusScreen extends StatefulWidget {
  const StatusScreen({super.key});
  @override
  State<StatusScreen> createState() => _StatusScreenState();
}

class _StatusScreenState extends State<StatusScreen> {
  String push = '—';

  Future<void> register() async {
    if (!firebaseReady) {
      setState(() => push = 'Firebase non initialisé');
      return;
    }
    try {
      final s = await FirebaseMessaging.instance.requestPermission();
      if (s.authorizationStatus != AuthorizationStatus.authorized) {
        setState(() => push = 'permission ${s.authorizationStatus.name}');
        return;
      }
      final token = await FirebaseMessaging.instance.getToken();
      setState(() => push = token == null ? 'pas de jeton' : 'jeton ${token.substring(0, 24)}…');
      Server.mark('push-register', token == null ? 'none' : 'token');
    } catch (e) {
      setState(() => push = '$e');
      Server.mark('push-register', '$e'.substring(0, '$e'.length.clamp(0, 80)));
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: Text(Texts.m('status'))),
        body: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text('${Texts.m('rust')} : ${core.coreVersion()}'),
            const SizedBox(height: 8),
            Text('${Texts.m('bridge')} : $bridgeResult'),
            const SizedBox(height: 8),
            Text('${Texts.m('push')} : $push'),
            const SizedBox(height: 8),
            Text('Serveur : ${Server.pairing.url}'),
            const SizedBox(height: 12),
            FilledButton(onPressed: register, child: Text(Texts.m('pushRegister'))),
          ]),
        ),
      );
}
