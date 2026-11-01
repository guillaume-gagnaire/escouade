<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router';

// A modal of the store that lives in the address (ESC-318): the page renders it open when its query says so, so that a
// link, a reload and the redirect after a form come back to it, script or not. Without script it is a plain open
// <dialog> over a veil that leads back to `closeTo`; with script it becomes modal (the rest of the page inert), the
// focus goes to its title and stays inside it (Tab and Shift+Tab loop), Escape or a click on the veil leaves for
// `closeTo`, and the focus goes back to `returnTo` (the « ⋯ » that opened it) once it is gone.
const props = defineProps<{
  /** Where closing leads: the same page without the modal's query. */
  closeTo: RouteLocationRaw;
  /** The id of the heading that names the modal (aria-labelledby). */
  labelledby: string;
  /** The words of the close button and of the veil. */
  closeLabel: string;
  /** The selector of what gets the focus back once the modal is closed. */
  returnTo?: string;
}>();

const box = ref<HTMLDialogElement | null>(null);

onMounted(() => {
  const d = box.value;
  if (!d) return;
  // Rendered open by the server (without script, it stays so): opened again as a modal, on top of the page.
  if (d.open) d.close();
  d.showModal();
  d.querySelector<HTMLElement>('[data-dialog-title]')?.focus();
});

onBeforeUnmount(() => {
  const back = props.returnTo;
  if (back) nextTick(() => document.querySelector<HTMLElement>(back)?.focus());
});

function close() {
  navigateTo(props.closeTo);
}

/** What can take the focus inside the modal, in order. */
function focusables(): HTMLElement[] {
  const all = box.value?.querySelectorAll<HTMLElement>(
    'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), summary, [tabindex]:not([tabindex="-1"])',
  );
  return [...(all ?? [])].filter((el) => el.getClientRects().length > 0);
}

function trap(e: KeyboardEvent) {
  if (e.key !== 'Tab') return;
  const list = focusables();
  if (!list.length) return;
  const first = list[0]!;
  const last = list[list.length - 1]!;
  const at = document.activeElement;
  const inside = box.value?.contains(at) ?? false;
  if (e.shiftKey && (at === first || !inside || !list.includes(at as HTMLElement))) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && (at === last || !inside)) {
    e.preventDefault();
    first.focus();
  }
}
</script>

<template>
  <NuxtLink class="veil" :to="closeTo" tabindex="-1" aria-hidden="true" />
  <dialog
    ref="box"
    open
    class="dialog"
    aria-modal="true"
    :aria-labelledby="labelledby"
    @cancel.prevent="close"
    @keydown="trap"
    @click.self="close"
  >
    <div class="sheet">
      <NuxtLink class="btn close" :to="closeTo" :aria-label="closeLabel" :title="closeLabel"
        ><svg class="i" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4L12 12M12 4L4 12" /></svg
      ></NuxtLink>
      <slot />
    </div>
  </dialog>
</template>

<style scoped>
.veil {
  position: fixed;
  z-index: 40;
  inset: 0;
  background: rgba(4, 5, 6, 0.66);
}
@media (prefers-color-scheme: light) {
  .veil {
    background: rgba(15, 17, 22, 0.36);
  }
}
.dialog {
  position: fixed;
  z-index: 41;
  inset: 0;
  width: min(640px, 100% - 32px);
  max-width: none;
  height: fit-content;
  max-height: calc(100% - 64px);
  margin: auto;
  padding: 0;
  overflow: hidden;
  border: 0;
  border-radius: 14px;
  background: var(--bg);
  color: var(--text);
  box-shadow:
    0 40px 100px -20px rgba(0, 0, 0, 0.8),
    0 0 0 1px var(--line-strong);
}
.dialog::backdrop {
  /* The veil is drawn by .veil, the same with or without script. */
  background: transparent;
}
.sheet {
  position: relative;
  display: flex;
  flex-direction: column;
  max-height: calc(100vh - 64px);
  max-height: calc(100dvh - 64px);
}
.close {
  position: absolute;
  z-index: 1;
  top: 18px;
  right: 18px;
  width: 30px;
  height: 30px;
  padding: 0;
  justify-content: center;
  border-color: transparent;
  border-radius: 7px;
  background: none;
  color: var(--muted);
}
.close:hover {
  color: var(--text);
  background: var(--hover);
}
@media (max-width: 720px) {
  /* On a phone, a sheet that rises from the bottom, the page's top still showing above it. */
  .dialog {
    inset: auto 0 0;
    width: 100%;
    max-height: calc(100% - 24px);
    border-radius: 16px 16px 0 0;
  }
  .sheet {
    max-height: calc(100vh - 24px);
    max-height: calc(100dvh - 24px);
  }
  .close {
    top: 12px;
    right: 12px;
  }
}
</style>
