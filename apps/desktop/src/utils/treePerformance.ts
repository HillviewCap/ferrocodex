/**
 * Tree Performance Utilities
 * 
 * Provides utilities for monitoring and optimizing tree performance,
 * including render time tracking, memory usage monitoring, and 
 * performance recommendations.
 */

export interface TreePerformanceConfig {
  maxRenderTime: number; // milliseconds
  maxMemoryUsage: number; // bytes
  virtualizationThreshold: number; // item count
  lazyLoadingThreshold: number; // depth
  debounceThreshold: number; // item count
}

export interface PerformanceMetrics {
  renderTime: number;
  memoryUsage: number;
  cacheHitRate: number;
  searchTime: number;
  lastMeasurement: Date;
  frameRate: number;
}

export interface PerformanceRecommendation {
  type: 'virtualization' | 'lazy-loading' | 'debouncing' | 'caching' | 'memory-optimization';
  priority: 'low' | 'medium' | 'high' | 'critical';
  description: string;
  impact: string;
  implementation: string;
}

export class TreePerformanceMonitor {
  private config: TreePerformanceConfig;
  private metrics: PerformanceMetrics;
  private renderTimeHistory: number[] = [];
  private frameTimeHistory: number[] = [];
  private memoryCheckInterval?: NodeJS.Timeout;
  private observers: ((metrics: PerformanceMetrics) => void)[] = [];

  constructor(config: Partial<TreePerformanceConfig> = {}) {
    this.config = {
      maxRenderTime: 16, // 60fps target
      maxMemoryUsage: 50 * 1024 * 1024, // 50MB
      virtualizationThreshold: 100,
      lazyLoadingThreshold: 10,
      debounceThreshold: 500,
      ...config,
    };

    this.metrics = {
      renderTime: 0,
      memoryUsage: 0,
      cacheHitRate: 0,
      searchTime: 0,
      lastMeasurement: new Date(),
      frameRate: 60,
    };

    this.startMemoryMonitoring();
  }

  /**
   * Measure render time for a tree operation
   */
  measureRenderTime<T>(operation: () => T): T {
    const startTime = performance.now();
    const result = operation();
    const renderTime = performance.now() - startTime;
    
    this.updateRenderTime(renderTime);
    return result;
  }

  /**
   * Measure async render time
   */
  async measureAsyncRenderTime<T>(operation: () => Promise<T>): Promise<T> {
    const startTime = performance.now();
    const result = await operation();
    const renderTime = performance.now() - startTime;
    
    this.updateRenderTime(renderTime);
    return result;
  }

  /**
   * Track frame rate using requestAnimationFrame
   */
  trackFrameRate(): () => void {
    let frameCount = 0;
    let lastTime = performance.now();
    let animationId: number;

    const measure = () => {
      frameCount++;
      const currentTime = performance.now();
      
      if (currentTime - lastTime >= 1000) {
        const frameRate = frameCount * 1000 / (currentTime - lastTime);
        this.updateFrameRate(frameRate);
        
        frameCount = 0;
        lastTime = currentTime;
      }
      
      animationId = requestAnimationFrame(measure);
    };

    animationId = requestAnimationFrame(measure);
    
    return () => {
      cancelAnimationFrame(animationId);
    };
  }

  /**
   * Measure memory usage
   */
  measureMemoryUsage(): number {
    if ('memory' in performance) {
      // @ts-ignore - Chrome-specific API
      const memory = (performance as any).memory;
      return memory.usedJSHeapSize;
    }
    return 0;
  }

