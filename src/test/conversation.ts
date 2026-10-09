// Test utility: the element a conversation lives in. The keys that answer a request only act there.

/** A `<main class="conv">`, to render a card into as Conversation.svelte hosts it. */
export function conversationHost(): HTMLElement {
  const main = document.createElement('main');
  main.className = 'conv';
  document.body.appendChild(main);
  return main;
}
