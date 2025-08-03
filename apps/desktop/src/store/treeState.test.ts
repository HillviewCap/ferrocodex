import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { 
  useTreeStateStore,
  useExpandedKeys,
  useSelectedAssetId,
  useTreeStateActions,
} from './treeState';

// Mock localStorage
const mockLocalStorage = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: mockLocalStorage,
});

describe('TreeState Store', () => {
  beforeEach(() => {
    // Clear all mocks
    vi.clearAllMocks();
    
    // Reset the store
    const { result } = renderHook(() => useTreeStateStore());
    act(() => {
      result.current.reset();
    });
  });

  describe('useTreeStateStore', () => {
    it('initializes with default state', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      expect(result.current.expandedKeys).toEqual(new Set());
      expect(result.current.selectedAssetId).toBeNull();
      expect(result.current.breadcrumbPath).toEqual([]);
      expect(result.current.navigationHistory).toEqual([]);
      expect(result.current.currentPosition).toBe(-1);
      expect(result.current.viewMode).toBe('virtualized');
      expect(result.current.searchValue).toBe('');
      expect(result.current.showHidden).toBe(false);
      expect(result.current.sortBy).toBe('name');
      expect(result.current.sortOrder).toBe('asc');
      expect(result.current.virtualizationEnabled).toBe(true);
      expect(result.current.itemHeight).toBe(32);
      expect(result.current.overscanCount).toBe(5);
      expect(result.current.sidebarWidth).toBe(400);
      expect(result.current.showPerformanceMetrics).toBe(false);
      expect(result.current.compactMode).toBe(false);
    });

    it('expands and collapses nodes', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.expandNode(1);
      });
      
      expect(result.current.expandedKeys.has(1)).toBe(true);
      
      act(() => {
        result.current.collapseNode(1);
      });
      
      expect(result.current.expandedKeys.has(1)).toBe(false);
    });

    it('toggles nodes', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      // Initially not expanded
      expect(result.current.expandedKeys.has(1)).toBe(false);
      
      act(() => {
        result.current.toggleNode(1);
      });
      
      expect(result.current.expandedKeys.has(1)).toBe(true);
      
      act(() => {
        result.current.toggleNode(1);
      });
      
      expect(result.current.expandedKeys.has(1)).toBe(false);
    });

    it('selects assets and manages navigation history', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.selectAsset(1);
      });
      
      expect(result.current.selectedAssetId).toBe(1);
      expect(result.current.navigationHistory).toEqual([1]);
      expect(result.current.currentPosition).toBe(0);
      
      act(() => {
        result.current.selectAsset(2);
      });
      
      expect(result.current.selectedAssetId).toBe(2);
      expect(result.current.navigationHistory).toEqual([1, 2]);
      expect(result.current.currentPosition).toBe(1);
    });

    it('navigates back and forward through history', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      // Build navigation history
      act(() => {
        result.current.selectAsset(1);
        result.current.selectAsset(2);
        result.current.selectAsset(3);
      });
      
      expect(result.current.selectedAssetId).toBe(3);
      expect(result.current.currentPosition).toBe(2);
      
      // Navigate back
      act(() => {
        result.current.navigateBack();
      });
      
      expect(result.current.selectedAssetId).toBe(2);
      expect(result.current.currentPosition).toBe(1);
      
      act(() => {
        result.current.navigateBack();
      });
      
      expect(result.current.selectedAssetId).toBe(1);
      expect(result.current.currentPosition).toBe(0);
      
      // Can't navigate back further
      act(() => {
        result.current.navigateBack();
      });
      
      expect(result.current.selectedAssetId).toBe(1);
      expect(result.current.currentPosition).toBe(0);
      
      // Navigate forward
      act(() => {
        result.current.navigateForward();
      });
      
      expect(result.current.selectedAssetId).toBe(2);
      expect(result.current.currentPosition).toBe(1);
    });

    it('clears navigation history', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.selectAsset(1);
        result.current.selectAsset(2);
      });
      
      expect(result.current.navigationHistory.length).toBe(2);
      
      act(() => {
        result.current.clearNavigationHistory();
      });
      
      expect(result.current.navigationHistory).toEqual([]);
      expect(result.current.currentPosition).toBe(-1);
    });

    it('updates search value', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.setSearchValue('test search');
      });
      
      expect(result.current.searchValue).toBe('test search');
    });

    it('updates view preferences', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.setViewMode('tree');
        result.current.setSortBy('created_at');
        result.current.setSortOrder('desc');
        result.current.setShowHidden(true);
      });
      
      expect(result.current.viewMode).toBe('tree');
      expect(result.current.sortBy).toBe('created_at');
      expect(result.current.sortOrder).toBe('desc');
      expect(result.current.showHidden).toBe(true);
    });

    it('updates virtualization settings with validation', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.setVirtualizationEnabled(false);
        result.current.setItemHeight(10); // Should be clamped to 24
        result.current.setOverscanCount(50); // Should be clamped to 20
      });
      
      expect(result.current.virtualizationEnabled).toBe(false);
      expect(result.current.itemHeight).toBe(24); // Minimum value
      expect(result.current.overscanCount).toBe(20); // Maximum value
      
      act(() => {
        result.current.setItemHeight(150); // Should be clamped to 100
        result.current.setOverscanCount(0); // Should be clamped to 1
      });
      
      expect(result.current.itemHeight).toBe(100); // Maximum value
      expect(result.current.overscanCount).toBe(1); // Minimum value
    });

    it('updates UI preferences with validation', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.setSidebarWidth(100); // Should be clamped to 200
        result.current.setShowPerformanceMetrics(true);
        result.current.setCompactMode(true);
      });
      
      expect(result.current.sidebarWidth).toBe(200); // Minimum value
      expect(result.current.showPerformanceMetrics).toBe(true);
      expect(result.current.compactMode).toBe(true);
      
      act(() => {
        result.current.setSidebarWidth(1000); // Should be clamped to 800
      });
      
      expect(result.current.sidebarWidth).toBe(800); // Maximum value
    });

    it('collapses all nodes', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.expandNode(1);
        result.current.expandNode(2);
        result.current.expandNode(3);
      });
      
      expect(result.current.expandedKeys.size).toBe(3);
      
      act(() => {
        result.current.collapseAll();
      });
      
      expect(result.current.expandedKeys.size).toBe(0);
    });

    it('restores view state', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      const newState = {
        expandedKeys: new Set([1, 2, 3]),
        selectedAssetId: 2,
        viewMode: 'tree' as const,
        searchValue: 'restored search',
        compactMode: true,
      };
      
      act(() => {
        result.current.restoreViewState(newState);
      });
      
      expect(result.current.expandedKeys).toEqual(new Set([1, 2, 3]));
      expect(result.current.selectedAssetId).toBe(2);
      expect(result.current.viewMode).toBe('tree');
      expect(result.current.searchValue).toBe('restored search');
      expect(result.current.compactMode).toBe(true);
    });

    it('gets persisted state', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      act(() => {
        result.current.expandNode(1);
        result.current.selectAsset(2);
        result.current.setViewMode('tree');
      });
      
      const persistedState = result.current.getPersistedState();
      
      expect(persistedState.expandedKeys).toEqual(new Set([1]));
      expect(persistedState.selectedAssetId).toBe(2);
      expect(persistedState.viewMode).toBe('tree');
    });

    it('resets to initial state', () => {
      const { result } = renderHook(() => useTreeStateStore());
      
      // Modify state
      act(() => {
        result.current.expandNode(1);
        result.current.selectAsset(2);
        result.current.setSearchValue('test');
        result.current.setCompactMode(true);
      });
      
      // Verify state was modified
      expect(result.current.expandedKeys.size).toBe(1);
      expect(result.current.selectedAssetId).toBe(2);
      expect(result.current.searchValue).toBe('test');
      expect(result.current.compactMode).toBe(true);
      
      // Reset
      act(() => {
        result.current.reset();
      });
      
      // Verify state was reset
      expect(result.current.expandedKeys.size).toBe(0);
      expect(result.current.selectedAssetId).toBeNull();
      expect(result.current.searchValue).toBe('');
      expect(result.current.compactMode).toBe(false);
    });
  });

  describe('Selector hooks', () => {
    it('useExpandedKeys returns expanded keys', () => {
      const { result: storeResult } = renderHook(() => useTreeStateStore());
      const { result: selectorResult } = renderHook(() => useExpandedKeys());
      
      act(() => {
        storeResult.current.expandNode(1);
        storeResult.current.expandNode(2);
      });
      
      expect(selectorResult.current).toEqual(new Set([1, 2]));
    });

    it('useSelectedAssetId returns selected asset ID', () => {
      const { result: storeResult } = renderHook(() => useTreeStateStore());
      const { result: selectorResult } = renderHook(() => useSelectedAssetId());
      
      act(() => {
        storeResult.current.selectAsset(42);
      });
      
      expect(selectorResult.current).toBe(42);
    });

    it('useTreeStateActions returns action functions', () => {
      const { result } = renderHook(() => useTreeStateActions());
      
      // Check that actions object has the expected function properties
      expect(result.current).toHaveProperty('expandNode');
      expect(result.current).toHaveProperty('collapseNode');
      expect(result.current).toHaveProperty('selectAsset');
      expect(result.current).toHaveProperty('setSearchValue');
      expect(result.current).toHaveProperty('reset');
      
      expect(typeof result.current.expandNode).toBe('function');
      expect(typeof result.current.collapseNode).toBe('function');
      expect(typeof result.current.selectAsset).toBe('function');
      expect(typeof result.current.setSearchValue).toBe('function');
      expect(typeof result.current.reset).toBe('function');
    });
  });

  describe('Persistence', () => {
    it('attempts to save to localStorage', () => {
      // The actual persistence is handled by Zustand middleware
      // This test verifies the store initializes correctly with mocked localStorage
      const { result } = renderHook(() => useTreeStateStore());
      
      expect(result.current).toBeDefined();
      expect(typeof result.current.expandNode).toBe('function');
    });
  });
});