  /**
   * Generate performance recommendations
   */
  getRecommendations(treeSize: number, maxDepth: number): PerformanceRecommendation[] {
    const recommendations: PerformanceRecommendation[] = [];

    // Virtualization recommendation
    if (treeSize > this.config.virtualizationThreshold) {
      recommendations.push({
        type: 'virtualization',
        priority: treeSize > 1000 ? 'high' : 'medium',
        description: `Tree has ${treeSize} items, virtualization recommended`,
        impact: 'Significantly improves render performance for large trees',
        implementation: 'Use react-window or similar virtualization library',
      });
    }

    // Lazy loading recommendation
    if (maxDepth > this.config.lazyLoadingThreshold) {
      recommendations.push({
        type: 'lazy-loading',
        priority: maxDepth > 20 ? 'high' : 'medium',
        description: `Tree depth is ${maxDepth}, lazy loading recommended`,
        impact: 'Reduces initial render time and memory usage',
        implementation: 'Load child nodes only when parent is expanded',
      });
    }

    // Debouncing recommendation
    if (treeSize > this.config.debounceThreshold) {
      recommendations.push({
        type: 'debouncing',
        priority: this.metrics.searchTime > 100 ? 'high' : 'medium',
        description: 'Search operations should be debounced for large trees',
        impact: 'Prevents excessive re-renders during typing',
        implementation: 'Use debounced search with 300ms delay',
      });
    }

    // Performance warnings
    if (this.metrics.renderTime > this.config.maxRenderTime) {
      recommendations.push({
        type: 'memory-optimization',
        priority: 'critical',
        description: `Render time ${this.metrics.renderTime.toFixed(2)}ms exceeds target ${this.config.maxRenderTime}ms`,
        impact: 'Poor user experience due to slow rendering',
        implementation: 'Implement virtualization and reduce component complexity',
      });
    }

    if (this.metrics.memoryUsage > this.config.maxMemoryUsage) {
      recommendations.push({
        type: 'memory-optimization',
        priority: 'high',
        description: `Memory usage ${Math.round(this.metrics.memoryUsage / 1024 / 1024)}MB exceeds limit`,
        impact: 'High memory usage may cause performance issues',
        implementation: 'Implement proper cleanup and reduce cache size',
      });
    }

    if (this.metrics.frameRate < 30) {
      recommendations.push({
        type: 'virtualization',
        priority: 'critical',
        description: `Frame rate ${this.metrics.frameRate.toFixed(1)}fps is below acceptable threshold`,
        impact: 'Janky scrolling and poor user experience',
        implementation: 'Optimize render pipeline and reduce DOM nodes',
      });
    }

    return recommendations;
  }

  /**
   * Get performance report
   */
  getPerformanceReport(): {
    metrics: PerformanceMetrics;
    trends: {
      renderTimetrend: 'improving' | 'stable' | 'degrading';
      frameRateTrend: 'improving' | 'stable' | 'degrading';
    };
    isOptimal: boolean;
  } {
    const recentRenderTimes = this.renderTimeHistory.slice(-10);
    const recentFrameRates = this.frameTimeHistory.slice(-10);
    
    const renderTimeTrend = this.calculateTrend(recentRenderTimes);
    const frameRateTrend = this.calculateTrend(recentFrameRates.map(f => -f)); // Invert for proper trend
    
    return {
      metrics: { ...this.metrics },
      trends: {
        renderTimetrend: renderTimeTrend,
        frameRateTrend: frameRateTrend,
      },
      isOptimal: 
        this.metrics.renderTime < this.config.maxRenderTime &&
        this.metrics.frameRate > 45 &&
        this.metrics.memoryUsage < this.config.maxMemoryUsage,
    };
  }

  /**
   * Subscribe to performance updates
   */
  subscribe(callback: (metrics: PerformanceMetrics) => void): () => void {
    this.observers.push(callback);
    return () => {
      const index = this.observers.indexOf(callback);
      if (index > -1) {
        this.observers.splice(index, 1);
      }
    };
  }

  /**
   * Cleanup resources
   */
  destroy(): void {
    if (this.memoryCheckInterval) {
      clearInterval(this.memoryCheckInterval);
    }
    this.observers = [];
  }

  private updateRenderTime(renderTime: number): void {
    this.metrics.renderTime = renderTime;
    this.metrics.lastMeasurement = new Date();
    
    this.renderTimeHistory.push(renderTime);
    if (this.renderTimeHistory.length > 100) {
      this.renderTimeHistory.shift();
    }
    
    this.notifyObservers();
  }

