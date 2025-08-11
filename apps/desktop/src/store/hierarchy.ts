import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { AssetHierarchy, AssetInfo, MoveAssetRequest } from '../types/assets';

interface HierarchyState {
  // Data
  hierarchyData: AssetHierarchy[];
  selectedAsset: AssetHierarchy | null;
  expandedKeys: string[];
  
  // Loading states
  isLoading: boolean;
  isMoving: boolean;
  error: string | null;
  
  // Actions
  loadHierarchy: (token: string) => Promise<void>;
  selectAsset: (asset: AssetHierarchy | null) => void;
  setExpandedKeys: (keys: string[]) => void;
  moveAsset: (token: string, request: MoveAssetRequest) => Promise<void>;
  validateMove: (token: string, assetId: number, newParentId: number | null) => Promise<boolean>;
  refreshHierarchy: (token: string) => Promise<void>;
  
  // Utility functions
  findAssetById: (id: number) => AssetHierarchy | null;
  getAssetPath: (token: string, assetId: number) => Promise<AssetInfo[]>;
  
  // Reset
  reset: () => void;
}

const initialState = {
  hierarchyData: [],
  selectedAsset: null,
  expandedKeys: [],
  isLoading: false,
  isMoving: false,
  error: null,
};

export const useHierarchyStore = create<HierarchyState>()(
  devtools(
    (set, get) => ({
      ...initialState,

      loadHierarchy: async (token: string) => {
        set({ isLoading: true, error: null });
        try {
          const hierarchy = await invoke<AssetHierarchy[]>('get_asset_hierarchy', { token });
          set({ hierarchyData: hierarchy, isLoading: false });
        } catch (error) {
          console.error('Failed to load hierarchy:', error);
          set({ 
            error: error instanceof Error ? error.message : 'Failed to load hierarchy',
            isLoading: false 
          });
        }
      },

      selectAsset: (asset: AssetHierarchy | null) => {
        set({ selectedAsset: asset });
      },

      setExpandedKeys: (keys: string[]) => {
        set({ expandedKeys: keys });
      },

      moveAsset: async (token: string, request: MoveAssetRequest) => {
        set({ isMoving: true, error: null });
        try {
          await invoke('move_asset', {
            token,
            assetId: request.asset_id,
            newParentId: request.new_parent_id,
            newSortOrder: request.new_sort_order,
          });
          
          // Refresh hierarchy after successful move
          await get().refreshHierarchy(token);
          set({ isMoving: false });
        } catch (error) {
          console.error('Failed to move asset:', error);
          set({ 
            error: error instanceof Error ? error.message : 'Failed to move asset',
            isMoving: false 
          });
          throw error; // Re-throw to allow caller to handle
        }
      },

      validateMove: async (token: string, assetId: number, newParentId: number | null) => {
        try {
          const isValid = await invoke<boolean>('validate_asset_move', {
            token,
            assetId,
            newParentId,
          });
          return isValid;
        } catch (error) {
          console.error('Failed to validate move:', error);
          return false;
        }
      },

      refreshHierarchy: async (token: string) => {
        // Use loadHierarchy but don't change loading state if already loading
        const { isLoading } = get();
        if (!isLoading) {
          await get().loadHierarchy(token);
        } else {
          try {
            const hierarchy = await invoke<AssetHierarchy[]>('get_asset_hierarchy', { token });
            set({ hierarchyData: hierarchy });
          } catch (error) {
            console.error('Failed to refresh hierarchy:', error);
          }
        }
      },

      findAssetById: (id: number): AssetHierarchy | null => {
        const { hierarchyData } = get();
        
        const findInTree = (assets: AssetHierarchy[]): AssetHierarchy | null => {
          for (const asset of assets) {
            if (asset.id === id) {
              return asset;
            }
            if (asset.children.length > 0) {
              const found = findInTree(asset.children);
              if (found) return found;
            }
          }
          return null;
        };
        
        return findInTree(hierarchyData);
      },

      getAssetPath: async (token: string, assetId: number) => {
        try {
          const path = await invoke<AssetInfo[]>('get_asset_path', {
            token,
            assetId,
          });
          return path;
        } catch (error) {
          console.error('Failed to get asset path:', error);
          throw error;
        }
      },

      reset: () => {
        set(initialState);
      },
    }),
    {
      name: 'hierarchy-store',
      partialize: (state: HierarchyState) => ({
        expandedKeys: state.expandedKeys,
        selectedAsset: state.selectedAsset,
      }),
    }
  )
);

// Selector hooks for better performance
export const useHierarchyData = () => useHierarchyStore(state => state.hierarchyData);
export const useSelectedAsset = () => useHierarchyStore(state => state.selectedAsset);
export const useHierarchyLoading = () => useHierarchyStore(state => state.isLoading);
export const useHierarchyError = () => useHierarchyStore(state => state.error);
export const useExpandedKeys = () => useHierarchyStore(state => state.expandedKeys);
export const useIsMoving = () => useHierarchyStore(state => state.isMoving);