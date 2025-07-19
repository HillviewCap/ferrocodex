import { create } from 'zustand';
import { BranchInfo, CreateBranchRequest, BranchVersionInfo, CreateBranchVersionRequest } from '../types/branches';

interface BranchState {
  branches: BranchInfo[];
  isLoading: boolean;
  error: string | null;
  selectedBranch: BranchInfo | null;
  
  // Branch version state
  branchVersions: { [branchId: number]: BranchVersionInfo[] };
  isLoadingVersions: boolean;
  versionsError: string | null;
  selectedBranchVersion: BranchVersionInfo | null;
  isImportingVersion: boolean;
  
  // Actions
  fetchBranches: (token: string, assetId: number) => Promise<void>;
  createBranch: (token: string, request: CreateBranchRequest) => Promise<BranchInfo>;
  getBranchDetails: (token: string, branchId: number) => Promise<BranchInfo>;
  selectBranch: (branch: BranchInfo | null) => void;
  clearError: () => void;
  refreshBranches: (token: string, assetId: number) => Promise<void>;
  addBranch: (branch: BranchInfo) => void;
  updateBranch: (branchId: number, updates: Partial<BranchInfo>) => void;
  removeBranch: (branchId: number) => void;
  
  // Branch version actions
  fetchBranchVersions: (token: string, branchId: number, page?: number, limit?: number) => Promise<void>;
  importVersionToBranch: (token: string, request: CreateBranchVersionRequest) => Promise<BranchVersionInfo>;
  getBranchLatestVersion: (token: string, branchId: number) => Promise<BranchVersionInfo | null>;
  compareBranchVersions: (token: string, branchId: number, version1Id: number, version2Id: number) => Promise<string>;
  selectBranchVersion: (version: BranchVersionInfo | null) => void;
  clearVersionsError: () => void;
  addBranchVersion: (branchId: number, version: BranchVersionInfo) => void;
  refreshBranchVersions: (token: string, branchId: number) => Promise<void>;
}

const useBranchStore = create<BranchState>((set, get) => ({
  branches: [],
  isLoading: false,
  error: null,
  selectedBranch: null,
  
  // Branch version state
  branchVersions: {},
  isLoadingVersions: false,
  versionsError: null,
  selectedBranchVersion: null,
  isImportingVersion: false,

  fetchBranches: async (token: string, assetId: number) => {
    set({ isLoading: true, error: null });
    try {
      const branches = await window.__TAURI__.invoke('get_branches', { 
        token, 
        asset_id: assetId 
      });
      set({ branches, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch branches:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch branches',
        isLoading: false 
      });
    }
  },

  createBranch: async (token: string, request: CreateBranchRequest) => {
    try {
      const branch = await window.__TAURI__.invoke('create_branch', {
        token,
        name: request.name,
        description: request.description || null,
        asset_id: request.asset_id,
        parent_version_id: request.parent_version_id
      });
      
      // Add to the beginning of the branches array
      set(state => ({
        branches: [branch, ...state.branches]
      }));
      
      return branch;
    } catch (error) {
      console.error('Failed to create branch:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to create branch';
      set({ error: errorMessage });
      throw error;
    }
  },

  getBranchDetails: async (token: string, branchId: number) => {
    try {
      const branch = await window.__TAURI__.invoke('get_branch_details', {
        token,
        branch_id: branchId
      });
      return branch;
    } catch (error) {
      console.error('Failed to get branch details:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to get branch details';
      set({ error: errorMessage });
      throw error;
    }
  },

  selectBranch: (branch: BranchInfo | null) => {
    set({ selectedBranch: branch });
  },

  clearError: () => {
    set({ error: null });
  },

  refreshBranches: async (token: string, assetId: number) => {
    await get().fetchBranches(token, assetId);
  },

  addBranch: (branch: BranchInfo) => {
    set(state => ({
      branches: [branch, ...state.branches]
    }));
  },

  updateBranch: (branchId: number, updates: Partial<BranchInfo>) => {
    set(state => ({
      branches: state.branches.map(branch => 
        branch.id === branchId 
          ? { ...branch, ...updates }
          : branch
      )
    }));
  },

  removeBranch: (branchId: number) => {
    set(state => ({
      branches: state.branches.filter(branch => branch.id !== branchId),
      selectedBranch: state.selectedBranch?.id === branchId ? null : state.selectedBranch
    }));
  },

  // Branch version actions
  fetchBranchVersions: async (token: string, branchId: number, page?: number, limit?: number) => {
    set({ isLoadingVersions: true, versionsError: null });
    try {
      const versions = await window.__TAURI__.invoke('get_branch_versions', {
        token,
        branch_id: branchId,
        page: page || 1,
        limit: limit || 50
      });
      set(state => ({
        branchVersions: {
          ...state.branchVersions,
          [branchId]: versions
        },
        isLoadingVersions: false
      }));
    } catch (error) {
      console.error('Failed to fetch branch versions:', error);
      set({
        versionsError: error instanceof Error ? error.message : 'Failed to fetch branch versions',
        isLoadingVersions: false
      });
    }
  },

  importVersionToBranch: async (token: string, request: CreateBranchVersionRequest) => {
    set({ isImportingVersion: true, versionsError: null });
    try {
      const versionInfo = await window.__TAURI__.invoke('import_version_to_branch', {
        token,
        branch_id: request.branch_id,
        file_path: request.file_path,
        notes: request.notes
      });
      
      // Add to the beginning of the versions array for this branch
      set(state => ({
        branchVersions: {
          ...state.branchVersions,
          [request.branch_id]: [versionInfo, ...(state.branchVersions[request.branch_id] || [])]
        },
        isImportingVersion: false
      }));
      
      return versionInfo;
    } catch (error) {
      console.error('Failed to import version to branch:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to import version to branch';
      set({ versionsError: errorMessage, isImportingVersion: false });
      throw error;
    }
  },

  getBranchLatestVersion: async (token: string, branchId: number) => {
    try {
      const version = await window.__TAURI__.invoke('get_branch_latest_version', {
        token,
        branch_id: branchId
      });
      return version || null;
    } catch (error) {
      console.error('Failed to get branch latest version:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to get branch latest version';
      set({ versionsError: errorMessage });
      throw error;
    }
  },

  compareBranchVersions: async (token: string, branchId: number, version1Id: number, version2Id: number) => {
    try {
      const diff = await window.__TAURI__.invoke('compare_branch_versions', {
        token,
        branch_id: branchId,
        version1_id: version1Id,
        version2_id: version2Id
      });
      return diff;
    } catch (error) {
      console.error('Failed to compare branch versions:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to compare branch versions';
      set({ versionsError: errorMessage });
      throw error;
    }
  },

  selectBranchVersion: (version: BranchVersionInfo | null) => {
    set({ selectedBranchVersion: version });
  },

  clearVersionsError: () => {
    set({ versionsError: null });
  },

  addBranchVersion: (branchId: number, version: BranchVersionInfo) => {
    set(state => ({
      branchVersions: {
        ...state.branchVersions,
        [branchId]: [version, ...(state.branchVersions[branchId] || [])]
      }
    }));
  },

  refreshBranchVersions: async (token: string, branchId: number) => {
    await get().fetchBranchVersions(token, branchId);
  }
}));

export default useBranchStore;