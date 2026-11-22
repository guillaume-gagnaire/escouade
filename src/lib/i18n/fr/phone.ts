import type { Tree } from '../types';

// Réglages › Téléphone (ESC-567, MO-2 of docs/MOBILE.md; docs/REMOTE.md « Le téléphone »): the phone's access to a core,
// the WireGuard tunnel built into Escouade, the UDP port and the box, the public address, Tailscale, the phones.
export default {
  intro:
    'Ton téléphone joint {where} par un tunnel WireGuard intégré à Escouade : ni VPN ni droit d’administrateur sur la machine, un seul port UDP ouvert, et l’API n’est servie que dans le tunnel, derrière le jeton de chaque appareil.',
  whereLocal: 'ce poste',
  whereServer: 'ce serveur',
  access: {
    title: 'L’accès',
    label: 'Accès depuis le téléphone',
    labelDesc: 'Allumé, le tunnel attend tes téléphones sur le port UDP ci-dessous. Éteint, rien n’écoute.',
  },
  port: {
    label: 'Port UDP',
    labelDesc: 'Celui que la box redirige vers cette machine ; 51820 par défaut, celui de WireGuard.',
    apply: 'Appliquer',
    invalid: 'Un port de 1 à 65535.',
  },
  state: {
    title: 'L’état',
    off: 'Éteint',
    starting: 'Démarrage…',
    on: 'Prêt sur le port UDP {port}',
    error: 'Ne démarre pas',
    api: 'API dans le tunnel : {api}',
    phones: { one: '{count} téléphone', other: '{count} téléphones' },
  },
  box: {
    title: 'La box',
    opened: 'Port UDP {port} ouvert dans la box par {by}, renouvelé tant que l’accès est allumé.',
    todo: 'Dans la box, redirige le port UDP {port} vers {local} (cette machine) : Escouade n’a pas pu l’ouvrir lui-même.',
    todoNoLocal: 'Dans la box, redirige le port UDP {port} vers cette machine : Escouade n’a pas pu l’ouvrir lui-même.',
    why: 'La box a répondu : {error}',
    cgnat:
      'Ton opérateur partage ton adresse IPv4 (CGNAT), ou ta box est derrière une autre : une redirection n’y fera rien depuis Internet.',
    cgnatWays:
      'Les issues : demander une IPv4 dédiée à l’opérateur (souvent gratuit), passer par l’IPv6 de la box, Tailscale, ou un serveur Escouade sur un VPS.',
    ipv6: 'Cette machine a une IPv6 publique ({ip}) : ouvre aussi le port UDP {port} dans le pare-feu IPv6 de la box.',
    server: 'Sur un serveur, pas de box : ouvre le port UDP {port} dans son pare-feu (par exemple : ufw allow {port}/udp).',
    direct: 'Cette machine a l’adresse publique elle-même : ouvre le port UDP {port} dans son pare-feu.',
    unchecked: 'Le réseau n’a pas encore été regardé.',
    checking: 'Regard sur le réseau…',
    probesOff: 'Le réseau n’est pas regardé ici (essais).',
    wanLabel: 'Adresse de la box sur Internet',
  },
  public: {
    label: 'Adresse publique',
    unknown: 'inconnue',
    changed: 'Changée le {date} (avant : {before}) : un téléphone appairé avant elle doit reprendre la nouvelle.',
    checked: 'Vue {when}',
    check: 'Revoir le réseau',
  },
  tailscale: {
    title: 'Tailscale',
    found: 'Tailscale est sur cette machine ({address}) : un téléphone de ton tailnet passe par lui, sans port à ouvrir, CGNAT compris.',
    absent: 'Tailscale n’est pas sur cette machine. S’il y vient, un téléphone de ton tailnet passera par lui, sans port à ouvrir.',
    label: 'Servir aussi sur l’adresse Tailscale',
    labelDesc: 'Pour un téléphone de ton tailnet seulement ; le jeton d’appareil reste demandé.',
    served: 'Servie sur {addr}',
  },
  phones: {
    title: 'Les téléphones',
    none: 'Aucun téléphone appairé. L’app d’Escouade pour Android et iOS les appairera par un QR code.',
    up: 'Tunnel ouvert · dernière poignée de main {ago}',
    idle: 'Tunnel au repos',
    address: 'Adresse dans le tunnel : {address}',
    remove: 'Retirer',
    removed: '{name} retiré et son accès révoqué',
    removedUnlinked: '{name} retiré du tunnel. Il n’était lié à aucun appareil : aucun jeton à révoquer.',
  },
  advanced: {
    key: 'Clé publique du tunnel : {key}',
    rotate: 'Changer la clé du tunnel',
    rotateDesc: 'Chaque téléphone devra être appairé de nouveau : l’ancienne clé n’est plus reconnue.',
    rotateButton: 'Changer la clé',
    rotateTitle: 'Changer la clé du tunnel ?',
    rotateText: 'Les téléphones appairés ne joignent plus {where} tant qu’ils n’ont pas la nouvelle clé.',
    rotated: 'Nouvelle clé du tunnel',
  },
} as const satisfies Tree;
