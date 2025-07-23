import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { FirmwareVersionInfo, UploadFirmwareRequest, FirmwareUploadProgress, FirmwareStatus } from '../types/firmware';
import useAuthStore from './auth';

interface FirmwareState {
  firmwareVersions: Record<number, FirmwareVersionInfo[]>; // Keyed by asset_id
  selectedFirmware: FirmwareVersionInfo | null;
  isLoading: boolean;
  isUploading: boolean;
  uploadProgress: FirmwareUploadProgress | null;
  error: string | null;
}

interface FirmwareActions {
  loadFirmwareVersions: (assetId: number) => Promise<void>;
  uploadFirmware: (request: UploadFirmwareRequest) => Promise<void>;
  deleteFirmware: (firmwareId: number) => Promise<void>;
  updateFirmwareStatus: (firmwareId: number, newStatus: FirmwareStatus, reason?: string) => Promise<void>;
  getAvailableStatusTransitions: (firmwareId: number) => Promise<FirmwareStatus[]>;
  promoteFirmwareToGolden: (firmwareId: number, reason: string) => Promise<void>;
  updateFirmwareNotes: (firmwareId: number, notes: string) => Promise<void>;
  setSelectedFirmware: (firmware: FirmwareVersionInfo | null) => void;
  clearError: () => void;
  resetUploadProgress: () => void;
}

const useFirmwareStore = create<FirmwareState & FirmwareActions>((set, get) => ({
  // State
  firmwareVersions: {},
  selectedFirmware: null,
  isLoading: false,
  isUploading: false,
  uploadProgress: null,
  error: null,

  // Actions
  loadFirmwareVersions: async (assetId: number) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ isLoading: true, error: null });
      
      const versions: FirmwareVersionInfo[] = await invoke('get_firmware_list', {
        token,
        assetId,
      });

      set(state => ({
        firmwareVersions: {
          ...state.firmwareVersions,
          [assetId]: versions,
        },
        isLoading: false,
      }));
    } catch (error) {
      set({ 
        isLoading: false, 
        error: error as string 
      });
      throw error;
    }
  },

  uploadFirmware: async (request: UploadFirmwareRequest) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ 
        isUploading: true, 
        error: null,
        uploadProgress: { 
          progress: 0, 
          status: 'preparing' 
        }
      });

      // Update progress to uploading
      set({ 
        uploadProgress: { 
          progress: 25, 
          status: 'uploading' 
        }
      });

      await invoke('upload_firmware', {
        token,
        assetId: request.asset_id,
        vendor: request.vendor,
        model: request.model,
        version: request.version,
        notes: request.notes,
        filePath: request.file_path,
      });

      // Update progress to processing
      set({ 
        uploadProgress: { 
          progress: 75, 
          status: 'processing' 
        }
      });

      // Reload firmware versions
      await get().loadFirmwareVersions(request.asset_id);

      set({ 
        isUploading: false,
        uploadProgress: { 
          progress: 100, 
          status: 'complete' 
        }
      });
    } catch (error) {
      set({ 
        isUploading: false, 
        error: error as string,
        uploadProgress: { 
          progress: 0, 
          status: 'error',
          message: error as string
        }
      });
      throw error;
    }
  },

  deleteFirmware: async (firmwareId: number) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ error: null });
      
      await invoke('delete_firmware', {
        token,
        firmwareId,
      });

      // Remove from state
      const firmwareVersions = get().firmwareVersions;
      const updatedVersions: Record<number, FirmwareVersionInfo[]> = {};
      
      for (const [assetId, versions] of Object.entries(firmwareVersions)) {
        updatedVersions[Number(assetId)] = versions.filter(v => v.id !== firmwareId);
      }

      set({ firmwareVersions: updatedVersions });
    } catch (error) {
      set({ error: error as string });
      throw error;
    }
  },

  setSelectedFirmware: (firmware: FirmwareVersionInfo | null) => {
    set({ selectedFirmware: firmware });
  },

  updateFirmwareStatus: async (firmwareId: number, newStatus: FirmwareStatus, reason?: string) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ error: null });
      
      await invoke('update_firmware_status', {
        token,
        firmwareId,
        newStatus,
        reason,
      });

      // Update the firmware in state
      const firmwareVersions = get().firmwareVersions;
      const updatedVersions: Record<number, FirmwareVersionInfo[]> = {};
      
      for (const [assetId, versions] of Object.entries(firmwareVersions)) {
        updatedVersions[Number(assetId)] = versions.map(v => 
          v.id === firmwareId 
            ? { ...v, status: newStatus, status_changed_at: new Date().toISOString() }
            : v
        );
      }

      set({ firmwareVersions: updatedVersions });
    } catch (error) {
      set({ error: error as string });
      throw error;
    }
  },

  getAvailableStatusTransitions: async (firmwareId: number): Promise<FirmwareStatus[]> => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      const transitions = await invoke<FirmwareStatus[]>('get_available_firmware_status_transitions', {
        token,
        firmwareId,
      });
      return transitions;
    } catch (error) {
      throw error;
    }
  },

  promoteFirmwareToGolden: async (firmwareId: number, reason: string) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ error: null });
      
      await invoke('promote_firmware_to_golden', {
        token,
        firmwareId,
        reason,
      });

      // Reload all firmware versions as Golden promotion affects multiple versions
      const firmwareVersions = get().firmwareVersions;
      for (const assetId of Object.keys(firmwareVersions)) {
        await get().loadFirmwareVersions(Number(assetId));
      }
    } catch (error) {
      set({ error: error as string });
      throw error;
    }
  },

  updateFirmwareNotes: async (firmwareId: number, notes: string) => {
    const token = useAuthStore.getState().token;
    if (!token) {
      throw new Error('Not authenticated');
    }

    try {
      set({ error: null });
      
      await invoke('update_firmware_notes', {
        token,
        firmwareId,
        notes,
      });

      // Update the firmware in state
      const firmwareVersions = get().firmwareVersions;
      const updatedVersions: Record<number, FirmwareVersionInfo[]> = {};
      
      for (const [assetId, versions] of Object.entries(firmwareVersions)) {
        updatedVersions[Number(assetId)] = versions.map(v => 
          v.id === firmwareId 
            ? { ...v, notes }
            : v
        );
      }

      set({ firmwareVersions: updatedVersions });
    } catch (error) {
      set({ error: error as string });
      throw error;
    }
  },

  clearError: () => set({ error: null }),

  resetUploadProgress: () => set({ 
    uploadProgress: null,
    isUploading: false 
  }),
}));

export default useFirmwareStore;