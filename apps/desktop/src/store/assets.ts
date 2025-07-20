import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo, ConfigurationVersionInfo } from '../types/assets';

interface AssetState {
  assets: AssetInfo[];
  isLoading: boolean;
  error: string | null;
  selectedAsset: AssetInfo | null;
  versions: ConfigurationVersionInfo[];
  versionsLoading: boolean;
  currentView: 'dashboard' | 'history';
  goldenVersions: Record<number, ConfigurationVersionInfo | null>; // Map asset_id to golden version
  goldenVersionsLoading: Record<number, boolean>; // Track loading state per asset
  
  // Actions
  fetchAssets: (token: string) => Promise<void>;
  createAsset: (token: string, name: string, description: string) => Promise<AssetInfo>;
  selectAsset: (asset: AssetInfo | null) => void;
  fetchVersions: (token: string, assetId: number) => Promise<void>;
  clearError: () => void;
  refreshAssets: (token: string) => Promise<void>;
  setCurrentView: (view: 'dashboard' | 'history') => void;
  navigateToHistory: (asset: AssetInfo) => void;
  navigateToDashboard: () => void;
  
  // Golden promotion actions
  fetchGoldenVersion: (token: string, assetId: number) => Promise<ConfigurationVersionInfo | null>;
  promoteToGolden: (token: string, versionId: number, reason: string) => Promise<void>;
  checkPromotionEligibility: (token: string, versionId: number) => Promise<boolean>;
  
  // Export actions
  exportConfiguration: (token: string, versionId: number, exportPath: string) => Promise<void>;
}

const useAssetStore = create<AssetState>((set, get) => ({
  assets: [],
  isLoading: false,
  error: null,
  selectedAsset: null,
  versions: [],
  versionsLoading: false,
  currentView: 'dashboard',
  goldenVersions: {},
  goldenVersionsLoading: {},

  fetchAssets: async (token: string) => {
    set({ isLoading: true, error: null });
    try {
      const assets = await invoke<AssetInfo[]>('get_dashboard_assets', { token });
      set({ assets, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch assets:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch assets',
        isLoading: false 
      });
    }
  },

  createAsset: async (token: string, name: string, description: string) => {
    try {
      const asset = await invoke<AssetInfo>('create_asset', {
        token,
        name,
        description
      });
      
      // Add to the beginning of the assets array
      set(state => ({
        assets: [asset, ...state.assets]
      }));
      
      return asset;
    } catch (error) {
      console.error('Failed to create asset:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to create asset';
      set({ error: errorMessage });
      throw error;
    }
  },

  selectAsset: (asset: AssetInfo | null) => {
    set({ selectedAsset: asset, versions: [] });
  },

  fetchVersions: async (token: string, assetId: number) => {
    set({ versionsLoading: true, error: null });
    try {
      const versions = await invoke<ConfigurationVersionInfo[]>('get_configuration_versions', {
        token,
        assetId: assetId
      });
      set({ versions, versionsLoading: false });
    } catch (error) {
      console.error('Failed to fetch versions:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch versions',
        versionsLoading: false 
      });
    }
  },

  clearError: () => {
    set({ error: null });
  },

  refreshAssets: async (token: string) => {
    await get().fetchAssets(token);
  },

  setCurrentView: (view: 'dashboard' | 'history') => {
    set({ currentView: view });
  },

  navigateToHistory: (asset: AssetInfo) => {
    set({ 
      selectedAsset: asset, 
      currentView: 'history',
      versions: [] // Clear previous versions
    });
  },

  navigateToDashboard: () => {
    set({ 
      currentView: 'dashboard',
      selectedAsset: null,
      versions: []
    });
  },

  fetchGoldenVersion: async (token: string, assetId: number) => {
    set(state => ({ 
      goldenVersionsLoading: { ...state.goldenVersionsLoading, [assetId]: true } 
    }));
    
    try {
      const goldenVersion = await invoke<ConfigurationVersionInfo | null>('get_golden_version', {
        token,
        assetId
      });
      
      set(state => ({
        goldenVersions: { ...state.goldenVersions, [assetId]: goldenVersion },
        goldenVersionsLoading: { ...state.goldenVersionsLoading, [assetId]: false }
      }));
      
      return goldenVersion;
    } catch (error) {
      console.error('Failed to fetch golden version:', error);
      set(state => ({
        goldenVersionsLoading: { ...state.goldenVersionsLoading, [assetId]: false },
        error: error instanceof Error ? error.message : 'Failed to fetch golden version'
      }));
      throw error;
    }
  },

  promoteToGolden: async (token: string, versionId: number, reason: string) => {
    try {
      await invoke<void>('promote_to_golden', {
        token,
        versionId: versionId,
        promotionReason: reason
      });
      
      // After successful promotion, we should refresh the versions and golden version
      // This will be handled by the calling component
    } catch (error) {
      console.error('Failed to promote to golden:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to promote to golden';
      set({ error: errorMessage });
      throw error;
    }
  },

  checkPromotionEligibility: async (token: string, versionId: number) => {
    try {
      const isEligible = await invoke<boolean>('get_promotion_eligibility', {
        token,
        versionId: versionId
      });
      return isEligible;
    } catch (error) {
      console.error('Failed to check promotion eligibility:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to check promotion eligibility';
      set({ error: errorMessage });
      throw error;
    }
  },

  exportConfiguration: async (token: string, versionId: number, exportPath: string) => {
    try {
      await invoke<void>('export_configuration_version', {
        token,
        versionId: versionId,
        exportPath: exportPath
      });
      
      // No state updates needed for successful export
    } catch (error) {
      console.error('Failed to export configuration:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to export configuration';
      set({ error: errorMessage });
      throw error;
    }
  }
}));

export default useAssetStore;