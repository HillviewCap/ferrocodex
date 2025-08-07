/**
 * Tree State Type Definitions
 * 
 * Comprehensive type definitions for tree navigation state management,
 * including view states, preferences, and navigation history.
 */

import { AssetInfo } from './assets';

// Core tree navigation state
export interface TreeNavigationState {
  expandedKeys: Set<number>;
  selectedAssetId: number | null;
  breadcrumbPath: AssetInfo[];
  navigationHistory: number[];
  currentNavigationPosition: number;
}

// Tree view preferences
export interface TreeViewPreferences {
  viewMode: 'tree' | 'virtualized' | 'compact';
  sortBy: 'name' | 'type' | 'created_at' | 'updated_at' | 'sort_order';
  sortOrder: 'asc' | 'desc';
  showHidden: boolean;
  filterByType?: 'all' | 'folders' | 'devices';
  groupByType: boolean;
}

// Performance and virtualization settings
export interface TreePerformanceSettings {
  virtualizationEnabled: boolean;
  itemHeight: number;
  overscanCount: number;
  lazyLoadingEnabled: boolean;
  cacheSize: number;
  debounceDelay: number;
  preloadDepth: number;
}

// UI state and layout preferences
export interface TreeUIState {
  sidebarWidth: number;
  showPerformanceMetrics: boolean;
  compactMode: boolean;
  showBreadcrumbs: boolean;
  showSearchBar: boolean;
  showQuickActions: boolean;
  theme: 'light' | 'dark' | 'auto';
}

// Search and filtering state
export interface TreeSearchState {
  searchValue: string;
  searchMode: 'name' | 'description' | 'all';
  caseSensitive: boolean;
  useRegex: boolean;
  searchInProgress: boolean;
  searchResults: {
    total: number;
    matches: Array<{
      assetId: number;
      matchType: 'name' | 'description';
      matchText: string;
    }>;
  };
}

// Keyboard navigation state
export interface KeyboardNavigationState {
  focusedNodeId: number | null;
  keyboardMode: boolean;
  lastKeyboardAction: 'expand' | 'collapse' | 'select' | 'navigate' | null;
  navigationPath: number[]; // Path to currently focused item
}

// Complete tree state interface
export interface CompleteTreeState {
  navigation: TreeNavigationState;
  preferences: TreeViewPreferences;
  performance: TreePerformanceSettings;
  ui: TreeUIState;
  search: TreeSearchState;
  keyboard: KeyboardNavigationState;
  lastUpdated: Date;
  version: number;
}

// State persistence configuration
export interface PersistenceConfig {
  enabled: boolean;
  autoSave: boolean;
  saveDelay: number;
  maxHistorySize: number;
  maxStateSize: number;
  compression: boolean;
  encryption: boolean;
}

// State restoration options
export interface StateRestorationOptions {
  restoreNavigation: boolean;
  restorePreferences: boolean;
  restoreUIState: boolean;
  restoreSearchState: boolean;
  skipMigration: boolean;
  fallbackToDefaults: boolean;
}

// Migration context for version upgrades
export interface StateMigrationContext {
  fromVersion: number;
  toVersion: number;
  migrationDate: Date;
  preserveUserData: boolean;
  backupOriginal: boolean;
}

// Performance metrics for state operations
export interface StatePerformanceMetrics {
  loadTime: number;
  saveTime: number;
  migrationTime: number;
  serializationTime: number;
  deserializationTime: number;
  stateSize: number;
  memoryUsage: number;
}

// Tree node context for state operations
export interface TreeNodeContext {
  nodeId: number;
  parentId: number | null;
  level: number;
  isExpanded: boolean;
  hasChildren: boolean;
  isVisible: boolean;
  isFiltered: boolean;
  isSelected: boolean;
  isFocused: boolean;
}

// State change event types
export type TreeStateChangeEvent = 
  | { type: 'NODE_EXPANDED'; nodeId: number }
  | { type: 'NODE_COLLAPSED'; nodeId: number }
  | { type: 'NODE_SELECTED'; nodeId: number | null }
  | { type: 'SEARCH_CHANGED'; searchValue: string }
  | { type: 'VIEW_MODE_CHANGED'; viewMode: TreeViewPreferences['viewMode'] }
  | { type: 'PREFERENCES_UPDATED'; preferences: Partial<TreeViewPreferences> }
  | { type: 'NAVIGATION_CHANGED'; navigationState: TreeNavigationState }
  | { type: 'STATE_RESTORED'; restoredState: Partial<CompleteTreeState> }
  | { type: 'STATE_PERSISTED'; persistedData: any };

// State validation result
export interface StateValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  fixes: Array<{
    field: string;
    oldValue: any;
    newValue: any;
    reason: string;
  }>;
}

