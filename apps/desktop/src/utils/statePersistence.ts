/**
 * State Persistence Utilities
 * 
 * Provides utilities for saving and restoring tree navigation state
 * including expanded nodes, selected assets, view preferences, and
 * navigation history with version migration support.
 */

import { TreeViewState } from '../store/treeState';

export interface PersistedTreeState {
  version: number;
  timestamp: Date;
  expandedKeys: number[];
  selectedAssetId: number | null;
  breadcrumbPath: any[];
  navigationHistory: number[];
  currentPosition: number;
  viewPreferences: {
    viewMode: string;
    sortBy: string;
    sortOrder: string;
    virtualizationEnabled: boolean;
    itemHeight: number;
    compactMode: boolean;
  };
  uiState: {
    sidebarWidth: number;
    showPerformanceMetrics: boolean;
  };
}

export interface MigrationStrategy {
  fromVersion: number;
  toVersion: number;
  migrate: (oldState: any) => any;
}

export class TreeStatePersistenceManager {
  private readonly STORAGE_KEY = 'ferrocodex-tree-state';
  private readonly CURRENT_VERSION = 2;
  private readonly MAX_STORAGE_SIZE = 1024 * 1024; // 1MB limit
  private readonly MAX_HISTORY_SIZE = 100;
  
  private migrationStrategies: MigrationStrategy[] = [
    {
      fromVersion: 1,
      toVersion: 2,
      migrate: (oldState: any) => ({
        ...oldState,
        version: 2,
        uiState: {
          sidebarWidth: oldState.sidebarWidth || 400,
          showPerformanceMetrics: oldState.showPerformanceMetrics || false,
        },
      }),
    },
  ];

  /**
   * Save tree state to localStorage with error handling
   */
  async saveState(state: Partial<TreeViewState>): Promise<void> {
    try {
      const persistedState: PersistedTreeState = {
        version: this.CURRENT_VERSION,
        timestamp: new Date(),
        expandedKeys: Array.from(state.expandedKeys || []),
        selectedAssetId: state.selectedAssetId || null,
        breadcrumbPath: state.breadcrumbPath || [],
        navigationHistory: (state.navigationHistory || []).slice(-this.MAX_HISTORY_SIZE),
        currentPosition: state.currentPosition || -1,
        viewPreferences: {
          viewMode: state.viewMode || 'virtualized',
          sortBy: state.sortBy || 'name',
          sortOrder: state.sortOrder || 'asc',
          virtualizationEnabled: state.virtualizationEnabled ?? true,
          itemHeight: state.itemHeight || 32,
          compactMode: state.compactMode || false,
        },
        uiState: {
          sidebarWidth: state.sidebarWidth || 400,
          showPerformanceMetrics: state.showPerformanceMetrics || false,
        },
      };

      const serializedState = JSON.stringify(persistedState);
      
      // Check storage size limit
      if (serializedState.length > this.MAX_STORAGE_SIZE) {
        console.warn('Tree state exceeds storage limit, trimming...');
        // Trim navigation history and try again
        persistedState.navigationHistory = persistedState.navigationHistory.slice(-20);
        const trimmedState = JSON.stringify(persistedState);
        
        if (trimmedState.length > this.MAX_STORAGE_SIZE) {
          throw new Error('Tree state too large even after trimming');
        }
        
        localStorage.setItem(this.STORAGE_KEY, trimmedState);
      } else {
        localStorage.setItem(this.STORAGE_KEY, serializedState);
      }

      console.debug('Tree state saved successfully');
    } catch (error) {
      console.error('Failed to save tree state:', error);
      throw error;
    }
  }

  /**
   * Load and migrate tree state from localStorage
   */
  async loadState(): Promise<Partial<TreeViewState> | null> {
    try {
      const serializedState = localStorage.getItem(this.STORAGE_KEY);
      if (!serializedState) {
        return null;
      }

      let persistedState = JSON.parse(serializedState) as PersistedTreeState;
      
      // Apply migrations if needed
      persistedState = await this.migrateState(persistedState);
      
      // Validate state structure
      this.validateState(persistedState);
      
      // Convert back to TreeViewState format
      const treeState: Partial<TreeViewState> = {
        expandedKeys: new Set(persistedState.expandedKeys),
        selectedAssetId: persistedState.selectedAssetId,
        breadcrumbPath: persistedState.breadcrumbPath,
        navigationHistory: persistedState.navigationHistory,
        currentPosition: persistedState.currentPosition,
        viewMode: persistedState.viewPreferences.viewMode as TreeViewState['viewMode'],
        sortBy: persistedState.viewPreferences.sortBy as TreeViewState['sortBy'],
        sortOrder: persistedState.viewPreferences.sortOrder as TreeViewState['sortOrder'],
        virtualizationEnabled: persistedState.viewPreferences.virtualizationEnabled,
        itemHeight: persistedState.viewPreferences.itemHeight,
        compactMode: persistedState.viewPreferences.compactMode,
        sidebarWidth: persistedState.uiState.sidebarWidth,
        showPerformanceMetrics: persistedState.uiState.showPerformanceMetrics,
      };

      console.debug('Tree state loaded successfully');
      return treeState;
    } catch (error) {
      console.error('Failed to load tree state:', error);
      // Clear corrupted state
      this.clearState();
      return null;
    }
  }

