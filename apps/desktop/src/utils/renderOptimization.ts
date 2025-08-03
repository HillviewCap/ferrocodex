/**
 * Render Optimization Utilities
 * 
 * Provides utilities for optimizing React render performance in tree components,
 * including memoization strategies, virtual scrolling helpers, and render scheduling.
 */

import React, { memo, useMemo, useCallback, useRef, useEffect } from 'react';
import { AssetHierarchy } from '../types/assets';

// Memoization utilities
export interface MemoizationConfig {
  compareProps?: (prevProps: any, nextProps: any) => boolean;
  compareChildren?: boolean;
  deepCompare?: boolean;
  maxCacheSize?: number;
}

// Create a memoized component with custom comparison
export function createMemoizedTreeNode<T>(
  Component: React.ComponentType<T>,
  config: MemoizationConfig = {}
) {
  const {
    compareProps,
    compareChildren = false,
    deepCompare = false,
    maxCacheSize = 100,
  } = config;

  return memo(Component, (prevProps, nextProps) => {
    // Custom prop comparison if provided
    if (compareProps) {
      return compareProps(prevProps, nextProps);
    }

    // Default shallow comparison with optimizations
    const prevKeys = Object.keys(prevProps as any);
    const nextKeys = Object.keys(nextProps as any);

    if (prevKeys.length !== nextKeys.length) {
      return false;
    }

    for (const key of prevKeys) {
      const prevValue = (prevProps as any)[key];
      const nextValue = (nextProps as any)[key];

      if (key === 'children' && !compareChildren) {
        continue; // Skip children comparison if not required
      }

      if (deepCompare && typeof prevValue === 'object' && typeof nextValue === 'object') {
        if (!deepEqual(prevValue, nextValue)) {
          return false;
        }
      } else if (prevValue !== nextValue) {
        return false;
      }
    }

    return true;
  });
}

// Deep equality check for objects
function deepEqual(obj1: any, obj2: any): boolean {
  if (obj1 === obj2) return true;
  
  if (obj1 == null || obj2 == null) return obj1 === obj2;
  
  if (typeof obj1 !== typeof obj2) return false;
  
  if (typeof obj1 !== 'object') return obj1 === obj2;
  
  if (Array.isArray(obj1) !== Array.isArray(obj2)) return false;
  
  const keys1 = Object.keys(obj1);
  const keys2 = Object.keys(obj2);
  
  if (keys1.length !== keys2.length) return false;
  
  for (const key of keys1) {
    if (!keys2.includes(key)) return false;
    if (!deepEqual(obj1[key], obj2[key])) return false;
  }
  
  return true;
}

// Render scheduling utilities
export class RenderScheduler {
  private static instance: RenderScheduler;
  private scheduledCallbacks = new Set<() => void>();
  private isScheduled = false;
  private priorityQueue: Array<{ callback: () => void; priority: number }> = [];

  static getInstance(): RenderScheduler {
    if (!this.instance) {
      this.instance = new RenderScheduler();
    }
    return this.instance;
  }

  /**
   * Schedule a callback to run in the next frame
   */
  schedule(callback: () => void, priority: number = 0): void {
    this.priorityQueue.push({ callback, priority });
    this.priorityQueue.sort((a, b) => b.priority - a.priority);
    
    if (!this.isScheduled) {
      this.isScheduled = true;
      requestAnimationFrame(() => this.flush());
    }
  }

  /**
   * Schedule a callback to run when the main thread is idle
   */
  scheduleIdle(callback: () => void): void {
    if ('requestIdleCallback' in window) {
      (window as any).requestIdleCallback(callback, { timeout: 1000 });
    } else {
      // Fallback for browsers without requestIdleCallback
      setTimeout(callback, 0);
    }
  }

