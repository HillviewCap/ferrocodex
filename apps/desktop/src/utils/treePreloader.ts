/**
 * Tree Preloader Utilities
 * 
 * Intelligent preloading system for tree nodes to improve user experience
 * by anticipating user navigation patterns and preloading likely-to-be-accessed nodes.
 */

import { AssetHierarchy, AssetInfo } from '../types/assets';
import { invoke } from '@tauri-apps/api/core';

export interface PreloadConfig {
  maxConcurrentRequests: number;
  preloadDepth: number;
  priorityThreshold: number;
  cacheSize: number;
  enabled: boolean;
  adaptivePriority: boolean;
}

export interface PreloadTask {
  nodeId: number;
  priority: number;
  depth: number;
  parentId: number | null;
  estimatedSize: number;
  type: 'node' | 'children' | 'metadata' | 'path';
  scheduledAt: Date;
  attempts: number;
}

export interface PreloadMetrics {
  totalPreloaded: number;
  successfulPreloads: number;
  failedPreloads: number;
  averageLoadTime: number;
  cacheHitImprovement: number;
  userBenefit: number; // How often preloaded data was actually used
}

export interface UserNavigationPattern {
  nodeId: number;
  accessCount: number;
  lastAccessed: Date;
  averageSessionTime: number;
  navigationPath: number[];
  childrenAccessRate: number;
  searchFrequency: number;
}

export class TreePreloader {
  private config: PreloadConfig;
  private preloadQueue: PreloadTask[] = [];
  private activeRequests = new Set<number>();
  private preloadedData = new Map<number, { data: any; preloadedAt: Date }>();
  private navigationPatterns = new Map<number, UserNavigationPattern>();
  private metrics: PreloadMetrics;
  private isRunning = false;
  private worker?: Worker;
  
  constructor(config: Partial<PreloadConfig> = {}) {
    this.config = {
      maxConcurrentRequests: 3,
      preloadDepth: 2,
      priorityThreshold: 0.5,
      cacheSize: 100,
      enabled: true,
      adaptivePriority: true,
      ...config,
    };
    
    this.metrics = {
      totalPreloaded: 0,
      successfulPreloads: 0,
      failedPreloads: 0,
      averageLoadTime: 0,
      cacheHitImprovement: 0,
      userBenefit: 0,
    };
    
    if (this.config.enabled) {
      this.startPreloading();
    }
  }

  /**
   * Start the preloading process
   */
  startPreloading(): void {
    if (this.isRunning) return;
    
    this.isRunning = true;
    this.processQueue();
    
    // Set up periodic queue processing
    setInterval(() => {
      if (this.isRunning && this.preloadQueue.length > 0) {
        this.processQueue();
      }
    }, 1000);
    
    // Adaptive priority adjustment
    if (this.config.adaptivePriority) {
      setInterval(() => {
        this.adjustPriorities();
      }, 10000); // Every 10 seconds
    }
  }

  /**
   * Stop the preloading process
   */
  stopPreloading(): void {
    this.isRunning = false;
    this.activeRequests.clear();
    this.preloadQueue = [];
  }

  /**
   * Track user navigation to build patterns
   */
  trackNavigation(nodeId: number, navigationPath: number[]): void {
    const existing = this.navigationPatterns.get(nodeId);
    const now = new Date();
    
    if (existing) {
      existing.accessCount++;
      existing.lastAccessed = now;
      existing.navigationPath = navigationPath;
    } else {
      this.navigationPatterns.set(nodeId, {
        nodeId,
        accessCount: 1,
        lastAccessed: now,
        averageSessionTime: 0,
        navigationPath,
        childrenAccessRate: 0,
        searchFrequency: 0,
      });
    }
    
    // Schedule preloading for likely next nodes
    this.scheduleSmartPreload(nodeId, navigationPath);
  }

  /**
   * Schedule preloading based on current user activity
   */
  scheduleSmartPreload(currentNodeId: number, navigationPath: number[]): void {
    if (!this.config.enabled) return;
    
    const tasks: PreloadTask[] = [];
    
    // 1. Preload children of current node (high priority)
    this.scheduleChildrenPreload(currentNodeId, 0.9, tasks);
    
    // 2. Preload siblings (medium priority)
    if (navigationPath.length > 0) {
      const parentId = navigationPath[navigationPath.length - 1];
      this.scheduleSiblingPreload(currentNodeId, parentId, 0.6, tasks);
    }
    
    // 3. Preload based on navigation patterns (adaptive priority)
    this.schedulePatternBasedPreload(currentNodeId, tasks);
    
    // 4. Preload frequently accessed nodes (low priority)
    this.scheduleFrequentNodesPreload(0.3, tasks);
    
    // Add tasks to queue
    tasks.forEach(task => this.addPreloadTask(task));
  }

