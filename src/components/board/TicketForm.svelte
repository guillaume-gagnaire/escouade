<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import { clipboardImage, copyToDisk, planByPath } from '../../lib/attachments';
  import { cycleRefusal, ticketBranch } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { MAX_FILES, draftFiles, filesChanged, keptFile, ticketPlace, type FormFile } from '../../lib/ticket-files';
  import { NotImageError, draftImages, imagesChanged, readTicketImage, shownImage, type FormImage } from '../../lib/ticket-images';
  import { menu } from '../../lib/menu.svelte';
  import { keyLabel, primaryKey } from '../../lib/platform';
  import { ariaEnter } from '../../lib/shortcuts';
  import { app, type TicketFormDraft } from '../../lib/state.svelte';
  import type { EngineId, Ticket, TicketDraft } from '../../lib/types';
  import { choosesEngine, engineName, engineOptions } from '../../lib/engines';
  import { EFFORTS, modelLabel } from '../../lib/models';
  import {
    accountChoices,
    autoAccountLabel,
    autoEffortLabel,
    autoModelLabel,
    effortChoices,
    effortRefusal,
    fitOptions,
    labelOf,
    modelChoices,
    type TicketOptions,
  } from '../../lib/ticket-options';
  import { accountName } from '../../lib/accounts';
  import { insertAt } from '../../lib/dictation';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { fSize } from '../../lib/format';
  import AttachmentChip from '../AttachmentChip.svelte';
  import EngineMark from '../EngineMark.svelte';
  import Icon from '../Icon.svelte';
  import MicButton from '../MicButton.svelte';
  import BranchPicker from '../branches/BranchPicker.svelte';
  import Modal from '../modals/Modal.svelte';
  import { tip } from '../../lib/tip';
  import { authorName, isTheirs, sharedFile } from '../../lib/sharing';
  import Switch from '../settings/Switch.svelte';
  import { DEFAULT_TICKET_PREFIX } from '../../lib/worktrees';

  // The form of a ticket, new or edited, in a large window to write long texts in. `onsubmit` saves the ticket and tells
  // whether it did: the window closes then, else it stays with what was typed. `resume`: what had been typed, back from a
  // dialog that took the window's place (the confirmation of its closing).
  let {
    ticket,
    projectId,
    onsubmit,
    resume,
  }: {
    ticket?: Ticket;
    projectId: string;
    onsubmit: (d: TicketDraft) => boolean | void | Promise<boolean | void>;
    resume?: TicketFormDraft;
  } = $props();

  // What the form opened with, to tell whether leaving it loses anything.
  const start = untrack(() => ({
    title: ticket?.title ?? '',
    description: ticket?.description ?? '',
    criteria: ticket?.criteria.map((c) => c.text).join('\n') ?? '',
    maxLoops: ticket?.maxLoops ?? 5,
    after: ticket?.after ?? [],
    branch: ticket?.branch ?? '',
    engine: ticket?.engine ?? null,
    options: { account: ticket?.account ?? '', model: ticket?.model ?? '', effort: ticket?.effort ?? '' },
    images: ticket?.images.map((i) => i.file) ?? [],
    files: ticket?.files.map((f) => f.file) ?? [],
  }));
  const uid = $props.id();
  /** « Partager avec l'équipe » (ESC-190): a new ticket starts as its project's « Partager chaque nouveau ticket » says. */
  const sharedStart = untrack(() =>
    ticket ? !!ticket.shared && !ticket.shared.gone : !!app.projects.find((p) => p.id === projectId)?.shareNew,
  );
  let shared = $state(sharedStart);
  /** Another one's ticket: not ours to share or not, what changes goes into its file for the team. */
  const theirs = $derived(!!ticket && isTheirs(ticket));

  let title = $state(untrack(() => resume?.title ?? start.title));
  let description = $state(untrack(() => resume?.description ?? start.description));
  let criteria = $state(untrack(() => resume?.criteria ?? start.criteria));
  let maxLoops = $state(untrack(() => resume?.maxLoops ?? start.maxLoops));
  /** The tickets it comes after, in the order they were checked; those done since stay, unlisted (they hold nothing back). */
  let after = $state(untrack(() => [...(resume?.after ?? start.after)]));
  /** The existing branch its agent takes up; empty: the ticket's own, made when it starts. */
  let branch = $state(untrack(() => resume?.branch ?? start.branch));
  /** The harness of its agents (« Harness », from two harnesses found on); null: the project's. */
  let engine = $state<EngineId | null>(untrack(() => (resume?.engine !== undefined ? resume.engine : start.engine)));
  let engineButton = $state<HTMLButtonElement>();
  /** Its account, model and effort (« Compte », « Modèle », « Effort », ESC-153): empty, « Auto ». */
  let options = $state<TicketOptions>(untrack(() => ({ ...(resume?.options ?? start.options) })));
  /** What the last change set back to « Auto », said under the fields until the next one. */
  let notices = $state<string[]>([]);
  let accountButton = $state<HTMLButtonElement>();
  let modelButton = $state<HTMLButtonElement>();
  let effortButton = $state<HTMLButtonElement>();
  let branchButton = $state<HTMLButtonElement>();
  let choosing = $state(false);
  let query = $state('');
  /** Why the last ticket checked was refused (it waits for this one already). */
  let refusal = $state<string | null>(null);
  /**
   * Its images, pasted or dropped in the window, or picked: under the description. Those the ticket has show once read
   * (`onMount`), their place meanwhile.
   */
  let images = $state<FormImage[]>(
    untrack(
      () =>
        resume?.images?.map((i) => ({ ...i })) ??
        (ticket?.images ?? []).map((i) => ({ name: i.name, mediaType: i.mediaType, file: i.file, data: '', url: '', size: 0 })),
    ),
  );
  /**
   * The files of the ticket's images as the window shows them, what leaving it compares `images` to: `start.images` less
   * those the backend could not read (their file is gone), which the window drops by itself. That is no change of the user's.
   */
  let held = $state(untrack(() => [...(resume?.held ?? start.images)]));
  /**
   * Its other files, as chips under the images: those the ticket has, and the copies this form made in the drafts (a chip is
   * there while its copy is written). Never a file where it lies: the agent reads a ticket's files later, when it may be gone.
   */
  let files = $state<FormFile[]>(untrack(() => resume?.files ?? ticket?.files.map(keptFile) ?? []));
  /** What sharing does, in the tooltip of the footer's switch: its file, what goes with it, who commits it. */
  const shareHelp = $derived(
    (ticket?.shared ? t('share.form.help', { file: sharedFile(ticket) ?? '' }) : t('share.form.helpNew')) +
      (files.some((x) => (x.size ?? 0) > 5 * 1024 * 1024) ? ' ' + t('share.form.big') : ''),
  );
  /** Files being read: the ticket waits for them before it is added or saved. */
  let reading = $state(0);
  /** A copy is being written: its chip has no path yet, the ticket waits for it as well. */
  const copying = $derived(files.some((f) => f.copying));
  let dragOver = $state(false);
  let fileInput = $state<HTMLInputElement>();
  let form = $state<HTMLDivElement>();
  let imageList = $state<HTMLUListElement>();
  let fileList = $state<HTMLUListElement>();
  let descriptionField = $state<HTMLTextAreaElement>();

  // Dictated into the description, at its cursor: the shortcut too, while the focus is in the form.
  const dictated: DictationTarget = {
    kind: 'ticket',
    id: `ticket-${Math.random().toString(36).slice(2)}`,
    el: () => form,
    insert: (said) => {
      const f = descriptionField;
      const at = f && document.activeElement === f ? [f.selectionStart, f.selectionEnd] : [description.length, description.length];
      const r = insertAt(description, at[0], at[1], said);
      description = r.text;
      void tick().then(() => {
        descriptionField?.focus();
        descriptionField?.setSelectionRange(r.caret, r.caret);
      });
    },
  };
  let busy = $state(false);
  const ready = $derived(title.trim().length > 0 && !busy && !reading && !copying);
  const changed = $derived(
    title !== start.title ||
      description !== start.description ||
      criteria !== start.criteria ||
      maxLoops !== start.maxLoops ||
      branch !== start.branch ||
      engine !== start.engine ||
      options.account !== start.options.account ||
      options.model !== start.options.model ||
      options.effort !== start.options.effort ||
      after.length !== start.after.length ||
      after.some((id) => !start.after.includes(id)) ||
      imagesChanged(images, held) ||
      filesChanged(files, start.files) ||
      shared !== sharedStart,
  );
  /** What it may come after: the other tickets of the project not done yet, oldest first. */
  const candidates = $derived(
    Object.values(app.tickets)
      .filter((x) => x.projectId === projectId && x.column !== 'done' && x.id !== ticket?.id)
      .sort((a, b) => a.createdAt - b.createdAt),
  );
  /** Those whose key (or, imported, the key there) holds the search; those checked stay in sight. */
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const holds = (key: string | undefined) => !!key && key.toLowerCase().includes(q);
    return candidates.filter((x) => after.includes(x.id) || holds(x.key) || holds(x.external?.key));
  });

  /**
   * The dialog it is. Another one taking its place (the confirmation of its closing, « Quitter Escouade ? ») comes back to
   * it once cancelled: what was typed is kept in it.
   */
  const self = untrack(() => app.modal);
  onDestroy(() => {
    if (self?.kind === 'ticket')
      self.resume = {
        title,
        description,
        criteria,
        maxLoops,
        after: [...after],
        branch,
        engine,
        options: { ...options },
        images: images.map((i) => ({ ...i })),
        held: [...held],
        // Not a copy: the one of a file being written is still this list's, which it takes the chip out of if it fails.
        files,
      };
  });

  /** The ticket's images not read yet. Those whose file is gone are not the ticket's any more: they go from the list. */
  onMount(() => {
    if (!ticket || !images.some((i) => i.file && !i.url)) return;
    api
      .ticketImages(ticket.id)
      .then((read) => {
        for (const r of read) {
          const i = images.find((x) => x.file === r.file);
          if (i) Object.assign(i, shownImage(r));
        }
        // Gone from its folder: the ticket lets go of it once saved, which the window says.
        const lost = images.filter((i) => i.file && !i.url);
        for (const i of lost) app.toast(t('board.form.images.missing', { name: i.name }), 'error');
        images = images.filter((i) => !i.file || i.url);
        // Not a change of the user's, but any save sends the images (`submit`, against `start.images`) to take it off the ticket.
        held = held.filter((file) => !lost.some((i) => i.file === file));
      })
      .catch(failed);
  });

  /**
   * The whole window of the ticket takes them: its fields, its head and foot, and the window itself, which has the focus
   * once a thumbnail or the space between the fields is clicked.
   */
  onMount(() => {
    const dialog = form?.closest<HTMLElement>('[role="dialog"]');
    if (!dialog) return;
    const over = (e: DragEvent) => {
      if (!e.dataTransfer?.types.includes('Files')) return;
      e.preventDefault();
      dragOver = true;
    };
    const leave = () => (dragOver = false);
    dialog.addEventListener('paste', onPaste);
    dialog.addEventListener('dragover', over);
    dialog.addEventListener('dragleave', leave);
    dialog.addEventListener('drop', onDrop);
    return () => {
      dialog.removeEventListener('paste', onPaste);
      dialog.removeEventListener('dragover', over);
      dialog.removeEventListener('dragleave', leave);
      dialog.removeEventListener('drop', onDrop);
    };
  });

  // A file that cannot be added is named in a toast, never left out without a word.
  async function addFiles(source: Iterable<File> | Promise<File[]>) {
    reading++;
    try {
      for (const f of await source) {
        try {
          await addFile(f);
        } catch (e) {
          failed(e);
        }
      }
    } catch (e) {
      failed(e);
    } finally {
      reading--;
    }
  }

  /**
   * An image the chat reads becomes an image of the ticket, with its preview; any other file, or an image that is none or too
   * big, is copied to the drafts.
   */
  async function addFile(f: File) {
    if (ticketPlace(f) === 'image') {
      try {
        images.push(await readTicketImage(f, images));
        return;
      } catch (e) {
        // A « .png » that holds something else is a file like any other; any other refusal is the user's to read.
        if (!(e instanceof NotImageError)) throw e;
      }
    }
    attachFile(f);
  }

  /**
   * Its chip is there at once, while the file is copied. Not waited for: the next files of a drop do not wait for the copy of
   * a big one, and the ticket waits for the chip itself (`copying`).
   */
  function attachFile(f: File) {
    if (files.length >= MAX_FILES) throw new Error(t('board.form.files.tooMany', { name: f.name, max: MAX_FILES }));
    const plan = planByPath(f, app.settings);
    if ('refusal' in plan) throw new Error(plan.refusal);
    const abort = new AbortController();
    const chip = $state<FormFile>({
      kind: 'file',
      name: f.name,
      mediaType: '',
      data: '',
      size: f.size,
      kept: '',
      warn: plan.warn,
      copying: true,
      progress: 0,
      cancel: () => abort.abort(),
    });
    files.push(chip);
    void copyChip(f, chip, abort.signal);
  }

  /** The copy of a file that its chip stands for, until its path is known; a failed one takes its chip away. */
  async function copyChip(f: File, chip: FormFile, signal: AbortSignal) {
    try {
      chip.file = await copyToDisk(f, 'draft', { signal, onProgress: (done) => (chip.progress = done / f.size) });
      chip.copying = false;
      chip.cancel = undefined;
    } catch (e) {
      const i = files.indexOf(chip);
      if (i >= 0) files.splice(i, 1);
      // Taken away by the user: nothing to say.
      if (!signal.aborted) failed(e);
    }
  }

  /**
   * Its chip goes with it, as an image's thumbnail does (the focus moves on in the same way). A copy still being written stops;
   * one finished is erased. A file the ticket has is not: it goes from the ticket when the ticket is saved, not before.
   */
  async function removeFile(i: number) {
    erase(files.splice(i, 1)[0]);
    await tick();
    const crosses = fileList?.querySelectorAll<HTMLButtonElement>('.rm') ?? [];
    (crosses[Math.min(i, crosses.length - 1)] ?? descriptionField)?.focus();
  }

  /** A copy this form is writing stops, one it made is erased; a file the ticket has is not touched (it has no `file`). */
  function erase(f: FormFile) {
    if (f.copying) f.cancel?.();
    else if (f.file) api.attachDiscard('draft', f.file.path).catch(() => {});
  }

  /** Giving the ticket up erases the copies this form made, which no ticket will ever have; never the files the ticket had. */
  function discardCopies() {
    files.forEach(erase);
  }

  /**
   * Its cross goes with it: the focus moves to the cross of the next image, else of the one before, else to the description.
   * It stays in the form, where Ctrl+Enter adds the ticket.
   */
  async function removeImage(i: number) {
    images.splice(i, 1);
    await tick();
    const crosses = imageList?.querySelectorAll<HTMLButtonElement>('.rm') ?? [];
    (crosses[Math.min(i, crosses.length - 1)] ?? descriptionField)?.focus();
  }

  function failed(e: unknown) {
    app.toast(e instanceof Error ? e.message : String(e), 'error');
  }

  /**
   * The files pasted are attached to the ticket. Text stays text, even with an image beside it: the cells of a spreadsheet
   * come with an image of themselves.
   */
  function onPaste(e: ClipboardEvent) {
    if (e.clipboardData?.types.includes('text/plain')) return;
    const list = [...(e.clipboardData?.files ?? [])];
    if (list.length) {
      e.preventDefault();
      addFiles(list);
    } else {
      addFiles(clipboardImage().then((f) => (f ? [f] : [])));
    }
  }

  function onDrop(e: DragEvent) {
    dragOver = false;
    const list = [...(e.dataTransfer?.files ?? [])];
    if (list.length) {
      // Left to the WebView, a dropped file would replace the app.
      e.preventDefault();
      addFiles(list);
    }
  }

  /** What the ticket does when there is no branch to take up: its own, named after its key once it has one. */
  const prefix = $derived(app.projects.find((p) => p.id === projectId)?.ticketBranchPrefix ?? DEFAULT_TICKET_PREFIX);
  const own = $derived(
    ticket ? t('branches.ticket.own', { branch: ticketBranch(ticket.key, prefix) }) : t('branches.ticket.ownNew', { prefix }),
  );

  /** « Celui du projet (…) », then each harness found. */
  const projectEngine = $derived(app.projects.find((p) => p.id === projectId)?.engine ?? 'claude');
  /** Its harness, and why it cannot be used any more when it cannot (ESC-391: « Codex (plugin coupé) »). */
  const engineLabel = $derived(
    engine
      ? (engineOptions(engine).find((o) => o.value === engine)?.label ?? engineName(engine))
      : t('board.form.engineProject', { engine: engineName(projectEngine) }),
  );

  function engineMenu() {
    if (!engineButton) return;
    menu.showAt(engineButton, [
      {
        label: t('board.form.engineProject', { engine: engineName(projectEngine) }),
        engine: projectEngine,
        checked: engine === null,
        onClick: () => setEngine(null),
      },
      ...engineOptions(engine).map((e) => ({
        label: e.label,
        engine: e.value,
        checked: engine === e.value,
        onClick: () => setEngine(e.value),
      })),
    ]);
  }

  /** Another harness: its accounts, models and efforts are others, what it has not goes back to « Auto ». */
  function setEngine(e: EngineId | null) {
    engine = e;
    choose({});
  }

  /** The ticket's harness and project, as the menus of its account, model and effort read them. */
  const optionsOf = $derived({
    project: app.projects.find((p) => p.id === projectId) ?? app.projects[0],
    engine: engine ?? projectEngine,
  });
  const accounts = $derived(accountChoices(optionsOf));
  const models = $derived(modelChoices(optionsOf, options.account));
  const efforts = $derived(effortChoices(optionsOf, options));
  /** Why « Effort » is greyed out: the model its agent would run takes none. */
  const noEffort = $derived(effortRefusal(optionsOf, options));
  const accountShown = $derived.by(() => {
    if (!options.account) return autoAccountLabel(optionsOf);
    const a = (app.settings.accounts ?? []).find((x) => x.id === options.account);
    return a ? accountName(a) : options.account;
  });
  const modelShown = $derived(
    options.model ? labelOf(models, options.model, modelLabel(options.model, app.models)) : autoModelLabel(optionsOf, options.account),
  );
  const effortShown = $derived(
    options.effort
      ? (EFFORTS.find((e) => e.value === options.effort)?.label ?? options.effort)
      : autoEffortLabel(optionsOf, options.account),
  );

  /** One of the three changed: what the others can no longer be goes back to « Auto », which the form says. */
  function choose(patch: Partial<TicketOptions>) {
    const fitted = fitOptions(optionsOf, { ...options, ...patch });
    options = fitted.options;
    notices = fitted.notices;
  }

  function accountMenu() {
    if (!accountButton) return;
    menu.showAt(accountButton, [
      { label: autoAccountLabel(optionsOf), title: t('board.form.options.autoAccountDetail'), onClick: () => choose({ account: '' }) },
      ...accounts.map((a) => ({
        label: a.label,
        title: t('board.form.options.pinnedDetail'),
        onClick: () => choose({ account: a.value }),
      })),
    ]);
  }

  function modelMenu() {
    if (!modelButton) return;
    menu.showAt(modelButton, [
      { label: autoModelLabel(optionsOf, options.account), onClick: () => choose({ model: '' }) },
      ...models.map((m) => ({ label: m.label, onClick: () => choose({ model: m.value }) })),
    ]);
  }

  function effortMenu() {
    if (!effortButton) return;
    menu.showAt(effortButton, [
      { label: autoEffortLabel(optionsOf, options.account), onClick: () => choose({ effort: '' }) },
      ...efforts.map((e) => ({ label: e.label, onClick: () => choose({ effort: e.value }) })),
    ]);
  }

  function branchMenu() {
    if (!branchButton) return;
    menu.showAt(branchButton, [
      { label: own, onClick: () => (branch = '') },
      {
        label: t('branches.ticket.existing'),
        // The branch is the user's: validating the ticket never deletes it.
        title: t('branches.ticket.existingKept'),
        // Once the menu has given the focus back to its button: the picker gives it back there in turn.
        onClick: async () => {
          await tick();
          choosing = true;
        },
      },
    ]);
  }

  /** « Après », the menu's button of the tickets it comes after, and its popover (the list and its search). */
  let afterButton = $state<HTMLButtonElement>();
  let afterPop = $state<HTMLDivElement>();
  let afterOpen = $state(false);
  /**
   * Where the popover opens, fixed to the window (the window's body would cut it): under its button, or over it when the
   * room under it is short.
   */
  let popAt = $state<{ left: number; top: number | null; bottom: number | null; width: number }>({
    left: 0,
    top: 0,
    bottom: null,
    width: 280,
  });
  /** What the button says: the keys of the tickets checked that are not done yet, else « Aucun ». */
  const afterShown = $derived(
    candidates
      .filter((x) => after.includes(x.id))
      .map((x) => x.key)
      .join(', ') || t('board.form.afterNone'),
  );

  function openAfter() {
    if (!afterButton) return;
    const r = afterButton.getBoundingClientRect();
    const width = Math.max(r.width, 280);
    const left = Math.min(r.left, window.innerWidth - width - 8);
    popAt =
      r.bottom + 240 > window.innerHeight
        ? { left, top: null, bottom: window.innerHeight - r.top + 4, width }
        : { left, top: r.bottom + 4, bottom: null, width };
    afterOpen = true;
  }

  /** Closed, the focus goes back to its button (`back`), but when a click elsewhere closed it. */
  function closeAfter(back = true) {
    afterOpen = false;
    query = '';
    if (back) afterButton?.focus();
  }

  /** Escape closes the popover alone, not the window under it. */
  function afterKey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    closeAfter();
  }

  // A click outside the popover and its button closes it.
  $effect(() => {
    if (!afterOpen) return;
    const away = (e: PointerEvent) => {
      const target = e.target as Node;
      if (!afterPop?.contains(target) && !afterButton?.contains(target)) closeAfter(false);
    };
    window.addEventListener('pointerdown', away, true);
    return () => window.removeEventListener('pointerdown', away, true);
  });

  /** A ticket checked that already waits for this one is refused: neither would ever start. */
  function toggle(x: Ticket, box: HTMLInputElement) {
    refusal = null;
    if (!box.checked) {
      after = after.filter((id) => id !== x.id);
      return;
    }
    const refused = ticket ? cycleRefusal(ticket, x, app.tickets) : null;
    if (refused) {
      refusal = refused;
      box.checked = false;
      return;
    }
    after = [...after, x.id];
  }

  /**
   * Leaves the window (Escape, its cross, a click outside, « Annuler »); what was typed is only given up once the user
   * agrees, « Annuler » there coming back to the form. Not while the ticket is saved: a refusal would have nowhere to show.
   */
  function leave() {
    if (busy) return;
    if (!changed) {
      app.modal = null;
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.form.discardTitle'),
      body: t('board.form.discardBody'),
      confirm: t('board.form.discard'),
      danger: true,
      // Given up for good: the copies this form made are of no use to anyone. Cancelled, the form comes back with them.
      onConfirm: discardCopies,
      onCancel: () => (app.modal = self),
    };
  }

  async function submit() {
    if (!ready) return;
    busy = true;
    // Replaced while it saves (« Quitter Escouade ? »), the window does not come back to save twice; refused, it does.
    app.markAnswered(self);
    let saved: boolean | void = false;
    try {
      saved = await onsubmit({
        title: title.trim(),
        description: description.trim(),
        criteria: criteria
          .split('\n')
          .map((l) => l.trim())
          .filter(Boolean),
        maxLoops,
        after: [...after],
        // Said only when it matters: a branch taken up, or the one of an edited ticket given up (empty).
        ...(branch || start.branch ? { branch } : {}),
        // Its harness, null for the project's; its account, model and effort, empty for « Auto ».
        engine,
        account: options.account,
        model: options.model,
        effort: options.effort,
        // Only when they changed from what the ticket had (not `held`: one gone from its folder is taken off at any save);
        // those of an edited ticket may not be read yet.
        ...(imagesChanged(images, start.images) ? { images: draftImages(images) } : {}),
        // The same for the files: the whole list, those kept by their file and the copies by their path.
        ...(filesChanged(files, start.files) ? { files: draftFiles(files) } : {}),
        // Shared or no longer (ESC-190): a new ticket says it always, an edited one when it changed.
        ...(!theirs && (!ticket || shared !== sharedStart) ? { shared } : {}),
      });
    } finally {
      busy = false;
    }
    if (!saved) app.markAnswered(self, false);
    // Not a dialog that took its place meanwhile.
    else if (app.modal === self) app.modal = null;
  }

  /**
   * Ctrl+Enter (Cmd+Enter on macOS) adds the ticket from any field; Enter alone only from the title, the textareas take it as
   * a new line. Not a key held down: its repeats would go on to the next window, or write lines.
   */
  function chord(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing || !primaryKey(e) || e.shiftKey || e.altKey) return;
    e.preventDefault();
    if (!e.repeat) void submit();
  }

  /**
   * The height of the window of the app: a short one gets a row less in the criteria and a tighter form, for the rest of it
   * (its account, model and effort included) to show. The description takes the room left (its style).
   */
  let viewHeight = $state(window.innerHeight);
  const short = $derived(viewHeight < 820);

  /** What the foot says around the key that adds the ticket, drawn in a key. */
  const hint = $derived(t(ticket ? 'board.form.saveHint' : 'board.form.addHint', { key: '\u0001' }).split('\u0001'));

  /** Hands a menu's button, made by the snippet `select`, to the variable its menu opens from. */
  function ref(node: HTMLButtonElement, set: (el: HTMLButtonElement) => void) {
    set(node);
  }
