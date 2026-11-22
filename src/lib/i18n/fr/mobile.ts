import type { Tree } from '../types';

// L'app du téléphone (MO-1, docs/MOBILE.md, direction B « le fil ») : sa coque, ses trois onglets, les puces d'espace,
// le « + » et leurs états vides. Les écrans détaillés viennent avec MO-4, MO-5, MO-7, MO-10 et MO-11.
export default {
  tabs: {
    label: 'Les onglets de l’app',
    inbox: 'À toi',
    agents: 'Agents',
    kanban: 'Kanban',
    inboxWaiting: { one: 'À toi, {count} en attente', other: 'À toi, {count} en attente' },
  },
  create: {
    label: 'Créer',
    title: 'Créer',
    project: 'Nouveau projet',
    projectHint: 'Sur un serveur : un dépôt cloné ou un de ses dossiers',
    agent: 'Nouvel agent',
    agentHint: 'Dans un projet, avec son premier message',
    ticket: 'Nouveau ticket',
    ticketHint: 'Dicté, relu avant de le créer',
    soon: 'Bientôt sur le téléphone',
    close: 'Fermer',
  },
  spaces: {
    label: 'Les espaces',
    everywhere: 'Partout',
    everywhereLabel: { one: 'Partout, {count} en attente', other: 'Partout, {count} en attente' },
    chip: { one: '{name}, {count} en attente', other: '{name}, {count} en attente' },
    chipCut: '{name}, injoignable',
    chipConnecting: '{name}, connexion…',
  },
  band: {
    cut: '{name} injoignable · nouvel essai dans {seconds} s',
    cutNow: '{name} injoignable · nouvel essai…',
    refused: '{name} refuse ce téléphone : {reason}',
    retry: 'Réessayer',
  },
  inbox: {
    stopped: 'Tes agents se sont arrêtés',
    asks: 'Dans ta boîte',
    empty: 'Rien ne t’attend',
    emptyIn: 'Rien ne t’attend sur {name}',
    emptyText: 'Un agent qui s’arrête sur une autorisation ou une question, et chaque demande de la boîte « À toi », s’affichent ici.',
    countsOnly: 'Le détail de {name} ne s’affiche pas ici (stratégie de l’espace) : {waiting} en attente.',
  },
  agents: {
    empty: 'Aucun agent',
    emptyIn: 'Aucun agent sur {name}',
    emptyText: 'Les agents de tes espaces, au travail ou arrêtés, s’affichent ici avec ce qu’ils font.',
  },
  kanban: {
    choose: 'Choisis un projet',
    chooseText: 'Le Kanban d’un projet s’ouvre ici, une colonne à la fois.',
  },
  welcome: {
    title: 'Escouade sur ce téléphone',
    text: 'Appaire ce téléphone à ton poste ou à un serveur Escouade : il te montrera ce qui t’attend sur chacun, et tu y répondras d’un pouce.',
    how: 'L’appairage par un QR code, montré sur le poste ou par un serveur, arrive bientôt.',
  },
  error: {
    deskOnly: '{command} se fait au poste.',
  },
} as const satisfies Tree;
