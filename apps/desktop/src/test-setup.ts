import '@testing-library/jest-dom';
import { afterAll, vi } from 'vitest';

// Browser API stubs below are plain functions/classes rather than vi.fn() so that
// tests calling vi.resetAllMocks() can't wipe their implementations.

// Mock window.matchMedia for Ant Design
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock window.prompt for copy-to-clipboard functionality
Object.defineProperty(window, 'prompt', {
  writable: true,
  value: vi.fn(),
});

// Return the element's inline style instead of jsdom's real getComputedStyle, which
// re-evaluates every injected Ant Design CSS rule on each call and is slow enough to
// time tests out. Inline styles are still visible to toHaveStyle.
const emptyStyle = document.createElement('div').style;
Object.defineProperty(window, 'getComputedStyle', {
  writable: true,
  value: (elt: Element) => (elt as HTMLElement)?.style ?? emptyStyle,
});

class MockObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return [];
  }
}

// Mock ResizeObserver and IntersectionObserver for Ant Design
Object.defineProperty(window, 'ResizeObserver', { writable: true, value: MockObserver });
Object.defineProperty(window, 'IntersectionObserver', { writable: true, value: MockObserver });

// Mock HTMLCanvasElement.getContext for Ant Design Charts
Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
  writable: true,
  value: () => null,
});

// Timers can outlive a test file: antd's message/notification auto-close, the bulk import
// progress poll, component delays. If one fires after Vitest tears down jsdom, the React update
// it triggers throws "window is not defined" and fails the run even though every test passed.
// Track timers and clear the ones still pending once the file's tests are done.
const pendingTimers = new Set<ReturnType<typeof setTimeout>>();
const realSetTimeout = globalThis.setTimeout;
const realClearTimeout = globalThis.clearTimeout;
const realSetInterval = globalThis.setInterval;
const realClearInterval = globalThis.clearInterval;

globalThis.setTimeout = Object.assign(
  (callback: (...args: unknown[]) => void, ms?: number, ...args: unknown[]) => {
    const id = realSetTimeout((...callbackArgs: unknown[]) => {
      pendingTimers.delete(id);
      callback(...callbackArgs);
    }, ms, ...args);
    pendingTimers.add(id);
    return id;
  },
  realSetTimeout,
) as typeof setTimeout;
globalThis.clearTimeout = ((id?: ReturnType<typeof setTimeout>) => {
  if (id !== undefined) pendingTimers.delete(id);
  realClearTimeout(id);
}) as typeof clearTimeout;
globalThis.setInterval = Object.assign(
  (callback: (...args: unknown[]) => void, ms?: number, ...args: unknown[]) => {
    const id = realSetInterval(callback, ms, ...args);
    pendingTimers.add(id);
    return id;
  },
  realSetInterval,
) as typeof setInterval;
globalThis.clearInterval = ((id?: ReturnType<typeof setInterval>) => {
  if (id !== undefined) pendingTimers.delete(id);
  realClearInterval(id);
}) as typeof clearInterval;

afterAll(async () => {
  for (const id of pendingTimers) {
    realClearTimeout(id);
    realClearInterval(id);
  }
  pendingTimers.clear();
  // Let React finish work it already queued (it flushes effects in setImmediate tasks, which
  // run before the next timer)
  for (let i = 0; i < 3; i++) {
    await new Promise(resolve => realSetTimeout(resolve, 0));
  }
});
