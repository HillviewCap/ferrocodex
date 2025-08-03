import { create } from 'zustand';
import { devtools, persist, subscribeWithSelector } from 'zustand/middleware';
import { AssetInfo } from '../types/assets';

export interface TreeViewState {
  // Core tree state
  expandedKeys: Set<number>;
  selectedAssetId: number | null;
  
  // Navigation state
  breadcrumbPath: AssetInfo[];
  navigationHistory: number[];
  currentPosition: number;
  
  // View preferences
  viewMode: 'tree' | 'virtualized';
  searchValue: string;
  showHidden: boolean;
  sortBy: 'name' | 'type' | 'created_at' | 'updated_at';
  sortOrder: 'asc' | 'desc';
  
  // Performance settings
  virtualizationEnabled: boolean;
  itemHeight: number;
  overscanCount: number;
  
  // UI state
  sidebarWidth: number;
  showPerformanceMetrics: boolean;
  compactMode: boolean;
}

export interface TreeStateActions {
  // Core actions
  setExpandedKeys: (keys: Set<number>) => void;
  expandNode: (nodeId: number) => void;
  collapseNode: (nodeId: number) => void;
  toggleNode: (nodeId: number) => void;
  selectAsset: (assetId: number | null) => void;
  
  // Navigation actions
  setBreadcrumbPath: (path: AssetInfo[]) => void;
  navigateToAsset: (assetId: number) => void;
  navigateBack: () => void;
  navigateForward: () => void;
  clearNavigationHistory: () => void;
  
  // Search and filtering
  setSearchValue: (value: string) => void;
  setShowHidden: (show: boolean) => void;
  setSortBy: (sortBy: TreeViewState['sortBy']) => void;
  setSortOrder: (order: TreeViewState['sortOrder']) => void;
  
  // View preferences
  setViewMode: (mode: TreeViewState['viewMode']) => void;
  setVirtualizationEnabled: (enabled: boolean) => void;
  setItemHeight: (height: number) => void;
  setOverscanCount: (count: number) => void;
  
  // UI preferences
  setSidebarWidth: (width: number) => void;
  setShowPerformanceMetrics: (show: boolean) => void;
  setCompactMode: (compact: boolean) => void;
  
  // Batch operations
  expandAll: () => void;
  collapseAll: () => void;
  restoreViewState: (state: Partial<TreeViewState>) => void;
  
  // Persistence
  getPersistedState: () => TreeViewState;
  
  // Reset
  reset: () => void;
}

type TreeState = TreeViewState & TreeStateActions;

const initialState: TreeViewState = {
  // Core tree state
  expandedKeys: new Set<number>(),
  selectedAssetId: null,
  
  // Navigation state
  breadcrumbPath: [],
  navigationHistory: [],
  currentPosition: -1,
  
  // View preferences
  viewMode: 'virtualized',
  searchValue: '',
  showHidden: false,
  sortBy: 'name',
  sortOrder: 'asc',
  
  // Performance settings
  virtualizationEnabled: true,
  itemHeight: 32,
  overscanCount: 5,
  
  // UI state
  sidebarWidth: 400,
  showPerformanceMetrics: false,
  compactMode: false,
};

// Custom serializer for Set objects
const serializer = {
  serialize: (state: TreeState) => {
    return JSON.stringify({
      ...state,
      expandedKeys: Array.from(state.expandedKeys || new Set()),
    });
  },
  deserialize: (str: string) => {
    const parsed = JSON.parse(str);
    return {
      ...parsed,
      expandedKeys: new Set(parsed.expandedKeys || []),
    };
  },
};