  private updateFrameRate(frameRate: number): void {
    this.metrics.frameRate = frameRate;
    
    this.frameTimeHistory.push(frameRate);
    if (this.frameTimeHistory.length > 60) {
      this.frameTimeHistory.shift();
    }
    
    this.notifyObservers();
  }

  private startMemoryMonitoring(): void {
    this.memoryCheckInterval = setInterval(() => {
      this.metrics.memoryUsage = this.measureMemoryUsage();
      this.notifyObservers();
    }, 5000); // Check every 5 seconds
  }

  private calculateTrend(values: number[]): 'improving' | 'stable' | 'degrading' {
    if (values.length < 3) return 'stable';
    
    const recent = values.slice(-3);
    const older = values.slice(-6, -3);
    
    if (older.length === 0) return 'stable';
    
    const recentAvg = recent.reduce((a, b) => a + b, 0) / recent.length;
    const olderAvg = older.reduce((a, b) => a + b, 0) / older.length;
    
    const threshold = olderAvg * 0.1; // 10% threshold
    
    if (recentAvg < olderAvg - threshold) return 'improving';
    if (recentAvg > olderAvg + threshold) return 'degrading';
    return 'stable';
  }

  private notifyObservers(): void {
    this.observers.forEach(callback => {
      try {
        callback(this.metrics);
      } catch (error) {
        console.error('Error in performance observer:', error);
      }
    });
  }
}

/**
 * Cache implementation with LRU eviction
 */
export class TreeCache<K, V> {
  private cache = new Map<K, V>();
  private accessOrder = new Map<K, number>();
  private maxSize: number;
  private hitCount = 0;
  private missCount = 0;

  constructor(maxSize: number = 500) {
    this.maxSize = maxSize;
  }

  get(key: K): V | undefined {
    const value = this.cache.get(key);
    if (value !== undefined) {
      this.hitCount++;
      this.accessOrder.set(key, Date.now());
      return value;
    }
    this.missCount++;
    return undefined;
  }

  set(key: K, value: V): void {
    if (this.cache.size >= this.maxSize && !this.cache.has(key)) {
      this.evictLeastRecentlyUsed();
    }
    
    this.cache.set(key, value);
    this.accessOrder.set(key, Date.now());
  }

  has(key: K): boolean {
    return this.cache.has(key);
  }

  delete(key: K): boolean {
    this.accessOrder.delete(key);
    return this.cache.delete(key);
  }

  clear(): void {
    this.cache.clear();
    this.accessOrder.clear();
    this.hitCount = 0;
    this.missCount = 0;
  }

  getHitRate(): number {
    const total = this.hitCount + this.missCount;
    return total === 0 ? 0 : this.hitCount / total;
  }

  getStats(): {
    size: number;
    hitRate: number;
    hitCount: number;
    missCount: number;
  } {
    return {
      size: this.cache.size,
      hitRate: this.getHitRate(),
      hitCount: this.hitCount,
      missCount: this.missCount,
    };
  }

  private evictLeastRecentlyUsed(): void {
    let oldestKey: K | undefined;
    let oldestTime = Infinity;
    
    for (const [key, time] of this.accessOrder) {
      if (time < oldestTime) {
        oldestTime = time;
        oldestKey = key;
      }
    }
    
    if (oldestKey !== undefined) {
      this.delete(oldestKey);
    }
  }
}

/**
 * Debounce utility with configurable delay
 */
export function debounce<T extends (...args: any[]) => any>(
  func: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: NodeJS.Timeout;
  
  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => func(...args), delay);
  };
}

/**
 * Throttle utility for rate limiting
 */
export function throttle<T extends (...args: any[]) => any>(
  func: T,
  delay: number
): (...args: Parameters<T>) => void {
  let lastCall = 0;
  let timeoutId: NodeJS.Timeout;
  
  return (...args: Parameters<T>) => {
    const now = Date.now();
    
    if (now - lastCall >= delay) {
      lastCall = now;
      func(...args);
    } else {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        lastCall = Date.now();
        func(...args);
      }, delay - (now - lastCall));
    }
  };
}