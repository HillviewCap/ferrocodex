import { describe, it, expect } from 'vitest';
import { parseUtcTimestamp, formatUtcTimestamp, formatUtcDate } from './utcTimestamp';

describe('parseUtcTimestamp', () => {
  it('reads a zoneless database timestamp as UTC', () => {
    expect(parseUtcTimestamp('2026-10-19 14:30:05').getTime()).toBe(Date.UTC(2026, 9, 19, 14, 30, 5));
  });

  it('keeps fractional seconds', () => {
    expect(parseUtcTimestamp('2026-10-19 14:30:05.250').getTime()).toBe(Date.UTC(2026, 9, 19, 14, 30, 5, 250));
  });

  it('leaves timestamps that carry a zone alone', () => {
    expect(parseUtcTimestamp('2026-10-19T14:30:05Z').getTime()).toBe(Date.UTC(2026, 9, 19, 14, 30, 5));
    expect(parseUtcTimestamp('2026-10-19T16:30:05+02:00').getTime()).toBe(Date.UTC(2026, 9, 19, 14, 30, 5));
  });
});

describe('formatUtcTimestamp', () => {
  it('formats in local time', () => {
    expect(formatUtcTimestamp('2026-10-19 14:30:05')).toBe(new Date(Date.UTC(2026, 9, 19, 14, 30, 5)).toLocaleString());
  });

  it('returns unparseable input unchanged', () => {
    expect(formatUtcTimestamp('not a date')).toBe('not a date');
  });
});

describe('formatUtcDate', () => {
  it('formats the local date', () => {
    expect(formatUtcDate('2026-10-19 23:30:00')).toBe(new Date(Date.UTC(2026, 9, 19, 23, 30)).toLocaleDateString());
  });
});