  /**
   * Schedule preloading of node children
   */
  private scheduleChildrenPreload(nodeId: number, priority: number, tasks: PreloadTask[]): void {
    if (this.isPreloaded(nodeId) || this.activeRequests.has(nodeId)) return;
    
    tasks.push({
      nodeId,
      priority,
      depth: 1,
      parentId: null,
      estimatedSize: 1024, // Rough estimate
      type: 'children',
      scheduledAt: new Date(),
      attempts: 0,
    });
  }

  /**
   * Schedule preloading of sibling nodes
   */
  private scheduleSiblingPreload(currentNodeId: number, parentId: number, priority: number, tasks: PreloadTask[]): void {
    // This would require fetching sibling information
    // For now, we'll schedule the parent's children
    if (!this.isPreloaded(parentId) && !this.activeRequests.has(parentId)) {
      tasks.push({
        nodeId: parentId,
        priority: priority * 0.8,
        depth: 1,
        parentId: null,
        estimatedSize: 2048,
        type: 'children',
        scheduledAt: new Date(),
        attempts: 0,
      });
    }
  }

  /**
   * Schedule preloading based on user navigation patterns
   */
  private schedulePatternBasedPreload(currentNodeId: number, tasks: PreloadTask[]): void {
    const pattern = this.navigationPatterns.get(currentNodeId);
    if (!pattern) return;
    
    // Predict next likely nodes based on navigation patterns
    const likelyNextNodes = this.predictNextNodes(currentNodeId);
    
    likelyNextNodes.forEach(({ nodeId, probability }) => {
      if (probability > this.config.priorityThreshold && 
          !this.isPreloaded(nodeId) && 
          !this.activeRequests.has(nodeId)) {
        
        tasks.push({
          nodeId,
          priority: probability,
          depth: 1,
          parentId: currentNodeId,
          estimatedSize: 1024,
          type: 'node',
          scheduledAt: new Date(),
          attempts: 0,
        });
      }
    });
  }

  /**
   * Schedule preloading of frequently accessed nodes
   */
  private scheduleFrequentNodesPreload(basePriority: number, tasks: PreloadTask[]): void {
    const frequentNodes = Array.from(this.navigationPatterns.values())
      .filter(pattern => pattern.accessCount > 3)
      .sort((a, b) => b.accessCount - a.accessCount)
      .slice(0, 10); // Top 10 most accessed
    
    frequentNodes.forEach(pattern => {
      if (!this.isPreloaded(pattern.nodeId) && !this.activeRequests.has(pattern.nodeId)) {
        const priority = basePriority * (pattern.accessCount / 10); // Scale by access count
        
        tasks.push({
          nodeId: pattern.nodeId,
          priority,
          depth: 1,
          parentId: null,
          estimatedSize: 1024,
          type: 'node',
          scheduledAt: new Date(),
          attempts: 0,
        });
      }
    });
  }

  /**
   * Add a preload task to the queue
   */
  private addPreloadTask(task: PreloadTask): void {
    // Check if task already exists
    const existingIndex = this.preloadQueue.findIndex(t => 
      t.nodeId === task.nodeId && t.type === task.type
    );
    
    if (existingIndex >= 0) {
      // Update priority if higher
      if (this.preloadQueue[existingIndex].priority < task.priority) {
        this.preloadQueue[existingIndex] = task;
      }
    } else {
      this.preloadQueue.push(task);
    }
    
    // Sort queue by priority
    this.preloadQueue.sort((a, b) => b.priority - a.priority);
    
    // Limit queue size
    if (this.preloadQueue.length > this.config.cacheSize) {
      this.preloadQueue = this.preloadQueue.slice(0, this.config.cacheSize);
    }
  }

  /**
   * Process the preload queue
   */
  private async processQueue(): Promise<void> {
    if (!this.isRunning || this.activeRequests.size >= this.config.maxConcurrentRequests) {
      return;
    }
    
    // Get next high-priority task
    const task = this.preloadQueue.find(t => 
      !this.activeRequests.has(t.nodeId) && 
      t.priority > this.config.priorityThreshold
    );
    
    if (!task) return;
    
    // Remove task from queue
    const taskIndex = this.preloadQueue.indexOf(task);
    this.preloadQueue.splice(taskIndex, 1);
    
    // Execute preload
    await this.executePreloadTask(task);
  }

