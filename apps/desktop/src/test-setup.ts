import '@testing-library/jest-dom';
// antd 5 needs this to render message/notification/Modal static methods on React 19
import '@ant-design/v5-patch-for-react-19';
import { vi } from 'vitest';

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
