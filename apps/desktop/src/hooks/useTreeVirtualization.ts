import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { AssetHierarchy } from '../types/assets';

export interface TreeVirtualizationConfig {
  data: AssetHierarchy[];
  expandedKeys: Set<number>;
  searchValue?: string;
  maxItems?: number;
  itemHeight?: number;
  overscanCount?: number;
}

export interface FlatTreeItem {
  id: number;
  asset: AssetHierarchy;
  level: number;
  isExpanded: boolean;
  hasChildren: boolean;
  isVisible: boolean;
  parentIds: number[];
  searchMatch?: boolean;
}

export interface PerformanceMetrics {
  totalItems: number;
  visibleItems: number;
  maxDepth: number;
  renderTime: number;
  searchTime: number;
  lastUpdate: Date;
}

export const useTreeVirtualization = ({
  data,
  expandedKeys,
  searchValue = '',
  maxItems = 10000,
  itemHeight = 32,
  overscanCount = 5,
}: TreeVirtualizationConfig) => {
  const [performanceMetrics, setPerformanceMetrics] = useState<PerformanceMetrics>({
    totalItems: 0,
    visibleItems: 0,
    maxDepth: 0,
    renderTime: 0,
    searchTime: 0,
    lastUpdate: new Date(),
  });

  const renderStartTime = useRef<number>(0);
  const searchStartTime = useRef<number>(0);

  // Memoized tree flattening with performance tracking
  const flattenedItems = useMemo(() => {
    renderStartTime.current = performance.now();

    const flattenTree = (
      items: AssetHierarchy[], 
      level: number = 0,
      parentIds: number[] = []
    ): FlatTreeItem[] => {
      const result: FlatTreeItem[] = [];
      
      for (const asset of items) {
        const isExpanded = expandedKeys.has(asset.id);
        const hasChildren = asset.children.length > 0;
        const currentParentIds = [...parentIds, asset.id];
        
        // Add current item
        result.push({
          id: asset.id,
          asset,
          level,
          isExpanded,
          hasChildren,
          isVisible: true,
          parentIds,
          searchMatch: false,
        });
        
        // Add children if expanded
        if (isExpanded && hasChildren) {
          result.push(...flattenTree(asset.children, level + 1, currentParentIds));
        }
      }
      
      return result;
    };

    const flattened = flattenTree(data);
    
    // Update performance metrics
    const renderTime = performance.now() - renderStartTime.current;
    const maxDepth = Math.max(...flattened.map(item => item.level), 0);
    
    setPerformanceMetrics(prev => ({
      ...prev,
      totalItems: flattened.length,
      maxDepth,
      renderTime,
      lastUpdate: new Date(),
    }));

    return flattened.slice(0, maxItems);
  }, [data, expandedKeys, maxItems]);

  // Search functionality with performance tracking
  const searchItems = useCallback((items: FlatTreeItem[], searchTerm: string): FlatTreeItem[] => {
    if (!searchTerm.trim()) {
      return items.map(item => ({ ...item, searchMatch: false }));
    }

    searchStartTime.current = performance.now();
    
    const searchLower = searchTerm.toLowerCase();
    const matchingIds = new Set<number>();
    const updatedItems: FlatTreeItem[] = [];
    
    // First pass: find direct matches
    items.forEach(item => {
      const nameMatch = item.asset.name.toLowerCase().includes(searchLower);
      const descriptionMatch = item.asset.description.toLowerCase().includes(searchLower);
      const isMatch = nameMatch || descriptionMatch;
      
      if (isMatch) {
        matchingIds.add(item.id);
        // Add all ancestors to show path
        item.parentIds.forEach(parentId => matchingIds.add(parentId));
      }
      
      updatedItems.push({
        ...item,
        searchMatch: isMatch,
      });
    });
    
    // Filter to only show matching items and their paths
    const filteredItems = updatedItems.filter(item => matchingIds.has(item.id));
    
    const searchTime = performance.now() - searchStartTime.current;
    setPerformanceMetrics(prev => ({
      ...prev,
      visibleItems: filteredItems.length,
      searchTime,
      lastUpdate: new Date(),
    }));
    
    return filteredItems;
  }, []);

  // Apply search filtering
  const visibleItems = useMemo(() => {
    return searchItems(flattenedItems, searchValue);
  }, [flattenedItems, searchValue, searchItems]);

  // Tree manipulation functions
  const expandNode = useCallback((nodeId: number) => {
    return new Set([...expandedKeys, nodeId]);
  }, [expandedKeys]);

  const collapseNode = useCallback((nodeId: number) => {
    const newKeys = new Set(expandedKeys);
    newKeys.delete(nodeId);
    return newKeys;
  }, [expandedKeys]);

  const toggleNode = useCallback((nodeId: number) => {
    return expandedKeys.has(nodeId) ? collapseNode(nodeId) : expandNode(nodeId);
  }, [expandedKeys, expandNode, collapseNode]);

  // Find item by ID
  const findItemById = useCallback((id: number): FlatTreeItem | undefined => {
    return visibleItems.find(item => item.id === id);
  }, [visibleItems]);

  // Get item path to root
  const getItemPath = useCallback((id: number): FlatTreeItem[] => {
    const item = findItemById(id);
    if (!item) return [];
    
    const path: FlatTreeItem[] = [];
    for (const parentId of item.parentIds) {
      const parent = findItemById(parentId);
      if (parent) {
        path.push(parent);
      }
    }
    path.push(item);
    
    return path;
  }, [findItemById]);

  // Scroll utilities for virtualized lists
  const getItemOffset = useCallback((index: number): number => {
    return index * itemHeight;
  }, [itemHeight]);

  const getVisibleRange = useCallback((scrollTop: number, containerHeight: number) => {
    const startIndex = Math.floor(scrollTop / itemHeight);
    const endIndex = Math.min(
      Math.ceil((scrollTop + containerHeight) / itemHeight) + overscanCount,
      visibleItems.length - 1
    );
    
    return {
      startIndex: Math.max(0, startIndex - overscanCount),
      endIndex,
      visibleStartIndex: startIndex,
      visibleEndIndex: Math.ceil((scrollTop + containerHeight) / itemHeight),
    };
  }, [itemHeight, overscanCount, visibleItems.length]);

  // Auto-expand search results for better UX
  useEffect(() => {
    if (searchValue.trim() && visibleItems.length > 0 && visibleItems.length < 100) {
      // Auto-expand items with search matches to show context
      const itemsToExpand = visibleItems
        .filter(item => item.hasChildren && item.searchMatch)
        .map(item => item.id);
      
      if (itemsToExpand.length > 0) {
        // This effect is just for tracking; actual expansion should be handled by parent
        console.debug('Auto-expand suggestions:', itemsToExpand);
      }
    }
  }, [searchValue, visibleItems]);

  // Performance monitoring
  const getPerformanceReport = useCallback(() => {
    return {
      ...performanceMetrics,
      memoryUsage: {
        totalItems: flattenedItems.length,
        visibleItems: visibleItems.length,
        cacheSize: flattenedItems.length * 50, // Rough estimate in bytes
      },
      performance: {
        isOptimal: performanceMetrics.renderTime < 16, // 60fps target
        renderTime: performanceMetrics.renderTime,
        searchTime: performanceMetrics.searchTime,
      },
      recommendations: {
        useVirtualization: flattenedItems.length > 100,
        useLazyLoading: performanceMetrics.maxDepth > 10,
        useDebouncing: flattenedItems.length > 500,
      },
    };
  }, [performanceMetrics, flattenedItems.length, visibleItems.length]);

  return {
    // Data
    flattenedItems,
    visibleItems,
    
    // Metrics
    performanceMetrics,
    getPerformanceReport,
    
    // Navigation
    expandNode,
    collapseNode,
    toggleNode,
    findItemById,
    getItemPath,
    
    // Search
    searchItems,
    
    // Virtualization utilities
    getItemOffset,
    getVisibleRange,
    itemHeight,
    totalHeight: visibleItems.length * itemHeight,
  };
};