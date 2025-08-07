import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useHierarchyStore } from './hierarchy';
import { AssetHierarchy } from '../types/assets';

// Mock Tauri API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

describe('HierarchyStore', () => {
  const mockHierarchyData: AssetHierarchy[] = [
    {
      id: 1,
      name: 'Production Line 1',
      description: 'Main production line',
      asset_type: 'Folder',
      parent_id: null,
      sort_order: 0,
      children: [
        {
          id: 2,
          name: 'PLC-001',
          description: 'Primary PLC',
          asset_type: 'Device',
          parent_id: 1,
          sort_order: 0,
          children: [],
          created_by: 1,
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ],
      created_by: 1,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    useHierarchyStore.getState().reset();
  });

  it('initializes with empty state', () => {
    const state = useHierarchyStore.getState();
    
    expect(state.hierarchyData).toEqual([]);
    expect(state.selectedAsset).toBeNull();
    expect(state.expandedKeys).toEqual([]);
    expect(state.isLoading).toBe(false);
    expect(state.isMoving).toBe(false);
    expect(state.error).toBeNull();
  });

  it('loads hierarchy data successfully', async () => {
    mockInvoke.mockResolvedValueOnce(mockHierarchyData);

    const { loadHierarchy } = useHierarchyStore.getState();
    await loadHierarchy('mock-token');

    const state = useHierarchyStore.getState();
    expect(state.hierarchyData).toEqual(mockHierarchyData);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith('get_asset_hierarchy', { token: 'mock-token' });
  });

  it('handles hierarchy loading error', async () => {
    const errorMessage = 'Failed to load hierarchy';
    mockInvoke.mockRejectedValueOnce(new Error(errorMessage));

    const { loadHierarchy } = useHierarchyStore.getState();
    await loadHierarchy('mock-token');

    const state = useHierarchyStore.getState();
    expect(state.hierarchyData).toEqual([]);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBe(errorMessage);
  });

  it('selects asset correctly', () => {
    const { selectAsset } = useHierarchyStore.getState();
    const asset = mockHierarchyData[0];
    
    selectAsset(asset);
    
    const state = useHierarchyStore.getState();
    expect(state.selectedAsset).toBe(asset);
  });

  it('clears selected asset', () => {
    const { selectAsset } = useHierarchyStore.getState();
    const asset = mockHierarchyData[0];
    
    selectAsset(asset);
    selectAsset(null);
    
    const state = useHierarchyStore.getState();
    expect(state.selectedAsset).toBeNull();
  });

  it('sets expanded keys', () => {
    const { setExpandedKeys } = useHierarchyStore.getState();
    const keys = ['1', '2', '3'];
    
    setExpandedKeys(keys);
    
    const state = useHierarchyStore.getState();
    expect(state.expandedKeys).toEqual(keys);
  });

  it('moves asset successfully', async () => {
    // Setup initial hierarchy data
    useHierarchyStore.setState({ hierarchyData: mockHierarchyData });
    
    mockInvoke
      .mockResolvedValueOnce(undefined) // move_asset call
      .mockResolvedValueOnce(mockHierarchyData); // refresh hierarchy call

    const { moveAsset } = useHierarchyStore.getState();
    await moveAsset('mock-token', {
      asset_id: 2,
      new_parent_id: null,
      new_sort_order: 0,
    });

    const state = useHierarchyStore.getState();
    expect(state.isMoving).toBe(false);
    expect(state.error).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith('move_asset', {
      token: 'mock-token',
      assetId: 2,
      newParentId: null,
      newSortOrder: 0,
    });
  });

  it('handles move asset error', async () => {
    const errorMessage = 'Failed to move asset';
    mockInvoke.mockRejectedValueOnce(new Error(errorMessage));

    const { moveAsset } = useHierarchyStore.getState();
    
    await expect(moveAsset('mock-token', {
      asset_id: 2,
      new_parent_id: null,
    })).rejects.toThrow(errorMessage);

    const state = useHierarchyStore.getState();
    expect(state.isMoving).toBe(false);
    expect(state.error).toBe(errorMessage);
  });

  it('validates move correctly', async () => {
    mockInvoke.mockResolvedValueOnce(true);

    const { validateMove } = useHierarchyStore.getState();
    const result = await validateMove('mock-token', 2, null);

    expect(result).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith('validate_asset_move', {
      token: 'mock-token',
      assetId: 2,
      newParentId: null,
    });
  });

  it('handles validate move error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Validation failed'));

    const { validateMove } = useHierarchyStore.getState();
    const result = await validateMove('mock-token', 2, null);

    expect(result).toBe(false);
  });

  it('finds asset by id correctly', () => {
    useHierarchyStore.setState({ hierarchyData: mockHierarchyData });

    const { findAssetById } = useHierarchyStore.getState();
    
    const asset1 = findAssetById(1);
    expect(asset1).toBe(mockHierarchyData[0]);
    
    const asset2 = findAssetById(2);
    expect(asset2).toBe(mockHierarchyData[0].children[0]);
    
    const nonExistent = findAssetById(999);
    expect(nonExistent).toBeNull();
  });

  it('gets asset path correctly', async () => {
    const mockPath = [
      { id: 1, name: 'Production Line 1' },
      { id: 2, name: 'PLC-001' },
    ];
    mockInvoke.mockResolvedValueOnce(mockPath);

    const { getAssetPath } = useHierarchyStore.getState();
    const result = await getAssetPath('mock-token', 2);

    expect(result).toEqual(mockPath);
    expect(mockInvoke).toHaveBeenCalledWith('get_asset_path', {
      token: 'mock-token',
      assetId: 2,
    });
  });

  it('refreshes hierarchy correctly', async () => {
    mockInvoke.mockResolvedValueOnce(mockHierarchyData);

    const { refreshHierarchy } = useHierarchyStore.getState();
    await refreshHierarchy('mock-token');

    const state = useHierarchyStore.getState();
    expect(state.hierarchyData).toEqual(mockHierarchyData);
  });

  it('resets state correctly', () => {
    // Set some state
    useHierarchyStore.setState({
      hierarchyData: mockHierarchyData,
      selectedAsset: mockHierarchyData[0],
      expandedKeys: ['1', '2'],
      error: 'Some error',
    });

    const { reset } = useHierarchyStore.getState();
    reset();

    const state = useHierarchyStore.getState();
    expect(state.hierarchyData).toEqual([]);
    expect(state.selectedAsset).toBeNull();
    expect(state.expandedKeys).toEqual([]);
    expect(state.error).toBeNull();
  });
});