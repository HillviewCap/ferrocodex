import { describe, it, expect } from 'vitest';
import { formatAssetLocation } from './assetLocation';

describe('formatAssetLocation', () => {
  const path = [
    { id: 1, name: 'Plant 1' },
    { id: 2, name: 'Building A' },
    { id: 3, name: 'Line 3' },
    { id: 4, name: 'PLC-001' },
  ];

  it('joins the ancestor folders and omits the asset itself', () => {
    expect(formatAssetLocation(path, 4)).toBe('Plant 1 › Building A › Line 3');
  });

  it('returns Root for a top-level asset', () => {
    expect(formatAssetLocation([{ id: 9, name: 'PLC-002' }], 9)).toBe('Root');
  });

  it('returns null for an empty or stale path', () => {
    expect(formatAssetLocation([], 4)).toBeNull();
    expect(formatAssetLocation(path, 99)).toBeNull();
  });
});
