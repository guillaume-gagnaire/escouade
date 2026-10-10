// jsdom has no PointerEvent: a mouse event that carries the pointer's id and, as a real pointer event does, coordinates
// that are not whole (on a scaled screen), for the tests of what is dragged with the pointer. Imported by those tests,
// not set up for all.

class TestPointerEvent extends MouseEvent {
  readonly pointerId: number;
  constructor(type: string, init: PointerEventInit = {}) {
    super(type, init);
    this.pointerId = init.pointerId ?? 0;
    Object.defineProperty(this, 'clientX', { value: init.clientX ?? 0, configurable: true });
    Object.defineProperty(this, 'clientY', { value: init.clientY ?? 0, configurable: true });
  }
}

if (typeof PointerEvent === 'undefined') {
  (globalThis as unknown as { PointerEvent: typeof TestPointerEvent }).PointerEvent = TestPointerEvent;
}
