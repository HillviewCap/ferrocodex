import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface AppState {
  isFirstLaunch: boolean | null;
  isDatabaseInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  eulaAccepted: boolean;
}

interface AppActions {
  checkFirstLaunch: () => Promise<void>;
  initializeDatabase: () => Promise<void>;
  setFirstLaunchComplete: () => void;
  setLoading: (loading: boolean) => void;
  clearError: () => void;
  acceptEula: () => void;
}

const useAppStore = create<AppState & AppActions>((set) => ({
  // State
  isFirstLaunch: null,
  isDatabaseInitialized: false,
  isLoading: false,
  error: null,
  eulaAccepted: false,

  // Actions
  checkFirstLaunch: async () => {
    try {
      set({ isLoading: true, error: null });
      console.log('Checking if this is first launch...');
      const isFirstLaunch: boolean = await invoke('is_first_launch');
      console.log('Is first launch result:', isFirstLaunch);
      set({ isFirstLaunch, isLoading: false });
    } catch (error) {
      console.error('Failed to check first launch:', error);
      set({ 
        error: error as string, 
        isLoading: false,
        isFirstLaunch: false
      });
    }
  },

  initializeDatabase: async () => {
    try {
      set({ isLoading: true, error: null });
      const success: boolean = await invoke('initialize_database');
      if (success) {
        const healthCheck: boolean = await invoke('database_health_check');
        set({ 
          isDatabaseInitialized: healthCheck, 
          isLoading: false 
        });
      } else {
        throw new Error('Database initialization failed');
      }
    } catch (error) {
      set({ 
        error: error as string, 
        isLoading: false,
        isDatabaseInitialized: false
      });
    }
  },

  setFirstLaunchComplete: () => set({ isFirstLaunch: false }),
  
  setLoading: (loading: boolean) => set({ isLoading: loading }),
  
  clearError: () => set({ error: null }),
  
  acceptEula: () => set({ eulaAccepted: true }),
}));

export default useAppStore;