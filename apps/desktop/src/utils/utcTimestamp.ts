// The backend stores timestamps as SQLite CURRENT_TIMESTAMP strings: UTC, formatted
// "YYYY-MM-DD HH:MM:SS", with no zone suffix. `new Date()` would read those as local time,
// so mark them as UTC before parsing.
const NAIVE_UTC_TIMESTAMP = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(\.\d+)?$/;

export const parseUtcTimestamp = (value: string): Date =>
  new Date(NAIVE_UTC_TIMESTAMP.test(value) ? `${value.replace(' ', 'T')}Z` : value);

// Local date and time for display; falls back to the raw string if it can't be parsed
export const formatUtcTimestamp = (value: string): string => {
  const date = parseUtcTimestamp(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
};

export const formatUtcDate = (value: string): string => {
  const date = parseUtcTimestamp(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString();
};
