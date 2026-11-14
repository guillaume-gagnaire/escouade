// La maquette React Native (Expo) de l'app mobile (ESC-445) : la liste des agents qui attendent, la conversation
// longue (FlashList), l'autorisation sous verrou biométrique (expo-local-authentication), l'appairage par QR code
// (expo-camera), le push (expo-notifications : FCM sous Android, APNs sous iOS) et le Rust par uniffi-bindgen-react-native.
import { useCallback, useEffect, useRef, useState } from 'react';
import { Pressable, ScrollView, StatusBar as RNStatusBar, StyleSheet, Text, View, ToastAndroid, Platform } from 'react-native';
import { FlashList } from '@shopify/flash-list';
import { CameraView, useCameraPermissions } from 'expo-camera';
import * as LocalAuthentication from 'expo-local-authentication';
import * as Notifications from 'expo-notifications';
import { StatusBar } from 'expo-status-bar';
import { Message } from './src/Message';
import { mobile, setLang, t } from './src/escouade';
import { benchInRust, coreVersion, open, seal } from './src/rust';
import { getJson, loadPairing, mark, pairing, parsePairUri, postJson, savePairing, type MobileItem, type WaitingAgent } from './src/server';
import { C } from './src/theme';

setLang('fr');
Notifications.setNotificationHandler({
  handleNotification: async () => ({ shouldShowBanner: true, shouldShowList: true, shouldPlaySound: false, shouldSetBadge: false }),
});

const say = (s: string) => (Platform.OS === 'android' ? ToastAndroid.show(s, ToastAndroid.LONG) : console.log(s));

let bridgeResult = '';
/** Le pont : 1 000 ouvertures appelées depuis le JavaScript, et le même travail fait dans le Rust. */
function measureBridge() {
  try {
    const key = pairing().key;
    const sealed = seal(key, 'x'.repeat(256));
    const n = 1000;
    const start = performance.now();
    for (let i = 0; i < n; i++) open(key, sealed);
    const perCall = ((performance.now() - start) * 1000) / n;
    const inRust = benchInRust(n, 256) / n;
    bridgeResult = `${perCall.toFixed(1)} µs par appel (dont ${inRust.toFixed(1)} µs de Rust pour seal + open)`;
    mark('bridge-us', perCall.toFixed(1));
  } catch (e) {
    bridgeResult = `erreur : ${e}`;
  }
}

function openPush(data: Record<string, unknown> | undefined) {
  const sealed = data?.sealed as string | undefined;
  let text = '(sans contenu scellé)';
  if (sealed) {
    try {
      text = open(pairing().key, sealed);
    } catch (e) {
      text = `ne s'ouvre pas : ${e}`;
    }
  }
  say(`${mobile.pushOpened} : ${text}`);
  mark('push-open', sealed ? 'decrypted' : 'plain');
}

type Screen = { name: 'list' } | { name: 'conv'; agent: WaitingAgent } | { name: 'pair' } | { name: 'status' } | { name: 'scan' };

