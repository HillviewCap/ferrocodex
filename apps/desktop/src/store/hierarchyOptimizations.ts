// Performance optimizations for large hierarchies

export interface PerformanceMetrics {
  treeSize: number;
  maxDepth: number;
  renderTime: number;
  lastOptimization: Date;
}

export class HierarchyPerformanceOptimizer {
  private static readonly LARGE_TREE_THRESHOLD = 1000;
  private static readonly DEEP_TREE_THRESHOLD = 20;
  private static readonly VIRTUALIZATION_THRESHOLD = 100;

  static shouldUseVirtualization(treeSize: number): boolean {
    return treeSize > this.VIRTUALIZATION_THRESHOLD;
  }

  static shouldUseLazyLoading(maxDepth: number): boolean {
    return maxDepth > this.DEEP_TREE_THRESHOLD;
  }

  static shouldUseDebouncing(treeSize: number): boolean {
    return treeSize > this.LARGE_TREE_THRESHOLD;
  }

  static calculateOptimalBatchSize(treeSize: number): number {
    if (treeSize < 100) return treeSize;
    if (treeSize < 500) return 50;
    if (treeSize < 1000) return 100;
    return 200;
  }

  static getPerformanceSettings(metrics: PerformanceMetrics) {
    return {
      useVirtualization: this.shouldUseVirtualization(metrics.treeSize),
      useLazyLoading: this.shouldUseLazyLoading(metrics.maxDepth),
      useDebouncing: this.shouldUseDebouncing(metrics.treeSize),
      batchSize: this.calculateOptimalBatchSize(metrics.treeSize),
      enableMemoization: metrics.treeSize > 50,
      enableSearch: metrics.treeSize > 20,
    };
  }
}

// Memoization utilities for tree nodes
export const memoizeTreeNode = <T>(fn: (node: T) => any) => {
  const cache = new Map<string, any>();
  
  return (node: T & { id: number; updated_at: string }) => {
    const key = `${node.id}-${node.updated_at}`;
    
    if (cache.has(key)) {
      return cache.get(key);
    }
    
    const result = fn(node);
    
    // Limit cache size to prevent memory leaks
    if (cache.size > 500) {
      const firstKey = cache.keys().next().value;
      cache.delete(firstKey);
    }
    
    cache.set(key, result);
    return result;
  };
};

// Debounced search for large hierarchies
export const createDebouncedSearch = (delay: number = 300) => {
  let timeoutId: number | undefined;
  
  return (searchFn: (term: string) => void) => {
    return (searchTerm: string) => {
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId);
      }
      timeoutId = window.setTimeout(() => {
        searchFn(searchTerm);
      }, delay);
    };
  };
};

// Tree flattening for virtualization
export interface FlatTreeNode {
  id: number;
  level: number;
  isExpanded: boolean;
  hasChildren: boolean;
  data: any;
}

export const flattenTree = (
  nodes: any[], 
  expandedKeys: Set<string | number>,
  level: number = 0
): FlatTreeNode[] => {
  const result: FlatTreeNode[] = [];
  
  for (const node of nodes) {
    const isExpanded = expandedKeys.has(node.id);
    const hasChildren = node.children && node.children.length > 0;
    
    result.push({
      id: node.id,
      level,
      isExpanded,
      hasChildren,
      data: node,
    });
    
    if (isExpanded && hasChildren) {
      result.push(...flattenTree(node.children, expandedKeys, level + 1));
    }
  }
  
  return result;
};

// Batch processing for large operations
export class BatchProcessor<T> {
  private queue: T[] = [];
  private processing = false;
  private batchSize: number;
  private processFn: (batch: T[]) => Promise<void>;

  constructor(batchSize: number, processFn: (batch: T[]) => Promise<void>) {
    this.batchSize = batchSize;
    this.processFn = processFn;
  }

  add(item: T): void {
    this.queue.push(item);
    this.scheduleProcessing();
  }

  addBatch(items: T[]): void {
    this.queue.push(...items);
    this.scheduleProcessing();
  }

  private scheduleProcessing(): void {
    if (!this.processing && this.queue.length > 0) {
      setTimeout(() => this.processBatch(), 0);
    }
  }

  private async processBatch(): Promise<void> {
    if (this.processing || this.queue.length === 0) return;
    
    this.processing = true;
    
    try {
      while (this.queue.length > 0) {
        const batch = this.queue.splice(0, this.batchSize);
        await this.processFn(batch);
        
        // Allow other tasks to run
        await new Promise(resolve => setTimeout(resolve, 0));
      }
    } finally {
      this.processing = false;
      
      // Process any items added while processing
      if (this.queue.length > 0) {
        this.scheduleProcessing();
      }
    }
  }

  get pendingCount(): number {
    return this.queue.length;
  }

  get isProcessing(): boolean {
    return this.processing;
  }
}