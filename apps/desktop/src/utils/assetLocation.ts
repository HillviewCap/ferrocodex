/**
 * Folder path of an asset, e.g. "Plant 1 › Building A › Line 3", or "Root" when the
 * asset has no parent folder. `path` is the root-to-asset chain from `get_asset_path`
 * (the asset itself is the last entry). Returns null while the chain is not for the
 * given asset yet, so a stale path from a previous selection is never shown.
 */
export const formatAssetLocation = (
  path: Array<{ id: number; name: string }>,
  assetId: number
): string | null => {
  if (path.length === 0 || path[path.length - 1].id !== assetId) return null;
  const ancestors = path.slice(0, -1);
  return ancestors.length > 0 ? ancestors.map(a => a.name).join(' › ') : 'Root';
};
