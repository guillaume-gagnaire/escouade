import type { Lang } from './language';

// The texts of the store's MCP server (ESC-317, docs/STORE.md §2.5): the short public page « Gérer ses plugins depuis
// un agent » (/agents, /en/agents), the consent page (/oauth/consent) and « Applications connectées » in the account.

export const MCP_URL = 'https://store.escouade.dev/mcp';
export const MCP_ADD = `claude mcp add --transport http escouade-store ${MCP_URL}`;

export interface ConnectedAppLine {
  ref: number;
  client: string;
  uri: string | null;
  redirectHost: string;
  scopes: string[];
  createdAt: string;
  usedAt: string | null;
  actions: { at: string; tool: string; ok: boolean }[];
}

export interface AgentsTexts {
  title: string;
  lead: string;
  address: string;
  addTitle: string;
  addLead: string;
  firstTitle: string;
  first: string[];
  canTitle: string;
  can: string[];
  adminTitle: string;
  admin: string;
  neverTitle: string;
  never: string[];
  revokeTitle: string;
  revoke: string;
  account: string;
  othersTitle: string;
  others: string;
  more: string;
}

export interface ConsentTexts {
  title: string;
  lead: (client: string) => string;
  account: (login: string) => string;
  from: (host: string) => string;
  words: string;
  scopes: Record<string, { title: string; items: string[] }>;
  refused: string;
  never: string;
  revoke: string;
  allow: string;
  deny: string;
  expired: string;
  signIn: string;
  guest: string;
  back: string;
}

export interface AppsTexts {
  title: string;
  about: string;
  none: string;
  how: string;
  since: (date: string) => string;
  used: (date: string) => string;
  unused: string;
  scopes: Record<string, string>;
  actions: string;
  failed: string;
  revoke: string;
}

const fr: { agents: AgentsTexts; consent: ConsentTexts; apps: AppsTexts } = {
  agents: {
    title: 'Gérer ses plugins depuis un agent',
    lead: 'Le store a un serveur MCP : un agent (Claude Code, ou tout client MCP) y soumet tes versions, suit leurs contrôles, les publie et gère tes plugins, connecté à ton compte par OAuth.',
    address: 'Son adresse',
    addTitle: 'L’ajouter à Claude Code',
    addLead: 'Une commande, puis /mcp dans Claude Code pour te connecter :',
    firstTitle: 'À la première connexion',
    first: [
      'Le navigateur s’ouvre sur le store, puis sur GitHub : la même connexion que sur le site, par l’App « Escouade Store ».',
      'Une page de consentement dit le client (avec le nom qu’il se donne), ton compte et ce qu’il pourra faire ; tu autorises ou tu refuses.',
      'Le client reçoit un jeton d’une heure, valable pour ce serveur seul, et un jeton de rafraîchissement qui change à chaque usage.',
    ],
    canTitle: 'Ce qu’il permet (portée « author »)',
    can: [
      'Lister tes dépôts où l’App est installée, les dossiers à manifeste d’un dépôt et ses étiquettes.',
      'Soumettre une version et suivre ses onze contrôles ; les relancer quand le store ou GitHub a échoué.',
      'Publier : la catégorie, « Demander la revue », la publication seule des nouvelles étiquettes.',
      'Lister tes plugins et leurs versions avec leur état ; retirer une version, retirer un plugin de la liste, changer sa source.',
    ],
    adminTitle: 'Pour les administrateurs (portée « review »)',
    admin:
      'Lire la file de revue, une version et son rapport, et déposer un rapport. Elle n’est donnée qu’aux comptes administrateurs du store, et demandée à part au consentement.',
    neverTitle: 'Ce qu’il ne fait jamais',
    never: [
      'Signer, révoquer, ou toucher à une clé : la signature et la liste de révocation se font sur le poste de l’administrateur, avec la clé qui ne le quitte pas.',
      'Rendre un jeton GitHub, un secret ou une adresse interne ; écrire dans un dépôt ; agir pour un autre compte que le tien.',
      'Faire passer pour une consigne ce qu’a écrit un plugin : un README, un manifeste ou un message rendu par un outil est marqué comme donnée tierce.',
    ],
    revokeTitle: 'Déconnecter un client',
    revoke:
      'Dans ton compte, « Applications connectées » montre chaque client, ses portées, son dernier usage et ses dernières actions ; « Déconnecter » coupe ses jetons aussitôt. Chaque action d’un outil est notée dans ton journal, « par un client MCP ».',
    account: 'Ouvrir mon compte',
    othersTitle: 'Les autres clients MCP',
    others:
      'La même adresse, en Streamable HTTP, avec OAuth 2.1 : le client découvre seul le serveur d’autorisation (RFC 9728), s’enregistre (RFC 7591) et se connecte avec PKCE.',
    more: 'La documentation complète',
  },
  consent: {
    title: 'Autoriser un client MCP',
    lead: (client: string) => `« ${client} » demande à gérer tes plugins sur le store.`,
    account: (login: string) => `Pour le compte ${login}`,
    from: (host: string) => `Il recevra la réponse sur ${host}.`,
    words: 'Ce nom est celui que le client se donne : le store ne le vérifie pas.',
    scopes: {
      author: {
        title: 'Gérer tes plugins',
        items: [
          'lire tes dépôts où l’App est installée, leurs dossiers et leurs étiquettes',
          'soumettre des versions, suivre leurs contrôles et les publier',
          'retirer une version, retirer un plugin de la liste, changer sa source',
        ],
      },
      review: {
        title: 'La file de revue (administrateur)',
        items: ['lire la file de revue, les versions et leurs rapports', 'déposer un rapport de revue'],
      },
    },
    refused: 'La file de revue est demandée, mais ce compte n’administre pas le store : elle ne sera pas donnée.',
    never: 'Il ne pourra ni signer, ni révoquer, ni toucher à une clé, ni écrire dans un dépôt.',
    revoke: 'Tu pourras le déconnecter à tout moment depuis ton compte, « Applications connectées ».',
    allow: 'Autoriser',
    deny: 'Refuser',
    expired: 'Cette demande a expiré ou a déjà reçu sa réponse. Relance la connexion depuis ton agent.',
    signIn: 'Se connecter avec GitHub',
    guest: 'Connecte-toi pour répondre à cette demande.',
    back: 'Gérer ses plugins depuis un agent',
  },
  apps: {
    title: 'Applications connectées',
    about: 'Les clients MCP que tu as autorisés à gérer tes plugins.',
    none: 'Aucun client MCP connecté.',
    how: 'Connecter un agent',
    since: (d: string) => `autorisé le ${d}`,
    used: (d: string) => `dernier usage le ${d}`,
    unused: 'pas encore utilisé',
    scopes: { author: 'plugins', review: 'file de revue' },
    actions: 'Dernières actions, par un client MCP',
    failed: 'refusé',
    revoke: 'Déconnecter',
  },
};

