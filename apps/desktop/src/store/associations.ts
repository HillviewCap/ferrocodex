import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { 
  AssetFileAssociation, 
  AssociationInfo, 
  FileImportSession, 
  AssociationSummary,
  HealthStatus,
  CreateAssociationRequest,
  AssociationType,
  ValidationResult,
  FileAssociationWizardState,
  AssetInfo
} from '../types/associations';

interface AssociationState {
  // Data state
  associations: AssociationInfo[];
  associationSummaries: Map<number, AssociationSummary>;
  importSessions: Map<number, FileImportSession>;
  healthStatuses: Map<number, HealthStatus>;
  
  // UI state
  loading: boolean;
  error: string | null;
  wizardState: FileAssociationWizardState | null;
  selectedAssetId: number | null;
  searchQuery: string;
  filterByType: AssociationType | null;
  filterByValidation: ValidationResult | null;
  
  // Actions
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  
  // Association management
  loadAssociations: (assetId: number) => Promise<void>;
  createAssociation: (request: CreateAssociationRequest) => Promise<AssetFileAssociation>;
  removeAssociation: (associationId: number) => Promise<void>;
  reorderAssociations: (assetId: number, associationOrder: Array<[number, number]>) => Promise<void>;
  
  // Search and filtering
  searchAssociations: (query: string, fileType?: AssociationType) => Promise<void>;
  setSearchQuery: (query: string) => void;
  setFilterByType: (type: AssociationType | null) => void;
  setFilterByValidation: (status: ValidationResult | null) => void;
  
  // Health monitoring
  loadHealthStatus: (assetId: number) => Promise<void>;
  repairAssociation: (associationId: number) => Promise<void>;
  
  // Import workflow
  startImportWizard: (assetId?: number) => void;
  updateWizardState: (updates: Partial<FileAssociationWizardState>) => void;
  nextWizardStep: () => void;
  previousWizardStep: () => void;
  completeWizard: () => Promise<void>;
  cancelWizard: () => void;
  
  // Utility actions
  clearError: () => void;
  refreshData: (assetId?: number) => Promise<void>;
  reset: () => void;
}

const initialWizardState: FileAssociationWizardState = {
  currentStep: 0,
  selectedFiles: [],
  uploadedFiles: [],
  associations: [],
  validationResults: [],
};

export const useAssociationStore = create<AssociationState>()(
  persist(
    (set, get) => ({
      // Initial state
      associations: [],
      associationSummaries: new Map(),
      importSessions: new Map(),
      healthStatuses: new Map(),
      loading: false,
      error: null,
      wizardState: null,
      selectedAssetId: null,
      searchQuery: '',
      filterByType: null,
      filterByValidation: null,

      // Basic state actions
      setLoading: (loading) => set({ loading }),
      setError: (error) => set({ error }),
      clearError: () => set({ error: null }),

      // Association management actions
      loadAssociations: async (assetId: number) => {
        set({ loading: true, error: null });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const associations = await invoke<AssociationInfo[]>('get_asset_file_associations', { assetId });
          
          set({ 
            associations, 
            selectedAssetId: assetId,
            loading: false 
          });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load associations';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      createAssociation: async (request: CreateAssociationRequest) => {
        set({ loading: true, error: null });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const association = await invoke<AssetFileAssociation>('create_file_association', { request });
          
          // Refresh associations for the asset
          await get().loadAssociations(request.assetId);
          
          set({ loading: false });
          return association;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to create association';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      removeAssociation: async (associationId: number) => {
        set({ loading: true, error: null });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('remove_file_association', { associationId });
          
          // Remove from local state
          set(state => ({
            associations: state.associations.filter(a => a.id !== associationId),
            loading: false
          }));
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to remove association';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      reorderAssociations: async (assetId: number, associationOrder: Array<[number, number]>) => {
        set({ loading: true, error: null });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('reorder_file_associations', { assetId, associationOrder });
          
          // Refresh associations to get updated order
          await get().loadAssociations(assetId);
          
          set({ loading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to reorder associations';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      // Search and filtering
      searchAssociations: async (query: string, fileType?: AssociationType) => {
        set({ loading: true, error: null, searchQuery: query });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const associations = await invoke<AssociationInfo[]>('search_associations', { 
            query, 
            fileType: fileType || null 
          });
          
          set({ associations, loading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Search failed';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      setSearchQuery: (query) => set({ searchQuery: query }),
      setFilterByType: (type) => set({ filterByType: type }),
      setFilterByValidation: (status) => set({ filterByValidation: status }),

      // Health monitoring
      loadHealthStatus: async (assetId: number) => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const healthStatus = await invoke<HealthStatus>('get_association_health_status', { assetId });
          
          set(state => ({
            healthStatuses: new Map(state.healthStatuses.set(assetId, healthStatus))
          }));
        } catch (error) {
          console.error('Failed to load health status:', error);
        }
      },

      repairAssociation: async (associationId: number) => {
        set({ loading: true, error: null });
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('repair_association', { associationId });
          
          // Refresh associations after repair
          const { selectedAssetId } = get();
          if (selectedAssetId) {
            await get().loadAssociations(selectedAssetId);
          }
          
          set({ loading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to repair association';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      // Import wizard management
      startImportWizard: (assetId?: number) => {
        const wizardState: FileAssociationWizardState = {
          ...initialWizardState,
          selectedAsset: assetId ? get().associations.find(a => a.assetId === assetId)?.assetName ? {
            id: assetId,
            name: get().associations.find(a => a.assetId === assetId)?.assetName || '',
            description: '',
            type: 'Device'
          } : undefined : undefined,
        };
        
        set({ wizardState });
      },

      updateWizardState: (updates) => {
        set(state => ({
          wizardState: state.wizardState ? {
            ...state.wizardState,
            ...updates
          } : null
        }));
      },

      nextWizardStep: () => {
        set(state => ({
          wizardState: state.wizardState ? {
            ...state.wizardState,
            currentStep: Math.min(state.wizardState.currentStep + 1, 3) // Assuming 4 steps (0-3)
          } : null
        }));
      },

      previousWizardStep: () => {
        set(state => ({
          wizardState: state.wizardState ? {
            ...state.wizardState,
            currentStep: Math.max(state.wizardState.currentStep - 1, 0)
          } : null
        }));
      },

      completeWizard: async () => {
        const { wizardState } = get();
        if (!wizardState) return;

        set({ loading: true, error: null });
        try {
          // Create all associations
          for (const association of wizardState.associations) {
            await get().createAssociation(association);
          }
          
          // Clear wizard state
          set({ wizardState: null, loading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to complete import';
          set({ error: errorMessage, loading: false });
          throw error;
        }
      },

      cancelWizard: () => {
        set({ wizardState: null });
      },

      // Utility actions
      refreshData: async (assetId?: number) => {
        const targetAssetId = assetId || get().selectedAssetId;
        if (targetAssetId) {
          await Promise.all([
            get().loadAssociations(targetAssetId),
            get().loadHealthStatus(targetAssetId)
          ]);
        }
      },

      reset: () => {
        set({
          associations: [],
          associationSummaries: new Map(),
          importSessions: new Map(),
          healthStatuses: new Map(),
          loading: false,
          error: null,
          wizardState: null,
          selectedAssetId: null,
          searchQuery: '',
          filterByType: null,
          filterByValidation: null,
        });
      },
    }),
    {
      name: 'association-store',
      partialize: (state) => ({
        searchQuery: state.searchQuery,
        filterByType: state.filterByType,
        filterByValidation: state.filterByValidation,
        selectedAssetId: state.selectedAssetId,
      }),
    }
  )
);

export default useAssociationStore;