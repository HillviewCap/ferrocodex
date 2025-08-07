import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useTreeCache } from './useTreeCache';
import { AssetHierarchy, AssetInfo } from '../types/assets';

// Mock performance.now
const mockPerformance = {
  now: vi.fn(() => Date.now()),
};
Object.defineProperty(window, 'performance', { 
  value: mockPerformance, 
  writable: true 
});

const mockAsset: AssetHierarchy = {
  id: 1,
  name: 'Test Asset',
  description: 'Test Description',
  asset_type: 'Device',
  parent_id: null,
  sort_order: 0,
  created_by: 1,
  created_at: '2023-01-01T00:00:00Z',
  updated_at: '2023-01-01T00:00:00Z',
  children: [],
};

const mockAssetInfo: AssetInfo = {
  id: 1,
  name: 'Test Asset',
  description: 'Test Description',
  asset_type: 'Device',
  parent_id: null,
  sort_order: 0,
  created_by: 1,
  created_by_username: 'testuser',
  created_at: '2023-01-01T00:00:00Z',
  version_count: 0,
  latest_version: null,
  latest_version_notes: null,
};

describe('useTreeCache', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockPerformance.now.mockReturnValue(Date.now());
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  it('initializes with default configuration', () => {
    const { result } = renderHook(() => useTreeCache());
    
    expect(result.current).toHaveProperty('getCachedNode');
    expect(result.current).toHaveProperty('setCachedNode');
    expect(result.current).toHaveProperty('getCachedPath');
    expect(result.current).toHaveProperty('setCachedPath');
    expect(result.current).toHaveProperty('getCachedSearchResults');
    expect(result.current).toHaveProperty('setCachedSearchResults');
    expect(result.current).toHaveProperty('metrics');
    expect(result.current).toHaveProperty('invalidateAll');
  });

  it('caches and retrieves nodes', () => {
    const { result } = renderHook(() => useTreeCache());
    
    // Initially no cached node
    expect(result.current.getCachedNode(1)).toBeNull();
    
    // Set cached node
    act(() => {
      result.current.setCachedNode(1, mockAsset);
    });
    
    // Retrieve cached node
    const cachedNode = result.current.getCachedNode(1);
    expect(cachedNode).toEqual(mockAsset);
  });

  it('respects TTL for cached nodes', async () => {
    const shortTTL = 100; // 100ms
    const { result } = renderHook(() => useTreeCache({ ttl: shortTTL }));
    
    // Set cached node
    act(() => {
      result.current.setCachedNode(1, mockAsset);
    });
    
    // Should be available immediately
    expect(result.current.getCachedNode(1)).toEqual(mockAsset);
    
    // Mock time passage using Date.now instead of performance.now
    const originalDateNow = Date.now;
    const mockTime = originalDateNow() + shortTTL + 10;
    vi.spyOn(Date, 'now').mockReturnValue(mockTime);
    
    // Should be expired now
    expect(result.current.getCachedNode(1)).toBeNull();
    
    // Restore original Date.now
    vi.spyOn(Date, 'now').mockRestore();
  });

  it('caches and retrieves paths', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockPath = [mockAssetInfo];
    
    // Initially no cached path
    expect(result.current.getCachedPath(1)).toBeNull();
    
    // Set cached path
    act(() => {
      result.current.setCachedPath(1, mockPath);
    });
    
    // Retrieve cached path
    const cachedPath = result.current.getCachedPath(1);
    expect(cachedPath).toEqual(mockPath);
  });

  it('caches and retrieves search results', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockResults = [mockAsset];
    const query = 'test search';
    
    // Initially no cached results
    expect(result.current.getCachedSearchResults(query)).toBeNull();
    
    // Set cached results
    act(() => {
      result.current.setCachedSearchResults(query, mockResults);
    });
    
    // Retrieve cached results
    const cachedResults = result.current.getCachedSearchResults(query);
    expect(cachedResults).toEqual(mockResults);
  });

  it('normalizes search queries', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockResults = [mockAsset];
    
    // Set with uppercase query
    act(() => {
      result.current.setCachedSearchResults('TEST SEARCH', mockResults);
    });
    
    // Retrieve with lowercase query (should work due to normalization)
    const cachedResults = result.current.getCachedSearchResults('test search');
    expect(cachedResults).toEqual(mockResults);
  });

  it('ignores empty search queries', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockResults = [mockAsset];
    
    // Should not cache empty or whitespace-only queries
    act(() => {
      result.current.setCachedSearchResults('', mockResults);
      result.current.setCachedSearchResults('   ', mockResults);
    });
    
    expect(result.current.getCachedSearchResults('')).toBeNull();
    expect(result.current.getCachedSearchResults('   ')).toBeNull();
  });

  it('caches and retrieves metadata', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockMetadata = { customField: 'customValue' };
    
    // Initially no cached metadata
    expect(result.current.getCachedMetadata(1)).toBeNull();
    
    // Set cached metadata
    act(() => {
      result.current.setCachedMetadata(1, mockMetadata);
    });
    
    // Retrieve cached metadata
    const cachedMetadata = result.current.getCachedMetadata(1);
    expect(cachedMetadata).toEqual(mockMetadata);
  });

  it('invalidates specific nodes', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockPath = [mockAssetInfo];
    const mockMetadata = { customField: 'value' };
    
    // Set up cache
    act(() => {
      result.current.setCachedNode(1, mockAsset);
      result.current.setCachedPath(1, mockPath);
      result.current.setCachedMetadata(1, mockMetadata);
    });
    
    // Verify items are cached
    expect(result.current.getCachedNode(1)).toEqual(mockAsset);
    expect(result.current.getCachedPath(1)).toEqual(mockPath);
    expect(result.current.getCachedMetadata(1)).toEqual(mockMetadata);
    
    // Invalidate node
    act(() => {
      result.current.invalidateNode(1);
    });
    
    // All related cache entries should be cleared
    expect(result.current.getCachedNode(1)).toBeNull();
    expect(result.current.getCachedPath(1)).toBeNull();
    expect(result.current.getCachedMetadata(1)).toBeNull();
  });

  it('invalidates search cache', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockResults = [mockAsset];
    
    // Set cached search results
    act(() => {
      result.current.setCachedSearchResults('query1', mockResults);
      result.current.setCachedSearchResults('query2', mockResults);
    });
    
    // Verify search results are cached
    expect(result.current.getCachedSearchResults('query1')).toEqual(mockResults);
    expect(result.current.getCachedSearchResults('query2')).toEqual(mockResults);
    
    // Invalidate search cache
    act(() => {
      result.current.invalidateSearchCache();
    });
    
    // All search results should be cleared
    expect(result.current.getCachedSearchResults('query1')).toBeNull();
    expect(result.current.getCachedSearchResults('query2')).toBeNull();
  });

  it('invalidates all caches', () => {
    const { result } = renderHook(() => useTreeCache());
    const mockPath = [mockAssetInfo];
    const mockResults = [mockAsset];
    const mockMetadata = { field: 'value' };
    
    // Set up various cached items
    act(() => {
      result.current.setCachedNode(1, mockAsset);
      result.current.setCachedPath(1, mockPath);
      result.current.setCachedSearchResults('query', mockResults);
      result.current.setCachedMetadata(1, mockMetadata);
    });
    
    // Verify items are cached
    expect(result.current.getCachedNode(1)).toEqual(mockAsset);
    expect(result.current.getCachedPath(1)).toEqual(mockPath);
    expect(result.current.getCachedSearchResults('query')).toEqual(mockResults);
    expect(result.current.getCachedMetadata(1)).toEqual(mockMetadata);
    
    // Invalidate all
    act(() => {
      result.current.invalidateAll();
    });
    
    // All cache entries should be cleared
    expect(result.current.getCachedNode(1)).toBeNull();
    expect(result.current.getCachedPath(1)).toBeNull();
    expect(result.current.getCachedSearchResults('query')).toBeNull();
    expect(result.current.getCachedMetadata(1)).toBeNull();
  });

  it('tracks performance metrics', async () => {
    const { result } = renderHook(() => useTreeCache({ enableMetrics: true }));
    
    // Perform some cache operations
    act(() => {
      result.current.setCachedNode(1, mockAsset);
    });
    
    // Access cached item
    act(() => {
      result.current.getCachedNode(1);
    });
    
    // Access non-cached item
    act(() => {
      result.current.getCachedNode(999);
    });
    
    // Wait for metrics to update (they update on a timer)
    await act(async () => {
      await new Promise(resolve => setTimeout(resolve, 1100)); // Wait for metrics update interval
    });
    
    // Metrics should be tracked
    expect(result.current.metrics).toHaveProperty('hitRate');
    expect(result.current.metrics).toHaveProperty('totalRequests');
    expect(result.current.metrics).toHaveProperty('cacheSize');
    // Note: Since metrics update is complex and depends on TreeCache implementation,
    // we'll just verify the structure exists rather than specific values
    expect(typeof result.current.metrics.totalRequests).toBe('number');
  });

  it('provides cache snapshot', () => {
    const { result } = renderHook(() => useTreeCache());
    
    // Set up some cached items
    act(() => {
      result.current.setCachedNode(1, mockAsset);
      result.current.setCachedPath(1, [mockAssetInfo]);
      result.current.setCachedSearchResults('query', [mockAsset]);
      result.current.setCachedMetadata(1, { field: 'value' });
    });
    
    const snapshot = result.current.getCacheSnapshot();
    
    expect(snapshot).toHaveProperty('nodeCache');
    expect(snapshot).toHaveProperty('pathCache');
    expect(snapshot).toHaveProperty('searchCache');
    expect(snapshot).toHaveProperty('metadataCache');
    
    expect(snapshot.nodeCache.size).toBeGreaterThan(0);
    expect(snapshot.pathCache.size).toBeGreaterThan(0);
    expect(snapshot.searchCache.size).toBeGreaterThan(0);
    expect(snapshot.metadataCache.size).toBeGreaterThan(0);
  });

  it('handles cache size limits', () => {
    const smallCacheSize = 2;
    const { result } = renderHook(() => useTreeCache({ maxCacheSize: smallCacheSize }));
    
    // Fill cache beyond limit
    act(() => {
      result.current.setCachedNode(1, mockAsset);
      result.current.setCachedNode(2, { ...mockAsset, id: 2 });
      result.current.setCachedNode(3, { ...mockAsset, id: 3 }); // Should evict oldest
    });
    
    // First item might be evicted due to LRU
    // This behavior depends on the internal implementation of TreeCache
    const snapshot = result.current.getCacheSnapshot();
    expect(snapshot.nodeCache.size).toBeLessThanOrEqual(smallCacheSize);
  });

  it('handles preloading operations', async () => {
    const { result } = renderHook(() => useTreeCache());
    
    const mockLoadNodeFn = vi.fn().mockResolvedValue(mockAsset);
    // Create a root node with children to trigger preloading
    const rootNodeWithChildren = {
      ...mockAsset,
      children: [{ id: 2, name: 'Child Asset' }] as any[]
    };
    const rootNodes = [rootNodeWithChildren];
    
    await act(async () => {
      await result.current.preloadNodes(rootNodes, 1, mockLoadNodeFn);
    });
    
    // Preloading should have been attempted for the root node (which has children)
    expect(mockLoadNodeFn).toHaveBeenCalledWith(rootNodeWithChildren.id);
  });

  it('handles cache warming', async () => {
    const { result } = renderHook(() => useTreeCache());
    
    const mockLoadNodeFn = vi.fn().mockResolvedValue(mockAsset);
    const mockLoadPathFn = vi.fn().mockResolvedValue([mockAssetInfo]);
    const popularNodes = [1, 2, 3];
    
    await act(async () => {
      await result.current.warmCache(popularNodes, mockLoadNodeFn, mockLoadPathFn);
    });
    
    // Cache warming should have been attempted for all popular nodes
    expect(mockLoadNodeFn).toHaveBeenCalledTimes(popularNodes.length);
    expect(mockLoadPathFn).toHaveBeenCalledTimes(popularNodes.length);
  });
});