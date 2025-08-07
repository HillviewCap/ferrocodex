import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { AssetHierarchy, AssetInfo } from '../types/assets';
import { TreeCache } from '../utils/treePerformance';

export interface TreeCacheConfig {
  maxCacheSize: number;
  ttl: number; // Time to live in milliseconds
  preloadDepth: number;
  enableMetrics: boolean;
  persistToIndexedDB: boolean;
}

export interface CacheEntry<T> {
  data: T;
  timestamp: number;
  accessCount: number;
  lastAccessed: number;
  size: number; // Estimated size in bytes
}

export interface CacheMetrics {
  hitRate: number;
  missRate: number;
  totalRequests: number;
  cacheSize: number;
  memoryUsage: number;
  evictionCount: number;
  averageAccessTime: number;
}

export const useTreeCache = (config: Partial<TreeCacheConfig> = {}) => {
  const {
    maxCacheSize = 500,
    ttl = 5 * 60 * 1000, // 5 minutes
    preloadDepth = 2,
    enableMetrics = true,
    persistToIndexedDB = false,
  } = config;

  // Cache instances
  const nodeCache = useRef(new TreeCache<string, AssetHierarchy>(maxCacheSize));
  const pathCache = useRef(new TreeCache<number, AssetInfo[]>(Math.floor(maxCacheSize / 2)));
  const searchCache = useRef(new TreeCache<string, AssetHierarchy[]>(Math.floor(maxCacheSize / 4)));
  const metadataCache = useRef(new TreeCache<number, any>(maxCacheSize));

  // Metrics state
  const [metrics, setMetrics] = useState<CacheMetrics>({
    hitRate: 0,
    missRate: 0,
    totalRequests: 0,
    cacheSize: 0,
    memoryUsage: 0,
    evictionCount: 0,
    averageAccessTime: 0,
  });

  // Performance tracking
  const accessTimes = useRef<number[]>([]);
  const evictionCount = useRef(0);

  // Node caching with TTL
  const getCachedNode = useCallback((nodeId: number): AssetHierarchy | null => {
    const startTime = performance.now();
    const key = `node-${nodeId}`;
    
    const cached = nodeCache.current.get(key);
    const accessTime = performance.now() - startTime;
    
    if (enableMetrics) {
      accessTimes.current.push(accessTime);
      if (accessTimes.current.length > 100) {
        accessTimes.current.shift();
      }
    }
    
    if (cached) {
      // Check TTL
      if (Date.now() - (cached as any).timestamp < ttl) {
        return (cached as any).data;
      } else {
        // Expired, remove from cache
        nodeCache.current.delete(key);
      }
    }
    
    return null;
  }, [ttl, enableMetrics]);

  const setCachedNode = useCallback((nodeId: number, node: AssetHierarchy) => {
    const key = `node-${nodeId}`;
    const entry: CacheEntry<AssetHierarchy> = {
      data: node,
      timestamp: Date.now(),
      accessCount: 1,
      lastAccessed: Date.now(),
      size: estimateObjectSize(node),
    };
    
    nodeCache.current.set(key, entry as any);
  }, []);

  // Path caching for breadcrumbs
  const getCachedPath = useCallback((assetId: number): AssetInfo[] | null => {
    const cached = pathCache.current.get(assetId);
    if (cached && Date.now() - (cached as any).timestamp < ttl) {
      return (cached as any).data;
    }
    return null;
  }, [ttl]);

  const setCachedPath = useCallback((assetId: number, path: AssetInfo[]) => {
    const entry: CacheEntry<AssetInfo[]> = {
      data: path,
      timestamp: Date.now(),
      accessCount: 1,
      lastAccessed: Date.now(),
      size: estimateObjectSize(path),
    };
    
    pathCache.current.set(assetId, entry as any);
  }, []);

  // Search result caching
  const getCachedSearchResults = useCallback((query: string): AssetHierarchy[] | null => {
    const normalizedQuery = query.toLowerCase().trim();
    if (!normalizedQuery) return null;
    
    const cached = searchCache.current.get(normalizedQuery);
    if (cached && Date.now() - (cached as any).timestamp < ttl) {
      return (cached as any).data;
    }
    return null;
  }, [ttl]);

  const setCachedSearchResults = useCallback((query: string, results: AssetHierarchy[]) => {
    const normalizedQuery = query.toLowerCase().trim();
    if (!normalizedQuery) return;
    
    const entry: CacheEntry<AssetHierarchy[]> = {
      data: results,
      timestamp: Date.now(),
      accessCount: 1,
      lastAccessed: Date.now(),
      size: estimateObjectSize(results),
    };
    
    searchCache.current.set(normalizedQuery, entry as any);
  }, []);

  // Metadata caching (for additional asset information)
  const getCachedMetadata = useCallback((assetId: number): any | null => {
    const cached = metadataCache.current.get(assetId);
    if (cached && Date.now() - cached.timestamp < ttl) {
      return cached.data;
    }
    return null;
  }, [ttl]);

  const setCachedMetadata = useCallback((assetId: number, metadata: any) => {
    const entry: CacheEntry<any> = {
      data: metadata,
      timestamp: Date.now(),
      accessCount: 1,
      lastAccessed: Date.now(),
      size: estimateObjectSize(metadata),
    };
    
    metadataCache.current.set(assetId, entry);
  }, []);

  // Preloading functionality
  const preloadNodes = useCallback(async (
    rootNodes: AssetHierarchy[], 
    depth: number = preloadDepth,
    loadNodeFn: (nodeId: number) => Promise<AssetHierarchy>
  ) => {
    const preloadQueue: Array<{ nodeId: number; currentDepth: number }> = [];
    
    // Initialize queue with root nodes
    rootNodes.forEach(node => {
      if (node.children.length > 0) {
        preloadQueue.push({ nodeId: node.id, currentDepth: 0 });
      }
    });
    
    const preloadBatch = async (batch: typeof preloadQueue) => {
      const promises = batch.map(async ({ nodeId, currentDepth }) => {
        try {
          // Skip if already cached
          if (getCachedNode(nodeId)) return;
          
          const node = await loadNodeFn(nodeId);
          setCachedNode(nodeId, node);
          
          // Add children to queue if within depth limit
          if (currentDepth < depth - 1 && node.children.length > 0) {
            node.children.forEach(child => {
              preloadQueue.push({ 
                nodeId: child.id, 
                currentDepth: currentDepth + 1 
              });
            });
          }
        } catch (error) {
          console.debug(`Preload failed for node ${nodeId}:`, error);
        }
      });
      
      await Promise.all(promises);
    };
    
    // Process in batches to avoid overwhelming the system
    const batchSize = 5;
    while (preloadQueue.length > 0) {
      const batch = preloadQueue.splice(0, batchSize);
      await preloadBatch(batch);
      
      // Allow other tasks to run
      await new Promise(resolve => setTimeout(resolve, 0));
    }
  }, [preloadDepth, getCachedNode, setCachedNode]);

  // Cache invalidation
  const invalidateNode = useCallback((nodeId: number) => {
    const key = `node-${nodeId}`;
    nodeCache.current.delete(key);
    pathCache.current.delete(nodeId);
    metadataCache.current.delete(nodeId);
  }, []);

  const invalidateSearchCache = useCallback(() => {
    searchCache.current.clear();
  }, []);

  const invalidateAll = useCallback(() => {
    nodeCache.current.clear();
    pathCache.current.clear();
    searchCache.current.clear();
    metadataCache.current.clear();
  }, []);

  // Cache warming (preload commonly accessed items)
  const warmCache = useCallback(async (
    popularNodes: number[],
    loadNodeFn: (nodeId: number) => Promise<AssetHierarchy>,
    loadPathFn: (assetId: number) => Promise<AssetInfo[]>
  ) => {
    const warmingPromises = popularNodes.map(async (nodeId) => {
      try {
        if (!getCachedNode(nodeId)) {
          const node = await loadNodeFn(nodeId);
          setCachedNode(nodeId, node);
        }
        
        if (!getCachedPath(nodeId)) {
          const path = await loadPathFn(nodeId);
          setCachedPath(nodeId, path);
        }
      } catch (error) {
        console.debug(`Cache warming failed for node ${nodeId}:`, error);
      }
    });
    
    await Promise.all(warmingPromises);
  }, [getCachedNode, getCachedPath, setCachedNode, setCachedPath]);

  // Update metrics periodically
  useEffect(() => {
    if (!enableMetrics) return;
    
    const updateMetrics = () => {
      const nodeStats = nodeCache.current.getStats();
      const pathStats = pathCache.current.getStats();
      const searchStats = searchCache.current.getStats();
      const metadataStats = metadataCache.current.getStats();
      
      const totalHits = nodeStats.hitCount + pathStats.hitCount + searchStats.hitCount + metadataStats.hitCount;
      const totalMisses = nodeStats.missCount + pathStats.missCount + searchStats.missCount + metadataStats.missCount;
      const totalRequests = totalHits + totalMisses;
      
      const averageAccessTime = accessTimes.current.length > 0
        ? accessTimes.current.reduce((a, b) => a + b, 0) / accessTimes.current.length
        : 0;
      
      setMetrics({
        hitRate: totalRequests > 0 ? totalHits / totalRequests : 0,
        missRate: totalRequests > 0 ? totalMisses / totalRequests : 0,
        totalRequests,
        cacheSize: nodeStats.size + pathStats.size + searchStats.size + metadataStats.size,
        memoryUsage: estimateCacheMemoryUsage(),
        evictionCount: evictionCount.current,
        averageAccessTime,
      });
    };
    
    const interval = setInterval(updateMetrics, 1000);
    return () => clearInterval(interval);
  }, [enableMetrics]);

  // IndexedDB persistence (if enabled)
  useEffect(() => {
    if (!persistToIndexedDB) return;
    
    // Implementation would go here for IndexedDB persistence
    // This is a complex feature that would serialize cache to IndexedDB
    console.debug('IndexedDB persistence not yet implemented');
  }, [persistToIndexedDB]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      invalidateAll();
    };
  }, [invalidateAll]);

  // Computed cache statistics
  const cacheStatistics = useMemo(() => {
    const isEfficient = metrics.hitRate > 0.8; // 80% hit rate target
    const isOptimalSize = metrics.cacheSize < maxCacheSize * 0.9;
    const isResponsive = metrics.averageAccessTime < 5; // 5ms target
    
    return {
      ...metrics,
      isEfficient,
      isOptimalSize,
      isResponsive,
      recommendations: [
        ...(metrics.hitRate < 0.6 ? ['Consider increasing cache size or TTL'] : []),
        ...(metrics.averageAccessTime > 10 ? ['Consider optimizing cache access patterns'] : []),
        ...(metrics.cacheSize > maxCacheSize * 0.95 ? ['Cache is nearly full, consider eviction'] : []),
      ],
    };
  }, [metrics, maxCacheSize]);

  return {
    // Node operations
    getCachedNode,
    setCachedNode,
    invalidateNode,
    
    // Path operations
    getCachedPath,
    setCachedPath,
    
    // Search operations
    getCachedSearchResults,
    setCachedSearchResults,
    invalidateSearchCache,
    
    // Metadata operations
    getCachedMetadata,
    setCachedMetadata,
    
    // Batch operations
    preloadNodes,
    warmCache,
    invalidateAll,
    
    // Metrics and statistics
    metrics: cacheStatistics,
    
    // Cache management
    getCacheSnapshot: () => ({
      nodeCache: nodeCache.current.getStats(),
      pathCache: pathCache.current.getStats(),
      searchCache: searchCache.current.getStats(),
      metadataCache: metadataCache.current.getStats(),
    }),
  };
};

// Utility functions
function estimateObjectSize(obj: any): number {
  try {
    return JSON.stringify(obj).length * 2; // Rough estimate (2 bytes per char)
  } catch {
    return 0;
  }
}

function estimateCacheMemoryUsage(): number {
  if ('memory' in performance) {
    // @ts-ignore - Chrome-specific API
    return (performance as any).memory?.usedJSHeapSize || 0;
  }
  return 0;
}