  /**
   * Execute a single preload task
   */
  private async executePreloadTask(task: PreloadTask): Promise<void> {
    this.activeRequests.add(task.nodeId);
    const startTime = performance.now();
    
    try {
      let data: any;
      
      switch (task.type) {
        case 'children':
          data = await this.preloadNodeChildren(task.nodeId);
          break;
        case 'node':
          data = await this.preloadNodeData(task.nodeId);
          break;
        case 'metadata':
          data = await this.preloadNodeMetadata(task.nodeId);
          break;
        case 'path':
          data = await this.preloadNodePath(task.nodeId);
          break;
        default:
          throw new Error(`Unknown task type: ${task.type}`);
      }
      
      // Store preloaded data
      this.preloadedData.set(task.nodeId, {
        data,
        preloadedAt: new Date(),
      });
      
      // Update metrics
      this.metrics.successfulPreloads++;
      this.metrics.totalPreloaded++;
      
      const loadTime = performance.now() - startTime;
      this.updateAverageLoadTime(loadTime);
      
    } catch (error) {
      console.debug(`Preload failed for node ${task.nodeId}:`, error);
      
      // Retry with exponential backoff
      task.attempts++;
      if (task.attempts < 3) {
        task.priority *= 0.5; // Reduce priority
        setTimeout(() => {
          this.addPreloadTask(task);
        }, Math.pow(2, task.attempts) * 1000);
      }
      
      this.metrics.failedPreloads++;
      this.metrics.totalPreloaded++;
    } finally {
      this.activeRequests.delete(task.nodeId);
    }
  }

  /**
   * Preload node children data
   */
  private async preloadNodeChildren(nodeId: number): Promise<AssetHierarchy[]> {
    // This would call the backend to get children
    // For now, we'll simulate with a placeholder
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve([]); // Placeholder
      }, 100);
    });
  }

  /**
   * Preload node data
   */
  private async preloadNodeData(nodeId: number): Promise<AssetHierarchy> {
    // This would call the backend to get node data
    return new Promise((resolve, reject) => {
      setTimeout(() => {
        // Simulate some failures
        if (Math.random() < 0.1) {
          reject(new Error('Simulated preload failure'));
        } else {
          resolve({} as AssetHierarchy); // Placeholder
        }
      }, 50);
    });
  }

  /**
   * Preload node metadata
   */
  private async preloadNodeMetadata(nodeId: number): Promise<any> {
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve({ nodeId, metadata: 'placeholder' });
      }, 30);
    });
  }

  /**
   * Preload node path
   */
  private async preloadNodePath(nodeId: number): Promise<AssetInfo[]> {
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve([]); // Placeholder
      }, 75);
    });
  }

  /**
   * Check if data is already preloaded
   */
  private isPreloaded(nodeId: number): boolean {
    return this.preloadedData.has(nodeId);
  }

  /**
   * Get preloaded data if available
   */
  getPreloadedData(nodeId: number): any | null {
    const entry = this.preloadedData.get(nodeId);
    if (!entry) return null;
    
    // Check if data is still fresh (within 5 minutes)
    const age = Date.now() - entry.preloadedAt.getTime();
    if (age > 5 * 60 * 1000) {
      this.preloadedData.delete(nodeId);
      return null;
    }
    
    // Track that preloaded data was used
    this.metrics.userBenefit++;
    
    return entry.data;
  }

  /**
   * Predict next likely nodes based on patterns
   */
  private predictNextNodes(currentNodeId: number): Array<{ nodeId: number; probability: number }> {
    const pattern = this.navigationPatterns.get(currentNodeId);
    if (!pattern) return [];
    
    // Simple prediction based on navigation path
    const predictions: Array<{ nodeId: number; probability: number }> = [];
    
    // Add nodes from typical navigation paths
    pattern.navigationPath.forEach((nodeId, index) => {
      if (nodeId !== currentNodeId) {
        const probability = 0.8 - (index * 0.1); // Higher probability for closer nodes
        predictions.push({ nodeId, probability });
      }
    });
    
    return predictions.filter(p => p.probability > 0.3);
  }

  /**
   * Adjust priorities based on actual usage patterns
   */
  private adjustPriorities(): void {
    // Increase priority threshold if hit rate is low
    const hitRate = this.metrics.userBenefit / Math.max(this.metrics.successfulPreloads, 1);
    
    if (hitRate < 0.3) {
      this.config.priorityThreshold = Math.min(0.8, this.config.priorityThreshold + 0.1);
    } else if (hitRate > 0.7) {
      this.config.priorityThreshold = Math.max(0.3, this.config.priorityThreshold - 0.1);
    }
  }

  /**
   * Update average load time metric
   */
  private updateAverageLoadTime(newTime: number): void {
    const currentAvg = this.metrics.averageLoadTime;
    const count = this.metrics.successfulPreloads;
    
    this.metrics.averageLoadTime = (currentAvg * (count - 1) + newTime) / count;
  }

  /**
   * Get current preloader metrics
   */
  getMetrics(): PreloadMetrics & { queueSize: number; activeRequests: number } {
    return {
      ...this.metrics,
      queueSize: this.preloadQueue.length,
      activeRequests: this.activeRequests.size,
    };
  }

  /**
   * Clear all preloaded data and patterns
   */
  clear(): void {
    this.preloadedData.clear();
    this.navigationPatterns.clear();
    this.preloadQueue = [];
    this.activeRequests.clear();
  }

  /**
   * Cleanup resources
   */
  destroy(): void {
    this.stopPreloading();
    this.clear();
    if (this.worker) {
      this.worker.terminate();
    }
  }
}

// Singleton instance
export const treePreloader = new TreePreloader();