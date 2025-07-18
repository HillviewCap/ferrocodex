import { create } from 'zustand';
import { AssetInfo, ConfigurationVersionInfo } from '../types/assets';

interface AssetState {
  assets: AssetInfo[];
  isLoading: boolean;
  error: string | null;
  selectedAsset: AssetInfo | null;
  versions: ConfigurationVersionInfo[];
  versionsLoading: boolean;
  currentView: 'dashboard' | 'history';
  
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
}

const useAssetStore = create<AssetState>((set, get) => ({
  assets: [],
  isLoading: false,
  error: null,
  selectedAsset: null,
  versions: [],
  versionsLoading: false,
  currentView: 'dashboard',

  fetchAssets: async (token: string) => {
    set({ isLoading: true, error: null });
    try {
      const assets = await window.__TAURI__.invoke('get_dashboard_assets', { token });
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
      const asset = await window.__TAURI__.invoke('create_asset', {
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
      const versions = await window.__TAURI__.invoke('get_configuration_versions', {
        token,
        asset_id: assetId
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
  }
}));

export default useAssetStore;