  /**
   * Flush all scheduled callbacks
   */
  private flush(): void {
    const startTime = performance.now();
    const timeSlice = 5; // 5ms time slice
    
    while (this.priorityQueue.length > 0 && (performance.now() - startTime) < timeSlice) {
      const { callback } = this.priorityQueue.shift()!;
      try {
        callback();
      } catch (error) {
        console.error('Scheduled callback error:', error);
      }
    }
    
    if (this.priorityQueue.length > 0) {
      // More work to do, schedule next frame
      requestAnimationFrame(() => this.flush());
    } else {
      this.isScheduled = false;
    }
  }

  /**
   * Cancel all scheduled callbacks
   */
  cancelAll(): void {
    this.priorityQueue = [];
    this.scheduledCallbacks.clear();
    this.isScheduled = false;
  }
}

// Virtual scrolling optimization
export interface VirtualScrollConfig {
  itemHeight: number;
  overscanCount: number;
  scrollingDelay: number;
  enableSmoothScrolling: boolean;
}

export function useVirtualScrollOptimization(
  itemCount: number,
  containerHeight: number,
  config: VirtualScrollConfig
) {
  const { itemHeight, overscanCount, scrollingDelay, enableSmoothScrolling } = config;
  
  const [scrollTop, setScrollTop] = React.useState(0);
  const [isScrolling, setIsScrolling] = React.useState(false);
  const scrollTimeoutRef = useRef<NodeJS.Timeout>();

  const visibleRange = useMemo(() => {
    const startIndex = Math.floor(scrollTop / itemHeight);
    const endIndex = Math.min(
      itemCount - 1,
      Math.floor((scrollTop + containerHeight) / itemHeight)
    );

    return {
      startIndex: Math.max(0, startIndex - overscanCount),
      endIndex: Math.min(itemCount - 1, endIndex + overscanCount),
      visibleStartIndex: startIndex,
      visibleEndIndex: endIndex,
    };
  }, [scrollTop, containerHeight, itemHeight, itemCount, overscanCount]);

  const handleScroll = useCallback((event: React.UIEvent<HTMLDivElement>) => {
    const newScrollTop = event.currentTarget.scrollTop;
    setScrollTop(newScrollTop);
    
    if (!isScrolling) {
      setIsScrolling(true);
    }
    
    // Clear existing timeout
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
    }
    
    // Set new timeout
    scrollTimeoutRef.current = setTimeout(() => {
      setIsScrolling(false);
    }, scrollingDelay);
  }, [isScrolling, scrollingDelay]);

  const scrollToIndex = useCallback((index: number, align: 'start' | 'center' | 'end' = 'start') => {
    let targetScrollTop: number;
    
    switch (align) {
      case 'center':
        targetScrollTop = index * itemHeight - containerHeight / 2 + itemHeight / 2;
        break;
      case 'end':
        targetScrollTop = index * itemHeight - containerHeight + itemHeight;
        break;
      default:
        targetScrollTop = index * itemHeight;
    }
    
    targetScrollTop = Math.max(0, Math.min(targetScrollTop, (itemCount * itemHeight) - containerHeight));
    
    if (enableSmoothScrolling) {
      // Smooth scroll implementation
      const startScrollTop = scrollTop;
      const distance = targetScrollTop - startScrollTop;
      const duration = 300; // ms
      const startTime = performance.now();
      
      const animateScroll = (currentTime: number) => {
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / duration, 1);
        
        // Easing function (ease-out)
        const easeOut = 1 - Math.pow(1 - progress, 3);
        const currentScrollTop = startScrollTop + distance * easeOut;
        
        setScrollTop(currentScrollTop);
        
        if (progress < 1) {
          requestAnimationFrame(animateScroll);
        }
      };
      
      requestAnimationFrame(animateScroll);
    } else {
      setScrollTop(targetScrollTop);
    }
  }, [scrollTop, itemHeight, containerHeight, itemCount, enableSmoothScrolling]);

  return {
    visibleRange,
    scrollTop,
    isScrolling,
    handleScroll,
    scrollToIndex,
    totalHeight: itemCount * itemHeight,
  };
}