  /**
   * Clear persisted state
   */
  clearState(): void {
    try {
      localStorage.removeItem(this.STORAGE_KEY);
      console.debug('Tree state cleared');
    } catch (error) {
      console.error('Failed to clear tree state:', error);
    }
  }

  /**
   * Export state for backup or transfer
   */
  async exportState(): Promise<string> {
    const state = await this.loadState();
    if (!state) {
      throw new Error('No state to export');
    }
    
    return JSON.stringify({
      exportedAt: new Date().toISOString(),
      version: this.CURRENT_VERSION,
      state,
    }, null, 2);
  }

  /**
   * Import state from backup
   */
  async importState(exportedData: string): Promise<void> {
    try {
      const importData = JSON.parse(exportedData);
      
      if (!importData.state) {
        throw new Error('Invalid export format');
      }
      
      await this.saveState(importData.state);
      console.debug('Tree state imported successfully');
    } catch (error) {
      console.error('Failed to import tree state:', error);
      throw error;
    }
  }

  /**
   * Get storage statistics
   */
  getStorageInfo(): {
    hasPersistedState: boolean;
    stateSize: number;
    lastSaved?: Date;
    version?: number;
  } {
    try {
      const serializedState = localStorage.getItem(this.STORAGE_KEY);
      if (!serializedState) {
        return { hasPersistedState: false, stateSize: 0 };
      }

      const persistedState = JSON.parse(serializedState);
      
      return {
        hasPersistedState: true,
        stateSize: serializedState.length,
        lastSaved: new Date(persistedState.timestamp),
        version: persistedState.version,
      };
    } catch (error) {
      return { hasPersistedState: false, stateSize: 0 };
    }
  }

  /**
   * Validate state restoration speed
   */
  async measureRestorationSpeed(): Promise<{
    loadTime: number;
    migrationTime: number;
    totalTime: number;
    isOptimal: boolean;
  }> {
    const startTime = performance.now();
    
    try {
      const serializedState = localStorage.getItem(this.STORAGE_KEY);
      if (!serializedState) {
        return {
          loadTime: 0,
          migrationTime: 0,
          totalTime: 0,
          isOptimal: true,
        };
      }

      const loadStartTime = performance.now();
      let persistedState = JSON.parse(serializedState);
      const loadTime = performance.now() - loadStartTime;
      
      const migrationStartTime = performance.now();
      persistedState = await this.migrateState(persistedState);
      const migrationTime = performance.now() - migrationStartTime;
      
      const totalTime = performance.now() - startTime;
      
      return {
        loadTime,
        migrationTime,
        totalTime,
        isOptimal: totalTime < 500, // Target: restore within 500ms
      };
    } catch (error) {
      return {
        loadTime: 0,
        migrationTime: 0,
        totalTime: performance.now() - startTime,
        isOptimal: false,
      };
    }
  }

  /**
   * Apply migration strategies
   */
  private async migrateState(state: any): Promise<PersistedTreeState> {
    if (!state.version || state.version === this.CURRENT_VERSION) {
      return state;
    }

    let migratedState = state;
    
    for (const strategy of this.migrationStrategies) {
      if (migratedState.version === strategy.fromVersion) {
        console.debug(`Migrating tree state from v${strategy.fromVersion} to v${strategy.toVersion}`);
        migratedState = strategy.migrate(migratedState);
      }
    }

    return migratedState;
  }

  /**
   * Validate state structure
   */
  private validateState(state: PersistedTreeState): void {
    if (!state.version || typeof state.version !== 'number') {
      throw new Error('Invalid state version');
    }

    if (!Array.isArray(state.expandedKeys)) {
      throw new Error('Invalid expandedKeys format');
    }

    if (state.selectedAssetId !== null && typeof state.selectedAssetId !== 'number') {
      throw new Error('Invalid selectedAssetId format');
    }

    if (!Array.isArray(state.navigationHistory)) {
      throw new Error('Invalid navigationHistory format');
    }

    if (!state.viewPreferences || typeof state.viewPreferences !== 'object') {
      throw new Error('Invalid viewPreferences format');
    }
  }
}

/**
 * Create a debounced save function to prevent excessive writes
 */
export function createDebouncedSave(
  persistenceManager: TreeStatePersistenceManager,
  delay: number = 300
): (state: Partial<TreeViewState>) => void {
  let timeoutId: NodeJS.Timeout;
  
  return (state: Partial<TreeViewState>) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
      persistenceManager.saveState(state).catch(error => {
        console.error('Debounced save failed:', error);
      });
    }, delay);
  };
}

/**
 * State validation utilities
 */
export const StateValidator = {
  isValidExpandedKeys: (keys: any): keys is Set<number> => {
    return keys instanceof Set && Array.from(keys).every(k => typeof k === 'number');
  },
  
  isValidAssetId: (id: any): id is number | null => {
    return id === null || (typeof id === 'number' && id > 0);
  },
  
  isValidNavigationHistory: (history: any): history is number[] => {
    return Array.isArray(history) && history.every(id => typeof id === 'number' && id > 0);
  },
  
  isValidBreadcrumbPath: (path: any): boolean => {
    return Array.isArray(path) && path.every(item => 
      item && typeof item.id === 'number' && typeof item.name === 'string'
    );
  },
};

// Singleton instance
export const treePersistenceManager = new TreeStatePersistenceManager();