// Tree state hooks return types
export interface TreeStateHookResult {
  state: CompleteTreeState;
  actions: TreeStateActions;
  selectors: TreeStateSelectors;
  utils: TreeStateUtils;
}

// Action creators interface
export interface TreeStateActions {
  // Navigation actions
  selectNode: (nodeId: number | null) => void;
  expandNode: (nodeId: number) => void;
  collapseNode: (nodeId: number) => void;
  toggleNode: (nodeId: number) => void;
  expandAll: () => void;
  collapseAll: () => void;
  
  // Navigation history
  navigateBack: () => void;
  navigateForward: () => void;
  clearHistory: () => void;
  
  // Search actions
  setSearchValue: (value: string) => void;
  clearSearch: () => void;
  
  // Preference actions
  setViewMode: (mode: TreeViewPreferences['viewMode']) => void;
  setSortBy: (sortBy: TreeViewPreferences['sortBy']) => void;
  setSortOrder: (order: TreeViewPreferences['sortOrder']) => void;
  
  // UI actions
  setSidebarWidth: (width: number) => void;
  setCompactMode: (compact: boolean) => void;
  
  // Batch operations
  batchUpdate: (updates: Partial<CompleteTreeState>) => void;
  
  // State management
  saveState: () => Promise<void>;
  loadState: () => Promise<void>;
  resetState: () => void;
  exportState: () => Promise<string>;
  importState: (data: string) => Promise<void>;
}

// Selector functions interface
export interface TreeStateSelectors {
  getExpandedNodes: () => Set<number>;
  getSelectedNode: () => number | null;
  getBreadcrumbPath: () => AssetInfo[];
  getNavigationHistory: () => { history: number[]; position: number };
  getSearchResults: () => TreeSearchState['searchResults'];
  getPerformanceSettings: () => TreePerformanceSettings;
  getUIPreferences: () => TreeUIState;
  
  // Computed selectors
  canNavigateBack: () => boolean;
  canNavigateForward: () => boolean;
  hasActiveSearch: () => boolean;
  isNodeExpanded: (nodeId: number) => boolean;
  isNodeSelected: (nodeId: number) => boolean;
  
  // Statistics
  getExpandedCount: () => number;
  getNavigationDepth: () => number;
  getStateSize: () => number;
}

// Utility functions interface
export interface TreeStateUtils {
  validateState: (state: Partial<CompleteTreeState>) => StateValidationResult;
  migrateState: (oldState: any, context: StateMigrationContext) => CompleteTreeState;
  compressState: (state: CompleteTreeState) => string;
  decompressState: (compressed: string) => CompleteTreeState;
  measurePerformance: () => StatePerformanceMetrics;
  
  // Path utilities
  getNodePath: (nodeId: number) => number[];
  findNodeById: (nodeId: number) => TreeNodeContext | null;
  getNodeChildren: (nodeId: number) => number[];
  getNodeParent: (nodeId: number) => number | null;
  
  // Search utilities
  searchNodes: (query: string, options?: Partial<TreeSearchState>) => number[];
  highlightMatches: (text: string, query: string) => { text: string; isMatch: boolean }[];
  
  // Navigation utilities
  getNextNode: (currentId: number) => number | null;
  getPreviousNode: (currentId: number) => number | null;
  getNextVisibleNode: (currentId: number) => number | null;
  getPreviousVisibleNode: (currentId: number) => number | null;
}

// Default state values
export const DEFAULT_TREE_STATE: CompleteTreeState = {
  navigation: {
    expandedKeys: new Set<number>(),
    selectedAssetId: null,
    breadcrumbPath: [],
    navigationHistory: [],
    currentNavigationPosition: -1,
  },
  preferences: {
    viewMode: 'virtualized',
    sortBy: 'name',
    sortOrder: 'asc',
    showHidden: false,
    filterByType: 'all',
    groupByType: false,
  },
  performance: {
    virtualizationEnabled: true,
    itemHeight: 32,
    overscanCount: 5,
    lazyLoadingEnabled: true,
    cacheSize: 500,
    debounceDelay: 300,
    preloadDepth: 2,
  },
  ui: {
    sidebarWidth: 400,
    showPerformanceMetrics: false,
    compactMode: false,
    showBreadcrumbs: true,
    showSearchBar: true,
    showQuickActions: true,
    theme: 'light',
  },
  search: {
    searchValue: '',
    searchMode: 'all',
    caseSensitive: false,
    useRegex: false,
    searchInProgress: false,
    searchResults: {
      total: 0,
      matches: [],
    },
  },
  keyboard: {
    focusedNodeId: null,
    keyboardMode: false,
    lastKeyboardAction: null,
    navigationPath: [],
  },
  lastUpdated: new Date(),
  version: 1,
};