export const useTreeStateStore = create<TreeState>()(
  devtools(
    subscribeWithSelector(
      persist(
        (set, get) => ({
          ...initialState,

          // Core actions
          setExpandedKeys: (keys: Set<number>) => {
            set({ expandedKeys: keys });
          },

          expandNode: (nodeId: number) => {
            const { expandedKeys } = get();
            const newKeys = new Set(expandedKeys);
            newKeys.add(nodeId);
            set({ expandedKeys: newKeys });
          },

          collapseNode: (nodeId: number) => {
            const { expandedKeys } = get();
            const newKeys = new Set(expandedKeys);
            newKeys.delete(nodeId);
            set({ expandedKeys: newKeys });
          },

          toggleNode: (nodeId: number) => {
            const { expandedKeys } = get();
            if (expandedKeys.has(nodeId)) {
              get().collapseNode(nodeId);
            } else {
              get().expandNode(nodeId);
            }
          },

          selectAsset: (assetId: number | null) => {
            const { navigationHistory, currentPosition } = get();
            
            if (assetId !== null) {
              // Add to navigation history
              const newHistory = navigationHistory.slice(0, currentPosition + 1);
              newHistory.push(assetId);
              
              set({
                selectedAssetId: assetId,
                navigationHistory: newHistory,
                currentPosition: newHistory.length - 1,
              });
            } else {
              set({ selectedAssetId: null });
            }
          },

          // Navigation actions
          setBreadcrumbPath: (path: AssetInfo[]) => {
            set({ breadcrumbPath: path });
          },

          navigateToAsset: (assetId: number) => {
            get().selectAsset(assetId);
          },

          navigateBack: () => {
            const { navigationHistory, currentPosition } = get();
            if (currentPosition > 0) {
              const newPosition = currentPosition - 1;
              const assetId = navigationHistory[newPosition];
              set({
                selectedAssetId: assetId,
                currentPosition: newPosition,
              });
            }
          },

          navigateForward: () => {
            const { navigationHistory, currentPosition } = get();
            if (currentPosition < navigationHistory.length - 1) {
              const newPosition = currentPosition + 1;
              const assetId = navigationHistory[newPosition];
              set({
                selectedAssetId: assetId,
                currentPosition: newPosition,
              });
            }
          },

          clearNavigationHistory: () => {
            set({
              navigationHistory: [],
              currentPosition: -1,
            });
          },

          // Search and filtering
          setSearchValue: (value: string) => {
            set({ searchValue: value });
          },

          setShowHidden: (show: boolean) => {
            set({ showHidden: show });
          },

          setSortBy: (sortBy: TreeViewState['sortBy']) => {
            set({ sortBy });
          },

          setSortOrder: (order: TreeViewState['sortOrder']) => {
            set({ sortOrder: order });
          },

          // View preferences
          setViewMode: (mode: TreeViewState['viewMode']) => {
            set({ viewMode: mode });
          },

          setVirtualizationEnabled: (enabled: boolean) => {
            set({ virtualizationEnabled: enabled });
          },

          setItemHeight: (height: number) => {
            set({ itemHeight: Math.max(24, Math.min(height, 100)) }); // Clamp between 24-100px
          },

          setOverscanCount: (count: number) => {
            set({ overscanCount: Math.max(1, Math.min(count, 20)) }); // Clamp between 1-20
          },

          // UI preferences
          setSidebarWidth: (width: number) => {
            set({ sidebarWidth: Math.max(200, Math.min(width, 800)) }); // Clamp between 200-800px
          },

          setShowPerformanceMetrics: (show: boolean) => {
            set({ showPerformanceMetrics: show });
          },

          setCompactMode: (compact: boolean) => {
            set({ compactMode: compact });
          },

          // Batch operations
          expandAll: (hierarchyData?: AssetInfo[]) => {
            if (!hierarchyData) {
              console.warn('expandAll requires hierarchy data context');
              return;
            }
            
            const getAllNodeIds = (items: AssetInfo[]): number[] => {
              const ids: number[] = [];
              for (const item of items) {
                ids.push(item.id);
                // Note: AssetInfo doesn't have children property, so this is a simplified implementation
                // In actual use, this would need to be called with full hierarchy data
              }
              return ids;
            };
            
            const allIds = getAllNodeIds(hierarchyData);
            set({ expandedKeys: new Set(allIds) });
          },

          collapseAll: () => {
            set({ expandedKeys: new Set<number>() });
          },

          restoreViewState: (state: Partial<TreeViewState>) => {
            set((current) => ({
              ...current,
              ...state,
              expandedKeys: state.expandedKeys || current.expandedKeys,
            }));
          },

          // Persistence
          getPersistedState: () => {
            const state = get();
            return {
              expandedKeys: state.expandedKeys,
              selectedAssetId: state.selectedAssetId,
              breadcrumbPath: state.breadcrumbPath,
              navigationHistory: state.navigationHistory,
              currentPosition: state.currentPosition,
              viewMode: state.viewMode,
              searchValue: state.searchValue,
              showHidden: state.showHidden,
              sortBy: state.sortBy,
              sortOrder: state.sortOrder,
              virtualizationEnabled: state.virtualizationEnabled,
              itemHeight: state.itemHeight,
              overscanCount: state.overscanCount,
              sidebarWidth: state.sidebarWidth,
              showPerformanceMetrics: state.showPerformanceMetrics,
              compactMode: state.compactMode,
            };
          },

          // Reset
          reset: () => {
            set(initialState);
          },
        }),
        {
          name: 'tree-view-state',
          // Only persist specific fields
          partialize: (state) => ({
            expandedKeys: state.expandedKeys,
            selectedAssetId: state.selectedAssetId,
            viewMode: state.viewMode,
            sortBy: state.sortBy,
            sortOrder: state.sortOrder,
            virtualizationEnabled: state.virtualizationEnabled,
            itemHeight: state.itemHeight,
            overscanCount: state.overscanCount,
            sidebarWidth: state.sidebarWidth,
            showPerformanceMetrics: state.showPerformanceMetrics,
            compactMode: state.compactMode,
          }),
          storage: {
            getItem: (name) => {
              const item = localStorage.getItem(name);
              return item ? serializer.deserialize(item) : null;
            },
            setItem: (name, value) => {
              localStorage.setItem(name, serializer.serialize(value as TreeState));
            },
            removeItem: (name) => {
              localStorage.removeItem(name);
            },
          },
          version: 1,
          migrate: (persistedState: any, version: number) => {
            // Handle state migrations if needed
            if (version === 0) {
              // Migrate from version 0 to 1
              return {
                ...persistedState,
                expandedKeys: new Set(persistedState.expandedKeys || []),
              };
            }
            return persistedState;
          },
        }
      )
    ),
    {
      name: 'tree-state',
    }
  )
);