const en: typeof fr = {
  agents: {
    title: 'Manage your plugins from an agent',
    lead: 'The store has an MCP server: an agent (Claude Code, or any MCP client) submits your versions there, follows their checks, publishes them and manages your plugins, connected to your account by OAuth.',
    address: 'Its address',
    addTitle: 'Add it to Claude Code',
    addLead: 'One command, then /mcp in Claude Code to connect:',
    firstTitle: 'The first time it connects',
    first: [
      'The browser opens on the store, then on GitHub: the same sign-in as on the site, through the “Escouade Store” App.',
      'A consent page says the client (with the name it gives itself), your account and what it will be able to do; you allow or refuse.',
      'The client gets a token of one hour, good for this server alone, and a refresh token that changes at each use.',
    ],
    canTitle: 'What it allows (scope “author”)',
    can: [
      'List your repositories where the App is installed, the folders of a repository that hold a manifest, and its tags.',
      'Submit a version and follow its eleven checks; run them again when the store or GitHub failed.',
      'Publish: the category, “Ask for the review”, new tags published on their own.',
      'List your plugins and their versions with their state; yank a version, unlist a plugin, move its source.',
    ],
    adminTitle: 'For administrators (scope “review”)',
    admin:
      'Read the review queue, a version and its report, and post a report. Given only to the store’s administrator accounts, and asked apart on the consent page.',
    neverTitle: 'What it never does',
    never: [
      'Sign, revoke, or touch a key: the signature and the revocation list are made on the administrator’s machine, with the key that never leaves it.',
      'Give a GitHub token, a secret or an internal address; write to a repository; act for another account than yours.',
      'Pass off what a plugin wrote as an instruction: a README, a manifest or a message a tool returns is marked as third-party data.',
    ],
    revokeTitle: 'Disconnect a client',
    revoke:
      'In your account, “Connected apps” shows each client, its scopes, its last use and its last actions; “Disconnect” stops its tokens at once. Each action of a tool is noted in your journal, “by an MCP client”.',
    account: 'Open my account',
    othersTitle: 'Other MCP clients',
    others:
      'The same address, over Streamable HTTP, with OAuth 2.1: the client finds the authorization server on its own (RFC 9728), registers (RFC 7591) and connects with PKCE.',
    more: 'The full documentation',
  },
  consent: {
    title: 'Allow an MCP client',
    lead: (client: string) => `“${client}” asks to manage your plugins on the store.`,
    account: (login: string) => `For the account ${login}`,
    from: (host: string) => `It will get the answer on ${host}.`,
    words: 'This name is the one the client gives itself: the store does not check it.',
    scopes: {
      author: {
        title: 'Manage your plugins',
        items: [
          'read your repositories where the App is installed, their folders and their tags',
          'submit versions, follow their checks and publish them',
          'yank a version, unlist a plugin, move its source',
        ],
      },
      review: {
        title: 'The review queue (administrator)',
        items: ['read the review queue, the versions and their reports', 'post a review report'],
      },
    },
    refused: 'The review queue is asked, but this account does not administer the store: it will not be given.',
    never: 'It will not be able to sign, revoke, touch a key, nor write to a repository.',
    revoke: 'You can disconnect it at any time from your account, “Connected apps”.',
    allow: 'Allow',
    deny: 'Refuse',
    expired: 'This request expired or was already answered. Start the connection again from your agent.',
    signIn: 'Sign in with GitHub',
    guest: 'Sign in to answer this request.',
    back: 'Manage your plugins from an agent',
  },
  apps: {
    title: 'Connected apps',
    about: 'The MCP clients you allowed to manage your plugins.',
    none: 'No MCP client connected.',
    how: 'Connect an agent',
    since: (d: string) => `allowed on ${d}`,
    used: (d: string) => `last used on ${d}`,
    unused: 'not used yet',
    scopes: { author: 'plugins', review: 'review queue' },
    actions: 'Last actions, by an MCP client',
    failed: 'refused',
    revoke: 'Disconnect',
  },
};

export const AGENTS_TEXTS: Record<Lang, typeof fr> = { fr, en };