export default function App() {
  const [screen, setScreen] = useState<Screen>({ name: 'list' });
  const [agents, setAgents] = useState<WaitingAgent[]>([]);
  const [error, setError] = useState('');

  useEffect(() => {
    (async () => {
      await loadPairing();
      try {
        setAgents(await getJson<WaitingAgent[]>('/api/agents'));
        requestAnimationFrame(() => mark('ready', 4));
      } catch (e) {
        setError(String(e));
      }
      setTimeout(measureBridge, 1500);
      const last = await Notifications.getLastNotificationResponseAsync();
      if (last) openPush(last.notification.request.content.data as Record<string, unknown>);
    })();
    const sub = Notifications.addNotificationResponseReceivedListener((r) => openPush(r.notification.request.content.data as Record<string, unknown>));
    return () => sub.remove();
  }, []);

  const back = () => setScreen({ name: 'list' });
  const title =
    screen.name === 'conv' ? screen.agent.name : screen.name === 'pair' ? mobile.pairTitle : screen.name === 'status' ? mobile.status : screen.name === 'scan' ? mobile.scan : t('nav.titleBar.waiting');

  return (
    <View style={s.shell}>
      <StatusBar style="light" />
      <View style={s.header}>
        {screen.name !== 'list' && (
          <Pressable onPress={back} accessibilityLabel={mobile.back}>
            <Text style={s.back}>‹</Text>
          </Pressable>
        )}
        <Text numberOfLines={1} style={s.h1}>
          {title}
        </Text>
        {screen.name === 'list' && (
          <>
            <Pressable style={s.chip} onPress={() => setScreen({ name: 'pair' })}>
              <Text style={s.text}>{mobile.pair}</Text>
            </Pressable>
            <Pressable style={s.chip} onPress={() => setScreen({ name: 'status' })}>
              <Text style={s.text}>{mobile.status}</Text>
            </Pressable>
          </>
        )}
      </View>
      {screen.name === 'list' && (
        <ScrollView contentContainerStyle={{ padding: 8 }}>
          {error ? <Text style={{ color: C.del }}>{error}</Text> : null}
          {agents.map((a) => (
            <Pressable key={a.id} style={s.agent} onPress={() => setScreen({ name: 'conv', agent: a })}>
              <View style={[s.dot, { backgroundColor: a.waiting ? C.wait : C.run }]} />
              <View style={{ flex: 1 }}>
                <Text style={s.text}>
                  <Text style={{ fontWeight: '600' }}>{a.name}</Text> <Text style={{ color: C.muted }}>{a.project}</Text>
                </Text>
                <Text style={{ color: C.soft }}>{a.preview}</Text>
              </View>
              <Text style={s.badge}>{a.waiting === 'permission' ? mobile.permission : a.waiting === 'question' ? mobile.question : mobile.noWait}</Text>
            </Pressable>
          ))}
        </ScrollView>
      )}
      {screen.name === 'conv' && <Conversation agent={screen.agent} />}
      {screen.name === 'pair' && <Pair onScan={() => setScreen({ name: 'scan' })} />}
      {screen.name === 'scan' && <Scanner onDone={() => setScreen({ name: 'pair' })} />}
      {screen.name === 'status' && <Status />}
    </View>
  );
}

function Conversation({ agent }: { agent: WaitingAgent }) {
  const [items, setItems] = useState<MobileItem[]>([]);
  useEffect(() => {
    getJson<MobileItem[]>(`/api/agents/${agent.id}/conversation`).then((list) => {
      setItems(list);
      requestAnimationFrame(() => mark('conv-ready', list.length));
    });
  }, [agent.id]);

  const answer = useCallback(
    async (decision: 'allow' | 'deny') => {
      const r = await LocalAuthentication.authenticateAsync({ promptMessage: mobile.lockReason });
      if (!r.success) {
        mark('perm-locked', r.error);
        say(`${mobile.lockFailed} (${r.error})`);
        return;
      }
      await postJson(`/api/agents/${agent.id}/permission`, { decision, app: 'react-native', biometric: 'empreinte' });
      mark('perm-sent', decision);
      say(`${mobile.sent} : ${t(decision === 'allow' ? 'composer.permission.allowed' : 'composer.permission.denied')}`);
    },
    [agent.id],
  );

  return (
    <FlashList
      data={items}
      keyExtractor={(it) => it.id}
      getItemType={(it) => (it.kind === 'tool' ? `tool-${it.name}` : it.kind)}
      maintainVisibleContentPosition={{ startRenderingFromBottom: true, autoscrollToBottomThreshold: 0.2 }}
      contentContainerStyle={{ padding: 12 }}
      renderItem={({ item }) => (
        <View style={{ marginBottom: 10 }}>
          <Message item={item} onAnswer={item.kind === 'permission' ? answer : undefined} />
        </View>
      )}
    />
  );
}

function Pair({ onScan }: { onScan: () => void }) {
  const p = pairing();
  return (
    <View style={s.pane}>
      <Text style={s.text}>{mobile.pairHelp}</Text>
      <Pressable style={s.primary} onPress={onScan}>
        <Text style={s.primaryText}>{mobile.scan}</Text>
      </Pressable>
      <Text style={s.text}>{p.name ? `${mobile.paired} : ${p.name} (${p.url})` : mobile.notPaired}</Text>
      {lastPair ? <Text style={{ color: C.add }}>{lastPair}</Text> : null}
    </View>
  );
}

