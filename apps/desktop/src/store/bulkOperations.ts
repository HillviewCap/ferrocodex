import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import {
  BulkOperation,
  BulkOperationProgress,
  BulkOperationHistory,
  BulkMoveRequest,
  BulkDeleteRequest,
  BulkExportRequest,
  BulkClassifyRequest,
  ValidationResult,
  UndoResult,
  SelectionState,
  AssetSelectionInfo,
  BulkOperationUIState,
  SelectionManager,
  KeyboardSelectionState,
  BulkOperationType,
  BulkOperationStatus
} from '../types/bulkOperations';

interface BulkOperationsState extends SelectionManager {
  // Selection State
  selection: SelectionState;
  keyboardState: KeyboardSelectionState;
  
  // Operations State
  activeOperations: Map<string, BulkOperation>;
  operationProgress: Map<string, BulkOperationProgress>;
  operationHistory: BulkOperationHistory | null;
  
  // UI State
  ui: BulkOperationUIState;
  
  // Loading and Error State
  isLoading: boolean;
  error: string | null;
  
  // Selection Actions
  selectAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => void;
  deselectAsset: (assetId: number) => void;
  toggleAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => void;
  selectAll: (assetIds: number[], context: 'tree' | 'search') => void;
  selectNone: () => void;
  invertSelection: (availableAssetIds: number[]) => void;
  selectRange: (fromId: number, toId: number, assetIds: number[]) => void;
  isSelected: (assetId: number) => boolean;
  getSelectedCount: () => number;
  getSelectedAssets: () => number[];
  getSelectionInfo: (assetId: number) => AssetSelectionInfo | null;
  
  // Keyboard Navigation
  setKeyboardNavigationMode: (enabled: boolean) => void;
  setLastFocusedAsset: (assetId: number | null) => void;
  setSelectionAnchor: (assetId: number | null) => void;
  
  // Bulk Operation Actions
  startBulkMove: (request: BulkMoveRequest) => Promise<string>;
  startBulkDelete: (request: BulkDeleteRequest) => Promise<string>;
  startBulkExport: (request: BulkExportRequest) => Promise<string>;
  startBulkClassify: (request: BulkClassifyRequest) => Promise<string>;
  
  // Operation Monitoring
  getOperationProgress: (operationId: string) => Promise<BulkOperationProgress>;
  cancelOperation: (operationId: string) => Promise<void>;
  
  // Validation
  validateBulkMove: (assetIds: number[], newParentId: number | null) => Promise<ValidationResult>;
  validateBulkDelete: (assetIds: number[]) => Promise<ValidationResult>;
  validateBulkExport: (assetIds: number[], format: string) => Promise<ValidationResult>;
  validateBulkClassify: (assetIds: number[], classification: string) => Promise<ValidationResult>;
  
  // History and Undo
  loadOperationHistory: (userId?: number, limit?: number) => Promise<void>;
  undoOperation: (operationId: string) => Promise<UndoResult>;
  
  // UI Actions
  showSelectionToolbar: (show: boolean) => void;
  showProgressModal: (show: boolean) => void;
  showValidationDialog: (show: boolean) => void;
  showHistoryPanel: (show: boolean) => void;
  setCurrentOperation: (operation: BulkOperation | null) => void;
  setValidationResults: (results: ValidationResult | null) => void;
  setResolutionChoice: (conflictId: string, choiceId: string) => void;
  
  // Utility Actions
  setError: (error: string | null) => void;
  clearError: () => void;
  reset: () => void;
  resetSelection: () => void;
}