// Tree-specific optimization hooks
export function useTreeNodeOptimization(node: AssetHierarchy, isVisible: boolean) {
  // Only compute expensive operations when node is visible
  const computedProps = useMemo(() => {
    if (!isVisible) return null;
    
    return {
      hasChildren: node.children.length > 0,
      childCount: node.children.length,
      depth: 0, // This would be calculated based on parent chain
      isFolder: node.asset_type === 'Folder',
    };
  }, [node, isVisible]);

  // Memoize event handlers
  const handleClick = useCallback((event: React.MouseEvent) => {
    event.stopPropagation();
    // Handle click logic
  }, []);

  const handleDoubleClick = useCallback((event: React.MouseEvent) => {
    event.stopPropagation();
    // Handle double click logic
  }, []);

  return {
    computedProps,
    handlers: {
      onClick: handleClick,
      onDoubleClick: handleDoubleClick,
    },
  };
}

// Performance monitoring for renders
export function useRenderPerformance(componentName: string) {
  const renderStartTime = useRef<number>(0);
  const renderCount = useRef<number>(0);

  useEffect(() => {
    renderStartTime.current = performance.now();
    renderCount.current++;
  });

  useEffect(() => {
    const renderTime = performance.now() - renderStartTime.current;
    
    if (process.env.NODE_ENV === 'development') {
      if (renderTime > 16) { // More than one frame at 60fps
        console.warn(`Slow render detected in ${componentName}: ${renderTime.toFixed(2)}ms`);
      }
    }
    
    // Report to performance monitoring system
    if (renderCount.current % 10 === 0) { // Every 10th render
      console.debug(`${componentName} average render time: ${renderTime.toFixed(2)}ms`);
    }
  });

  return {
    renderCount: renderCount.current,
  };
}

// Intersection Observer for lazy loading
export function useIntersectionObserver(
  callback: (isIntersecting: boolean) => void,
  options: IntersectionObserverInit = {}
) {
  const targetRef = useRef<HTMLElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    const target = targetRef.current;
    if (!target) return;

    observerRef.current = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        callback(entry.isIntersecting);
      },
      {
        threshold: 0.1,
        rootMargin: '50px',
        ...options,
      }
    );

    observerRef.current.observe(target);

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [callback, options]);

  return targetRef;
}

// Batch DOM updates
export function useBatchedUpdates() {
  const pendingUpdates = useRef<(() => void)[]>([]);
  const isScheduled = useRef(false);

  const scheduleUpdate = useCallback((update: () => void) => {
    pendingUpdates.current.push(update);
    
    if (!isScheduled.current) {
      isScheduled.current = true;
      requestAnimationFrame(() => {
        const updates = pendingUpdates.current.splice(0);
        updates.forEach(update => update());
        isScheduled.current = false;
      });
    }
  }, []);

  return { scheduleUpdate };
}

// Render optimization utilities
export const RenderOptimization = {
  scheduler: RenderScheduler.getInstance(),
  
  // Create a render-optimized tree node
  createOptimizedNode: createMemoizedTreeNode,
  
  // Debounce heavy operations
  debounce: <T extends (...args: any[]) => any>(
    func: T,
    delay: number
  ): ((...args: Parameters<T>) => void) => {
    let timeoutId: NodeJS.Timeout;
    return (...args: Parameters<T>) => {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => func(...args), delay);
    };
  },
  
  // Throttle frequent operations
  throttle: <T extends (...args: any[]) => any>(
    func: T,
    limit: number
  ): ((...args: Parameters<T>) => void) => {
    let inThrottle: boolean;
    return (...args: Parameters<T>) => {
      if (!inThrottle) {
        func(...args);
        inThrottle = true;
        setTimeout(() => (inThrottle = false), limit);
      }
    };
  },
  
  // Check if component should update
  shouldComponentUpdate: (prevProps: any, nextProps: any, keys: string[] = []): boolean => {
    if (keys.length === 0) {
      keys = Object.keys(nextProps);
    }
    
    return keys.some(key => prevProps[key] !== nextProps[key]);
  },
};