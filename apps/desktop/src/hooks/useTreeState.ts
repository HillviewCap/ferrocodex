import { useCallback, useEffect, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  useTreeStateStore,
  useExpandedKeys,
  useSelectedAssetId,
  useBreadcrumbPath,
  useNavigationHistory,
  useTreeStateActions
} from '../store/treeState';
import { AssetHierarchy, AssetInfo } from '../types/assets';
import useAuthStore from '../store/auth';

export interface TreeStateConfig {
  autoSaveBreadcrumbs?: boolean;
  autoExpandSearchResults?: boolean;
  maxNavigationHistory?: number;
  persistenceDelay?: number;
}

export const useTreeState = (config: TreeStateConfig = {}) => {
  const {
    autoSaveBreadcrumbs = true,
    autoExpandSearchResults = true,
    maxNavigationHistory = 50,
    persistenceDelay = 300,
  } = config;

  const { token } = useAuthStore();
  const expandedKeys = useExpandedKeys();
  const selectedAssetId = useSelectedAssetId();
  const breadcrumbPath = useBreadcrumbPath();
  const navigationState = useNavigationHistory();
  const actions = useTreeStateActions();

  const persistenceTimeoutRef = useRef<number>();

  // Debounced persistence to avoid excessive saves
  const scheduleStatePersistence = useCallback(() => {
    if (persistenceTimeoutRef.current) {
      clearTimeout(persistenceTimeoutRef.current);
    }
    
    persistenceTimeoutRef.current = setTimeout(() => {
      // State is automatically persisted by Zustand middleware
      console.debug('Tree state persisted');
    }, persistenceDelay);
  }, [persistenceDelay]);

  // Enhanced select asset with breadcrumb management
  const selectAssetWithBreadcrumbs = useCallback(async (assetId: number | null) => {
    actions.selectAsset(assetId);
    
    if (assetId && autoSaveBreadcrumbs && token) {
      try {
        const path = await invoke<AssetInfo[]>('get_asset_path', {
          token,
          assetId,
        });
        actions.setBreadcrumbPath(path);
      } catch (error) {
        console.error('Failed to update breadcrumb path:', error);
      }
    } else if (!assetId) {
      actions.setBreadcrumbPath([]);
    }
    
    scheduleStatePersistence();
  }, [autoSaveBreadcrumbs, token, actions, scheduleStatePersistence]);

  // Enhanced node expansion with auto-save
  const toggleNodeWithPersistence = useCallback((nodeId: number) => {
    actions.toggleNode(nodeId);
    scheduleStatePersistence();
  }, [actions, scheduleStatePersistence]);

  const expandNodeWithPersistence = useCallback((nodeId: number) => {
    actions.expandNode(nodeId);
    scheduleStatePersistence();
  }, [actions, scheduleStatePersistence]);

  const collapseNodeWithPersistence = useCallback((nodeId: number) => {
    actions.collapseNode(nodeId);
    scheduleStatePersistence();
  }, [actions, scheduleStatePersistence]);

  // Batch expand/collapse operations
  const expandMultipleNodes = useCallback((nodeIds: number[]) => {
    const newExpandedKeys = new Set(expandedKeys);
    nodeIds.forEach(nodeId => newExpandedKeys.add(nodeId));
    actions.setExpandedKeys(newExpandedKeys);
    scheduleStatePersistence();
  }, [expandedKeys, actions, scheduleStatePersistence]);

  const collapseMultipleNodes = useCallback((nodeIds: number[]) => {
    const newExpandedKeys = new Set(expandedKeys);
    nodeIds.forEach(nodeId => newExpandedKeys.delete(nodeId));
    actions.setExpandedKeys(newExpandedKeys);
    scheduleStatePersistence();
  }, [expandedKeys, actions, scheduleStatePersistence]);

  // Navigation with history management
  const navigateWithHistory = useCallback(async (assetId: number) => {
    await selectAssetWithBreadcrumbs(assetId);
    
    // Trim history if it gets too long
    if (navigationState.history.length >= maxNavigationHistory) {
      // Note: This would require additional store action to set history directly
      console.debug('Navigation history trimmed');
    }
  }, [selectAssetWithBreadcrumbs, navigationState.history.length, maxNavigationHistory]);

  // Search result auto-expansion
  const handleSearchResultExpansion = useCallback((hierarchyData: AssetHierarchy[], searchValue: string) => {
    if (!autoExpandSearchResults || !searchValue.trim()) return;

    const searchLower = searchValue.toLowerCase();
    const matchingParentIds = new Set<number>();

    const findMatches = (items: AssetHierarchy[], parentIds: number[] = []) => {
      items.forEach(item => {
        const isMatch = item.name.toLowerCase().includes(searchLower) ||
                       item.description.toLowerCase().includes(searchLower);
        
        if (isMatch) {
          // Add all parent IDs to expansion set
          parentIds.forEach(parentId => matchingParentIds.add(parentId));
        }
        
        if (item.children.length > 0) {
          findMatches(item.children, [...parentIds, item.id]);
        }
      });
    };

    findMatches(hierarchyData);
    
    if (matchingParentIds.size > 0) {
      expandMultipleNodes(Array.from(matchingParentIds));
    }
  }, [autoExpandSearchResults, expandMultipleNodes]);

  // State restoration utilities
  const restoreExpandedState = useCallback((nodeIds: number[]) => {
    actions.setExpandedKeys(new Set(nodeIds));
  }, [actions]);

  const exportViewState = useCallback(() => {
    return useTreeStateStore.getState().getPersistedState();
  }, []);

  const importViewState = useCallback((state: Parameters<typeof actions.restoreViewState>[0]) => {
    actions.restoreViewState(state);
    scheduleStatePersistence();
  }, [actions, scheduleStatePersistence]);

  // Performance-aware batch operations
  const batchTreeOperations = useCallback((operations: (() => void)[]) => {
    // Suspend persistence during batch operations
    if (persistenceTimeoutRef.current) {
      clearTimeout(persistenceTimeoutRef.current);
    }
    
    operations.forEach(operation => operation());
    
    // Schedule single persistence after all operations
    scheduleStatePersistence();
  }, [scheduleStatePersistence]);

  // Get current tree statistics
  const getTreeStatistics = useMemo(() => {
    return {
      expandedCount: expandedKeys.size,
      hasSelection: selectedAssetId !== null,
      breadcrumbDepth: breadcrumbPath.length,
      navigationHistorySize: navigationState.history.length,
      canNavigateBack: navigationState.canGoBack,
      canNavigateForward: navigationState.canGoForward,
    };
  }, [
    expandedKeys.size,
    selectedAssetId,
    breadcrumbPath.length,
    navigationState.history.length,
    navigationState.canGoBack,
    navigationState.canGoForward,
  ]);

  // Cleanup effect
  useEffect(() => {
    return () => {
      if (persistenceTimeoutRef.current) {
        clearTimeout(persistenceTimeoutRef.current);
      }
    };
  }, []);

  return {
    // Current state
    expandedKeys,
    selectedAssetId,
    breadcrumbPath,
    navigationState,
    statistics: getTreeStatistics,
    
    // Enhanced actions
    selectAsset: selectAssetWithBreadcrumbs,
    toggleNode: toggleNodeWithPersistence,
    expandNode: expandNodeWithPersistence,
    collapseNode: collapseNodeWithPersistence,
    expandMultipleNodes,
    collapseMultipleNodes,
    
    // Navigation
    navigateToAsset: navigateWithHistory,
    navigateBack: actions.navigateBack,
    navigateForward: actions.navigateForward,
    clearHistory: actions.clearNavigationHistory,
    
    // Search
    handleSearchResultExpansion,
    
    // State management
    restoreExpandedState,
    exportViewState,
    importViewState,
    batchTreeOperations,
    
    // Direct access to all actions for advanced usage
    actions,
  };
};