let lastPair = '';

function Scanner({ onDone }: { onDone: () => void }) {
  const [perm, request] = useCameraPermissions();
  const done = useRef(false);
  useEffect(() => {
    if (perm && !perm.granted) request();
  }, [perm, request]);
  if (!perm?.granted) return <Text style={s.text}>…</Text>;
  return (
    <CameraView
      style={{ flex: 1 }}
      facing="back"
      barcodeScannerSettings={{ barcodeTypes: ['qr'] }}
      onBarcodeScanned={async ({ data }) => {
        if (done.current) return;
        done.current = true;
        const p = parsePairUri(data);
        if (!p) {
          lastPair = `QR inconnu : ${data}`;
          onDone();
          return;
        }
        await savePairing(p);
        // Le serveur scelle un texte avec la clé du QR code : l'ouvrir par le Rust prouve l'appairage et le pont.
        const { sealed } = await getJson<{ sealed: string }>('/api/challenge');
        lastPair = open(p.key, sealed);
        mark('paired', lastPair.length);
        onDone();
      }}
    />
  );
}

function Status() {
  const [push, setPush] = useState('—');
  const register = async () => {
    try {
      const perm = await Notifications.requestPermissionsAsync();
      if (!perm.granted) {
        setPush(`permission ${perm.status}`);
        return;
      }
      const token = await Notifications.getDevicePushTokenAsync();
      setPush(`jeton ${String(token.data).slice(0, 24)}…`);
      mark('push-register', 'token');
    } catch (e) {
      setPush(String(e));
      mark('push-register', String(e).slice(0, 80));
    }
  };
  return (
    <View style={s.pane}>
      <Text style={s.text}>
        {mobile.rust} : {coreVersion()}
      </Text>
      <Text style={s.text}>
        {mobile.bridge} : {bridgeResult}
      </Text>
      <Text style={s.text}>
        {mobile.push} : {push}
      </Text>
      <Text style={s.text}>Serveur : {pairing().url}</Text>
      <Pressable style={s.primary} onPress={register}>
        <Text style={s.primaryText}>{mobile.pushRegister}</Text>
      </Pressable>
    </View>
  );
}

const s = StyleSheet.create({
  // Android 15 dessine sous la barre d'état (bord à bord) : l'en-tête descend de sa hauteur.
  shell: { flex: 1, backgroundColor: C.panel, paddingTop: Platform.OS === 'android' ? (RNStatusBar.currentHeight ?? 24) + 4 : 50 },
  header: { flexDirection: 'row', alignItems: 'center', gap: 8, paddingHorizontal: 14, paddingVertical: 10, backgroundColor: C.frame, borderBottomWidth: 1, borderBottomColor: C.line },
  h1: { flex: 1, color: C.text, fontSize: 17, fontWeight: '600' },
  back: { color: C.text, fontSize: 28, paddingRight: 8 },
  chip: { backgroundColor: C.raise, borderRadius: 999, paddingHorizontal: 12, paddingVertical: 6, borderWidth: 1, borderColor: C.line },
  text: { color: C.text, fontSize: 15, marginBottom: 8 },
  agent: { flexDirection: 'row', alignItems: 'center', gap: 12, padding: 14, marginBottom: 6, backgroundColor: C.raise, borderRadius: 12, borderWidth: 1, borderColor: C.line },
  dot: { width: 10, height: 10, borderRadius: 5 },
  badge: { color: C.wait, fontSize: 12, borderWidth: 1, borderColor: C.waitLine, backgroundColor: C.waitBg, borderRadius: 999, paddingHorizontal: 8, paddingVertical: 2 },
  pane: { padding: 16 },
  primary: { backgroundColor: C.brand, padding: 12, borderRadius: 10, alignItems: 'center', marginVertical: 12 },
  primaryText: { color: '#04110f', fontWeight: '600' },
});