// Selector hooks for performance optimization
export const useExpandedKeys = () => useTreeStateStore(state => state.expandedKeys);
export const useSelectedAssetId = () => useTreeStateStore(state => state.selectedAssetId);
export const useBreadcrumbPath = () => useTreeStateStore(state => state.breadcrumbPath);
export const useNavigationHistory = () => useTreeStateStore(state => ({
  history: state.navigationHistory,
  currentPosition: state.currentPosition,
  canGoBack: state.currentPosition > 0,
  canGoForward: state.currentPosition < state.navigationHistory.length - 1,
}));
export const useSearchValue = () => useTreeStateStore(state => state.searchValue);
export const useViewMode = () => useTreeStateStore(state => state.viewMode);
export const useTreeViewPreferences = () => useTreeStateStore(state => ({
  sortBy: state.sortBy,
  sortOrder: state.sortOrder,
  showHidden: state.showHidden,
  compactMode: state.compactMode,
}));
export const useVirtualizationSettings = () => useTreeStateStore(state => ({
  enabled: state.virtualizationEnabled,
  itemHeight: state.itemHeight,
  overscanCount: state.overscanCount,
}));
export const useUIPreferences = () => useTreeStateStore(state => ({
  sidebarWidth: state.sidebarWidth,
  showPerformanceMetrics: state.showPerformanceMetrics,
}));

// Action hooks
export const useTreeStateActions = () => {
  const store = useTreeStateStore();
  return {
    setExpandedKeys: store.setExpandedKeys,
    expandNode: store.expandNode,
    collapseNode: store.collapseNode,
    toggleNode: store.toggleNode,
    selectAsset: store.selectAsset,
    setBreadcrumbPath: store.setBreadcrumbPath,
    navigateToAsset: store.navigateToAsset,
    navigateBack: store.navigateBack,
    navigateForward: store.navigateForward,
    clearNavigationHistory: store.clearNavigationHistory,
    setSearchValue: store.setSearchValue,
    setShowHidden: store.setShowHidden,
    setSortBy: store.setSortBy,
    setSortOrder: store.setSortOrder,
    setViewMode: store.setViewMode,
    setVirtualizationEnabled: store.setVirtualizationEnabled,
    setItemHeight: store.setItemHeight,
    setOverscanCount: store.setOverscanCount,
    setSidebarWidth: store.setSidebarWidth,
    setShowPerformanceMetrics: store.setShowPerformanceMetrics,
    setCompactMode: store.setCompactMode,
    expandAll: store.expandAll,
    collapseAll: store.collapseAll,
    restoreViewState: store.restoreViewState,
    reset: store.reset,
  };
};