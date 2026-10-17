import { afterEach, beforeEach, expect, vi } from 'vitest';
import { cleanup } from '@testing-library/react';

/**
 * Fails a test when React or Ant Design emits a dev-mode warning during it.
 *
 * antd 6 logs `Warning: [antd: Component] \`prop\` is deprecated...` through rc-util
 * (console.error) when a deprecated prop or children-based API is still in use, so
 * this is the check that proves a screen was really migrated to the `items` style.
 *
 * React 19 dropped the `Warning:` prefix from its own messages, so a few of its
 * well-known phrasings are matched explicitly. `act(...)` warnings are deliberately
 * not treated as failures; they are about test timing, not component correctness.
 */
const FRAMEWORK_WARNING_PATTERNS: RegExp[] = [
  /^Warning:/,
  /\[antd\b/,
  /is deprecated/i,
  /Each child in a list should have a unique/,
  /Encountered two children with the same key/,
  /React does not recognize the/,
  /Received `.*` for a non-boolean attribute/,
  /Invalid (value for prop|DOM property|prop)/,
  /Unknown event handler property/,
  /cannot be a child of|validateDOMNesting|cannot appear as a (child|descendant)/,
  /Cannot update a component/,
  /Maximum update depth exceeded/,
  /changing an? (un)?controlled (input|component)/,
  /Function components cannot be given refs/,
  /Accessing element\.ref/,
  /The tag <.*> is unrecognized in this browser/,
  /Failed prop type/,
  /findDOMNode is deprecated/,
];

const formatArgs = (args: unknown[]): string =>
  args
    .map(arg => {
      if (typeof arg === 'string') return arg;
      if (arg instanceof Error) return arg.message;
      try {
        return JSON.stringify(arg);
      } catch {
        return String(arg);
      }
    })
    .join(' ');

export const isFrameworkWarning = (args: unknown[]): boolean => {
  const text = formatArgs(args);
  return FRAMEWORK_WARNING_PATTERNS.some(pattern => pattern.test(text));
};

/**
 * Call once at the top of a `describe` block (or at module scope). Installs
 * console.error/console.warn spies before each test and asserts after each test that
 * nothing React- or antd-shaped was logged. Non-framework logging (a component's own
 * `console.error('Failed to load ...')`) is silenced but ignored.
 */
export interface ConsoleGuardOptions {
  /**
   * Warnings to tolerate. Use only for documented, pre-existing warnings that are not
   * deprecations (say why in a comment at the call site) so the guard stays meaningful.
   */
  ignore?: RegExp[];
}

export function useConsoleGuard(options: ConsoleGuardOptions = {}): void {
  const ignore = options.ignore ?? [];
  let errorSpy: ReturnType<typeof vi.spyOn> | undefined;
  let warnSpy: ReturnType<typeof vi.spyOn> | undefined;

  beforeEach(() => {
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    // Unmount before asserting. If this hook throws, Testing Library's own afterEach
    // cleanup does not run, and the still-mounted tree would re-render (and re-warn)
    // inside the next test.
    cleanup();
    const calls = [...(errorSpy?.mock.calls ?? []), ...(warnSpy?.mock.calls ?? [])];
    errorSpy?.mockRestore();
    warnSpy?.mockRestore();

    const offenders = calls
      .filter(isFrameworkWarning)
      .map(formatArgs)
      .filter(text => !ignore.some(pattern => pattern.test(text)));
    expect(offenders, 'React/antd warnings were emitted during the test').toEqual([]);
  });
}