</script>

<svelte:window bind:innerHeight={viewHeight} />

{#snippet select(
  id: string,
  label: string,
  value: string,
  button: (el: HTMLButtonElement) => void,
  onclick: () => void,
  extra: { mark?: EngineId; more?: string; disabled?: string | null; describedby?: string } = {},
)}
  <div class="fld" role="group" aria-label={label}>
    <span class="lb" id="{uid}-{id}">{label}</span>
    <button
      class="select pick"
      use:ref={button}
      aria-haspopup="menu"
      aria-labelledby="{uid}-{id} {uid}-{id}-value"
      aria-describedby={extra.describedby}
      disabled={!!extra.disabled}
      use:tip={extra.disabled ?? (extra.more ? `${value} ${extra.more}` : value)}
      {onclick}
    >
      {#if extra.mark}<EngineMark engine={extra.mark} always />{/if}<b id="{uid}-{id}-value"
        >{value}{#if extra.more}<span class="sr">{' '}{extra.more}</span>{/if}</b
      ><Icon name="down" size={14} />
    </button>
  </div>
{/snippet}

<Modal title={ticket ? t('board.form.editTitle', { key: ticket.key }) : t('board.form.newTitle')} width={760} tall onclose={leave}>
  <div class="form" class:short class:drag={dragOver} role="presentation" bind:this={form} onkeydown={chord}>
    <div class="fld">
      <label for="{uid}-title">{t('board.form.title')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="{uid}-title"
        class="field title"
        bind:value={title}
        autofocus
        onkeydown={(e) => e.key === 'Enter' && !e.isComposing && !e.repeat && !primaryKey(e) && submit()}
      />
    </div>
    <!-- The description takes what the window leaves free: a large field to write in, whatever its height; a long text
         scrolls in it, never over the fields under it (ESC-189). -->
    <div class="fld desc">
      <div class="lab-row">
        <label for="{uid}-description">{t('board.form.description')}</label>
        <MicButton target={dictated} below />
      </div>
      <textarea
        bind:this={descriptionField}
        id="{uid}-description"
        class="field description"
        placeholder={t('board.form.descriptionPlaceholder')}
        bind:value={description}
      ></textarea>
      <input
        bind:this={fileInput}
        type="file"
        multiple
        hidden
        aria-label={t('board.form.attach')}
        onchange={(e) => {
          // Copied first: the list is the input's own, emptied with its value below, before `addFiles` reads it.
          addFiles([...(e.currentTarget.files ?? [])]);
          e.currentTarget.value = '';
        }}
      />
      <!-- Its images and its files in one row of chips, as the message's, and what attaches one at its end. -->
      <div class="atts">
        {#if images.length}
          <ul class="imgs" bind:this={imageList} aria-label={t('board.form.images.label')}>
            {#each images as img, i (i)}
              {@const label =
                images.length > 1 ? t('board.form.images.ranked', { name: img.name, n: i + 1, total: images.length }) : img.name}
              <!-- Screenshots are all named image.png: with several images, each has its rank in its label. -->
              <li class="att img" use:tip={{ text: label, cut: true }}>
                {#if img.url}
                  <img class="th" src={img.url} alt={label} />
                {:else}
                  <span class="th wait" role="img" aria-label={t('board.form.images.loading', { name: label })}></span>
                {/if}
                <span class="nm2"
                  ><b>{img.name}</b><span
                    >{img.size ? t('board.form.images.kindSize', { size: fSize(img.size) }) : t('board.form.images.kind')}</span
                  ></span
                >
                <button class="x rm" aria-label={t('board.form.images.remove', { name: label })} onclick={() => removeImage(i)}
                  ><Icon name="close" size={14} /></button
                >
              </li>
            {/each}
          </ul>
        {/if}
        {#if files.length}
          <ul class="files" bind:this={fileList} aria-label={t('board.form.files.label')}>
            {#each files as f, i (f)}
              <li><AttachmentChip attachment={f} onremove={() => removeFile(i)} /></li>
            {/each}
          </ul>
        {/if}
        <button class="btn attach" use:tip={t('board.form.attachTitle')} onclick={() => fileInput?.click()}
          ><Icon name="clip" size={14} />{t('board.form.attach')}</button
        >
      </div>
      {#if !short}<span class="help">{t('board.form.attachHelp')}</span>{/if}
    </div>
    <div class="fld">
      <!-- The loops beside the criteria: how many rounds its agent gets to meet them. -->
      <div class="lab-row">
        <label for="{uid}-criteria">{t('board.form.criteria')}</label>
        <div class="loops-row">
          <span class="lb" id="{uid}-loops">{t('board.form.maxLoops')}</span>
          <div class="segmented small loops" role="group" aria-labelledby="{uid}-loops">
            {#each [3, 5, 8] as n (n)}
              <button aria-pressed={maxLoops === n} onclick={() => (maxLoops = n)}>{n}</button>
            {/each}
          </div>
        </div>
      </div>
      <textarea
        id="{uid}-criteria"
        class="field"
        rows={short ? 5 : 6}
        placeholder={t('board.form.criteriaPlaceholder')}
        bind:value={criteria}
      ></textarea>
    </div>
    <div class="fgrid">
      {#if choosesEngine(engine)}
        <div class="engine">
          <!-- The project's: its brand says which, the rest of its name for a screen reader. -->
          {@render select(
            'engine',
            t('board.form.engine'),
            engine ? engineLabel : t('board.form.engineOwn'),
            (el) => (engineButton = el),
            engineMenu,
            { mark: engine ?? projectEngine, more: engine ? undefined : `(${engineName(projectEngine)})` },
          )}
          {#if !short}<span class="help">{t('board.form.engineHelp', { engine: engineName(projectEngine) })}</span>{/if}
        </div>
      {/if}
      <div class="fld" role="group" aria-label={t('branches.ticket.branch')}>
        <span class="lb" id="{uid}-branch">{t('branches.ticket.branch')}</span>
        <button
          class="select pick"
          bind:this={branchButton}
          aria-haspopup="menu"
          aria-labelledby="{uid}-branch {uid}-branch-value"
          use:tip={branch || own}
          onclick={branchMenu}
        >
          <Icon name="branch" size={14} /><b class="mono" id="{uid}-branch-value">{branch || own}</b><Icon name="down" size={14} />
        </button>
      </div>
      {#if candidates.length}
        <!-- « Après », a menu's button as in the preview: the tickets checked, the list and its search in its popover. -->
        <div class="fld after" role="group" aria-label={t('board.form.after')}>
          <span class="lb" id="{uid}-after">{t('board.form.after')}</span>
          <button
            class="select pick"
            bind:this={afterButton}
            aria-haspopup="dialog"
            aria-expanded={afterOpen}
            aria-labelledby="{uid}-after {uid}-after-value"
            use:tip={afterShown}
            onclick={() => (afterOpen ? closeAfter() : openAfter())}
          >
            <b id="{uid}-after-value">{afterShown}</b><Icon name="down" size={14} />
          </button>
          {#if afterOpen}
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <div
              class="pop"
              bind:this={afterPop}
              role="dialog"
              tabindex="-1"
              aria-label={t('board.form.after')}
              style:left="{popAt.left}px"
              style:top={popAt.top === null ? undefined : `${popAt.top}px`}
              style:bottom={popAt.bottom === null ? undefined : `${popAt.bottom}px`}
              style:width="{popAt.width}px"
              onkeydown={afterKey}
            >
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="field search"
                role="searchbox"
                placeholder={t('board.form.searchKey')}
                aria-label={t('board.form.searchKey')}
                spellcheck="false"
                autocomplete="off"
                autofocus
                bind:value={query}
              />
              {#if shown.length}
                <ul class="deps">
                  {#each shown as x (x.id)}
                    <li>
                      <label
                        ><input type="checkbox" checked={after.includes(x.id)} onchange={(e) => toggle(x, e.currentTarget)} /><span
                          class="key">{x.key}</span
                        > <span class="name" use:tip={{ text: x.title, cut: true }}>{x.title}</span></label
                      >
                    </li>
                  {/each}
                </ul>
              {:else}
                <span class="help">{t('board.form.noMatch')}</span>
              {/if}
              {#if refusal}<span class="refusal" role="alert">{refusal}</span>{/if}
            </div>
          {/if}
        </div>
      {/if}
      <!-- The account, the model and the effort of its agents (ESC-153), on « Auto » unless set. -->
      {@render select('account', t('board.form.options.account'), accountShown, (el) => (accountButton = el), accountMenu)}
      {@render select('model', t('board.form.options.model'), modelShown, (el) => (modelButton = el), modelMenu)}
      <!-- Greyed out with its reason when the model its agent would run takes no effort. -->
      {@render select(
        'effort',
        t('board.form.options.effort'),
        noEffort ? t('board.form.options.auto') : effortShown,
        (el) => (effortButton = el),
        effortMenu,
        { disabled: noEffort, describedby: noEffort ? `${uid}-effort-why` : undefined },
      )}
      {#if noEffort}<span class="sr" id="{uid}-effort-why">{noEffort}</span>{/if}
      {#each notices as n (n)}<span class="notice" role="status">{n}</span>{/each}
    </div>
  </div>
  {#snippet footer()}
    <!-- Shared with the team (ESC-190, choice 115), in the footer: the body keeps all its height for the description. What
         its file is, what goes with it and who commits it, in its tooltip. -->
    {#if theirs && ticket?.shared}
      <span class="share-foot" use:tip={t('share.form.theirs', { name: authorName(ticket) })}
        ><Icon name="team" size={14} /><span class="sr">{t('share.form.theirs', { name: authorName(ticket) })}</span><span
          aria-hidden="true">{t('share.card.by', { name: authorName(ticket) })}</span
        ></span
      >
    {:else}
      <span class="share-foot" use:tip={shareHelp}
        ><Switch label={t('share.form.label')} bind:on={shared} describedby="{uid}-share" /><span aria-hidden="true"
          >{t('share.form.label')}</span
        ><span class="sr" id="{uid}-share">{shareHelp}</span></span
      >
    {/if}
    <span class="hint">{hint[0]}<kbd>{keyLabel('Ctrl+Enter')}</kbd>{hint[1]}</span>
    <button class="btn" onclick={leave}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!ready} aria-keyshortcuts={ariaEnter()} onclick={submit}
      >{ticket ? t('common.save') : t('common.add')}</button
    >
  {/snippet}
</Modal>

{#if choosing}
  <BranchPicker over {projectId} anchor={branchButton} mode="pick" onclose={() => (choosing = false)} onpick={(b) => (branch = b.name)} />
{/if}

<style>
  /* The whole body of the window: a file is dropped anywhere in it, which the project's colour around it says. */
  .form {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: 0;
    min-height: 0;
    border-radius: 10px;
  }
  .form > * {
    flex: none;
  }
  .form.drag {
    outline: 1px solid color-mix(in oklch, var(--p) 60%, transparent);
    outline-offset: 8px;
  }
  /* A field: its label over it, as the preview's form (`#ticket`). */
  .fld {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
  }
  .fld label,
  .lb {
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .field {
    width: 100%;
    font-family: inherit;
  }
  .title {
    height: 30px;
  }
  textarea.field {
    resize: vertical;
    line-height: 1.5;
  }
  /*
   * The description's field takes the height the window leaves, whatever its text, down to five lines; past what it
   * holds, it scrolls in itself. Its block never gets smaller than its label, its five lines and its attachments: lower,
   * the window's body scrolls, and no field under it is drawn over it (ESC-189).
   */
  .form > .desc {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
  }
  .description {
    flex: 1 1 0;
    min-height: 116px;
    resize: none;
  }
  /* The description's label, and on its right the microphone that dictates into it. */
  .lab-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 18px;
  }
  .lab-row :global(.mic) {
    width: 24px;
    height: 22px;
  }
  .help {
    color: var(--muted);
    font-size: 11.5px;
  }
  /* The images, the files, and « Joindre un fichier », in one row of chips. */
  .atts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding-top: 2px;
  }
  .imgs,
  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .att {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    height: 40px;
    padding: 0 6px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
  }
  .att .th {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    object-fit: cover;
  }
  .att .th.wait {
    background: rgba(var(--fg-rgb), 0.06);
  }
  .nm2 {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .nm2 b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .nm2 span {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .attach {
    height: 40px;
  }
  /* Harness, loops, branch; account, model, effort: three a row. */
  .fgrid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  .engine {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
  }
  /* A menu's button, as « Harness » in the preview: its value, then the chevron at its end. */
  .select {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-width: 0;
    height: 32px;
    padding: 0 9px 0 11px;
    border: none;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: box-shadow 0.15s;
  }
  .select:hover:not(:disabled) {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .select:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .select b {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 550;
  }
  .select b.mono {
    font-size: 12px;
    font-weight: 500;
  }
  .select > :global(.icon:last-child) {
    margin-left: auto;
  }
  /* « Boucles max » at the right of the criteria's label. */
  .loops-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .loops button {
    min-width: 30px;
    font-variant-numeric: tabular-nums;
  }
  /* What a change set back to « Auto », across the grid. */
  .notice {
    grid-column: 1 / -1;
    color: var(--muted);
    font-size: 11.5px;
  }
  /* « Partager avec l'équipe » (ESC-190): the switch and its words at the left of the footer. */
  .share-foot {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    align-self: center;
    color: var(--soft);
    font-size: 12.5px;
    white-space: nowrap;
  }
  .share-foot > :global(.icon) {
    color: var(--muted);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  /* The popover of « Après », raised over the window: its search, then the tickets to check. */
  .pop {
    position: fixed;
    z-index: 60;
    display: grid;
    gap: 6px;
    padding: 8px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.6 * var(--shade)));
  }
  /* A long column of tickets scrolls rather than stretch the form. */
  .deps {
    display: grid;
    gap: 2px;
    max-height: 136px;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
  }
  .deps label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 4px 6px;
    border-radius: 6px;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 450;
    cursor: pointer;
  }
  .deps label:hover {
    background: var(--hover);
  }
  .deps input {
    flex: none;
    margin: 0;
    accent-color: var(--p-ink);
  }
  .deps .key {
    flex: none;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  /* A long title is cut, in full in its tooltip. */
  .deps .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .refusal {
    color: var(--del);
    font-size: 11.5px;
  }
  /* A short window: tighter, for the grid and the dependencies to show without scrolling. */
  .form.short {
    gap: 12px;
  }
  .form.short .fld {
    gap: 4px;
  }
  .form.short .atts {
    padding-top: 0;
  }
  /* The key that adds the ticket, at the foot's left: the buttons stay on the right. */
  .hint {
    flex: 1;
    align-self: center;
    color: var(--muted);
    font-size: 12px;
  }
</style>