const useBulkOperationsStore = create<BulkOperationsState>()(
  persist(
    (set, get) => ({
      // Initial State
      selection: {
        selected_asset_ids: new Set(),
        selection_mode: 'none',
        last_selection_anchor: null,
        selection_metadata: {},
      },
      keyboardState: {
        last_focused_asset: null,
        selection_anchor: null,
        keyboard_navigation_mode: false,
      },
      activeOperations: new Map(),
      operationProgress: new Map(),
      operationHistory: null,
      ui: {
        showSelectionToolbar: false,
        showProgressModal: false,
        showValidationDialog: false,
        showHistoryPanel: false,
        currentOperation: null,
        validationResults: null,
        resolutionChoices: {},
      },
      isLoading: false,
      error: null,

      // Selection Actions
      selectAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => {
        const state = get();
        const newSelectedIds = new Set(state.selection.selected_asset_ids);
        newSelectedIds.add(assetId);
        
        const newMetadata = {
          ...state.selection.selection_metadata,
          [assetId]: {
            selected_at: new Date().toISOString(),
            context,
          } as AssetSelectionInfo,
        };
        
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: newSelectedIds,
            selection_mode: newSelectedIds.size > 1 ? 'multiple' : 'single',
            last_selection_anchor: assetId,
            selection_metadata: newMetadata,
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: newSelectedIds.size > 0,
          },
        });
      },

      deselectAsset: (assetId: number) => {
        const state = get();
        const newSelectedIds = new Set(state.selection.selected_asset_ids);
        newSelectedIds.delete(assetId);
        
        const newMetadata = { ...state.selection.selection_metadata };
        delete newMetadata[assetId];
        
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: newSelectedIds,
            selection_mode: newSelectedIds.size > 1 ? 'multiple' : newSelectedIds.size === 1 ? 'single' : 'none',
            selection_metadata: newMetadata,
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: newSelectedIds.size > 0,
          },
        });
      },

      toggleAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => {
        const state = get();
        if (state.selection.selected_asset_ids.has(assetId)) {
          get().deselectAsset(assetId);
        } else {
          get().selectAsset(assetId, context);
        }
      },

      selectAll: (assetIds: number[], context: 'tree' | 'search') => {
        const state = get();
        const newSelectedIds = new Set(assetIds);
        const timestamp = new Date().toISOString();
        
        const newMetadata: Record<number, AssetSelectionInfo> = {};
        assetIds.forEach(id => {
          newMetadata[id] = {
            selected_at: timestamp,
            context,
          };
        });
        
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: newSelectedIds,
            selection_mode: newSelectedIds.size > 1 ? 'multiple' : newSelectedIds.size === 1 ? 'single' : 'none',
            selection_metadata: newMetadata,
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: newSelectedIds.size > 0,
          },
        });
      },

      selectNone: () => {
        const state = get();
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: new Set(),
            selection_mode: 'none',
            last_selection_anchor: null,
            selection_metadata: {},
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: false,
          },
        });
      },

      invertSelection: (availableAssetIds: number[]) => {
        const state = get();
        const currentSelected = state.selection.selected_asset_ids;
        const newSelectedIds = new Set<number>();
        const timestamp = new Date().toISOString();
        const newMetadata: Record<number, AssetSelectionInfo> = {};
        
        availableAssetIds.forEach(id => {
          if (!currentSelected.has(id)) {
            newSelectedIds.add(id);
            newMetadata[id] = {
              selected_at: timestamp,
              context: 'manual',
            };
          }
        });
        
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: newSelectedIds,
            selection_mode: newSelectedIds.size > 1 ? 'multiple' : newSelectedIds.size === 1 ? 'single' : 'none',
            selection_metadata: newMetadata,
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: newSelectedIds.size > 0,
          },
        });
      },

      selectRange: (fromId: number, toId: number, assetIds: number[]) => {
        const fromIndex = assetIds.indexOf(fromId);
        const toIndex = assetIds.indexOf(toId);
        
        if (fromIndex === -1 || toIndex === -1) return;
        
        const startIndex = Math.min(fromIndex, toIndex);
        const endIndex = Math.max(fromIndex, toIndex);
        const rangeIds = assetIds.slice(startIndex, endIndex + 1);
        
        const state = get();
        const newSelectedIds = new Set(state.selection.selected_asset_ids);
        const timestamp = new Date().toISOString();
        const newMetadata = { ...state.selection.selection_metadata };
        
        rangeIds.forEach(id => {
          newSelectedIds.add(id);
          newMetadata[id] = {
            selected_at: timestamp,
            context: 'manual',
          };
        });
        
        set({
          selection: {
            ...state.selection,
            selected_asset_ids: newSelectedIds,
            selection_mode: newSelectedIds.size > 1 ? 'multiple' : 'single',
            selection_metadata: newMetadata,
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: newSelectedIds.size > 0,
          },
        });
      },

      isSelected: (assetId: number) => {
        return get().selection.selected_asset_ids.has(assetId);
      },

      getSelectedCount: () => {
        return get().selection.selected_asset_ids.size;
      },

      getSelectedAssets: () => {
        return Array.from(get().selection.selected_asset_ids);
      },

      getSelectionInfo: (assetId: number) => {
        return get().selection.selection_metadata[assetId] || null;
      },

      // Keyboard Navigation
      setKeyboardNavigationMode: (enabled: boolean) => {
        const state = get();
        set({
          keyboardState: {
            ...state.keyboardState,
            keyboard_navigation_mode: enabled,
          },
        });
      },

      setLastFocusedAsset: (assetId: number | null) => {
        const state = get();
        set({
          keyboardState: {
            ...state.keyboardState,
            last_focused_asset: assetId,
          },
        });
      },

      setSelectionAnchor: (assetId: number | null) => {
        const state = get();
        set({
          keyboardState: {
            ...state.keyboardState,
            selection_anchor: assetId,
          },
        });
      },

      // Bulk Operation Actions
      startBulkMove: async (request: BulkMoveRequest) => {
        set({ isLoading: true, error: null });
        try {
          const operationId = await invoke<string>('start_bulk_move', {
            assetIds: request.asset_ids,
            newParentId: request.new_parent_id,
            options: request.options,
          });
          
          // Start monitoring progress
          get().getOperationProgress(operationId);
          
          return operationId;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to start bulk move operation';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      startBulkDelete: async (request: BulkDeleteRequest) => {
        set({ isLoading: true, error: null });
        try {
          const operationId = await invoke<string>('start_bulk_delete', {
            assetIds: request.asset_ids,
            options: request.options,
          });
          
          // Start monitoring progress
          get().getOperationProgress(operationId);
          
          return operationId;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to start bulk delete operation';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      startBulkExport: async (request: BulkExportRequest) => {
        set({ isLoading: true, error: null });
        try {
          const operationId = await invoke<string>('start_bulk_export', {
            assetIds: request.asset_ids,
            format: request.format,
            options: request.options,
          });
          
          // Start monitoring progress
          get().getOperationProgress(operationId);
          
          return operationId;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to start bulk export operation';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      startBulkClassify: async (request: BulkClassifyRequest) => {
        set({ isLoading: true, error: null });
        try {
          const operationId = await invoke<string>('start_bulk_classify', {
            assetIds: request.asset_ids,
            newClassification: request.new_classification,
            applyToChildren: request.apply_to_children,
          });
          
          // Start monitoring progress
          get().getOperationProgress(operationId);
          
          return operationId;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to start bulk classify operation';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      // Operation Monitoring
      getOperationProgress: async (operationId: string) => {
        try {
          const progress = await invoke<BulkOperationProgress>('get_bulk_operation_progress', {
            operationId,
          });
          
          const state = get();
          const newProgressMap = new Map(state.operationProgress);
          newProgressMap.set(operationId, progress);
          
          set({
            operationProgress: newProgressMap,
            isLoading: false,
          });
          
          // Continue polling if still processing
          if (progress.status === 'processing' || progress.status === 'validating') {
            setTimeout(() => get().getOperationProgress(operationId), 1000);
          }
          
          return progress;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to get operation progress';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      cancelOperation: async (operationId: string) => {
        set({ error: null });
        try {
          await invoke('cancel_bulk_operation', { operationId });
          
          // Update operation progress
          get().getOperationProgress(operationId);
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to cancel operation';
          set({ error: errorMessage });
          throw error;
        }
      },

      // Validation
      validateBulkMove: async (assetIds: number[], newParentId: number | null) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationResult>('validate_bulk_move', {
            assetIds,
            newParentId,
          });
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to validate bulk move';
          set({ error: errorMessage });
          throw error;
        }
      },

      validateBulkDelete: async (assetIds: number[]) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationResult>('validate_bulk_delete', {
            assetIds,
          });
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to validate bulk delete';
          set({ error: errorMessage });
          throw error;
        }
      },

      validateBulkExport: async (assetIds: number[], format: string) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationResult>('validate_bulk_export', {
            assetIds,
            format,
          });
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to validate bulk export';
          set({ error: errorMessage });
          throw error;
        }
      },

      validateBulkClassify: async (assetIds: number[], classification: string) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationResult>('validate_bulk_classify', {
            assetIds,
            classification,
          });
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to validate bulk classify';
          set({ error: errorMessage });
          throw error;
        }
      },

      // History and Undo
      loadOperationHistory: async (userId?: number, limit?: number) => {
        set({ isLoading: true, error: null });
        try {
          const history = await invoke<BulkOperationHistory>('get_bulk_operation_history', {
            userId,
            limit,
          });
          set({ operationHistory: history, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load operation history';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      undoOperation: async (operationId: string) => {
        set({ isLoading: true, error: null });
        try {
          const result = await invoke<UndoResult>('undo_bulk_operation', {
            operationId,
          });
          
          // Refresh operation history
          await get().loadOperationHistory();
          
          set({ isLoading: false });
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to undo operation';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      // UI Actions
      showSelectionToolbar: (show: boolean) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            showSelectionToolbar: show,
          },
        });
      },

      showProgressModal: (show: boolean) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            showProgressModal: show,
          },
        });
      },

      showValidationDialog: (show: boolean) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            showValidationDialog: show,
          },
        });
      },

      showHistoryPanel: (show: boolean) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            showHistoryPanel: show,
          },
        });
      },

      setCurrentOperation: (operation: BulkOperation | null) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            currentOperation: operation,
          },
        });
      },

      setValidationResults: (results: ValidationResult | null) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            validationResults: results,
          },
        });
      },

      setResolutionChoice: (conflictId: string, choiceId: string) => {
        const state = get();
        set({
          ui: {
            ...state.ui,
            resolutionChoices: {
              ...state.ui.resolutionChoices,
              [conflictId]: choiceId,
            },
          },
        });
      },

      // Utility Actions
      setError: (error: string | null) => {
        set({ error });
      },

      clearError: () => {
        set({ error: null });
      },

      reset: () => {
        set({
          selection: {
            selected_asset_ids: new Set(),
            selection_mode: 'none',
            last_selection_anchor: null,
            selection_metadata: {},
          },
          keyboardState: {
            last_focused_asset: null,
            selection_anchor: null,
            keyboard_navigation_mode: false,
          },
          activeOperations: new Map(),
          operationProgress: new Map(),
          operationHistory: null,
          ui: {
            showSelectionToolbar: false,
            showProgressModal: false,
            showValidationDialog: false,
            showHistoryPanel: false,
            currentOperation: null,
            validationResults: null,
            resolutionChoices: {},
          },
          isLoading: false,
          error: null,
        });
      },

      resetSelection: () => {
        const state = get();
        set({
          selection: {
            selected_asset_ids: new Set(),
            selection_mode: 'none',
            last_selection_anchor: null,
            selection_metadata: {},
          },
          ui: {
            ...state.ui,
            showSelectionToolbar: false,
          },
        });
      },
    }),
    {
      name: 'bulk-operations-storage',
      partialize: (state) => ({
        // Only persist selection state, not operation progress or UI state
        selection: {
          ...state.selection,
          selected_asset_ids: Array.from(state.selection.selected_asset_ids),
        },
      }),
      onRehydrateStorage: () => (state) => {
        if (state && state.selection) {
          // Convert persisted array back to Set
          state.selection.selected_asset_ids = new Set(state.selection.selected_asset_ids as unknown as number[]);
        }
      },
    }
  )
);

export default useBulkOperationsStore;