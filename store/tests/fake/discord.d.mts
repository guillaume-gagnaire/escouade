export interface FakeDiscord {
  base: string;
  received: { hook: string; at: number; body: any }[];
  hook(channel: string): string;
  of(channel: string): any[];
  plan(answers: { status: number; retryAfter?: number; delay?: number }[]): void;
  reset(): void;
  close(): Promise<void>;
}

export function fakeDiscord(port?: number): Promise<FakeDiscord>;
