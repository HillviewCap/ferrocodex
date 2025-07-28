import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { RequestTracker, RequestContext } from '../utils/requestTracking';

interface AppState {
  isFirstLaunch: boolean | null;
  isDatabaseInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  eulaAccepted: boolean;
  currentRequestId: string | null;
  activeRequests: RequestContext[];
}

interface AppActions {
  checkFirstLaunch: () => Promise<void>;
  initializeDatabase: () => Promise<void>;
  setFirstLaunchComplete: () => void;
  setLoading: (loading: boolean) => void;
  clearError: () => void;
  acceptEula: () => void;
  startRequest: (operation: string, component: string) => RequestContext;
  completeRequest: (requestId: string) => void;
  getActiveRequests: () => RequestContext[];
  updateActiveRequests: () => void;
}

const useAppStore = create<AppState & AppActions>((set, get) => ({
  // State
  isFirstLaunch: null,
  isDatabaseInitialized: false,
  isLoading: false,
  error: null,
  eulaAccepted: false,
  currentRequestId: null,
  activeRequests: [],

  // Actions
  checkFirstLaunch: async () => {
    const context = RequestTracker.createContext('checkFirstLaunch', 'AppStore');
    
    try {
      set({ isLoading: true, error: null, currentRequestId: context.requestId });
      console.log('Checking if this is first launch...');
      
      const isFirstLaunch: boolean = await invoke('is_first_launch', {
        request_id: context.requestId
      });
      
      console.log('Is first launch result:', isFirstLaunch);
      set({ isFirstLaunch, isLoading: false, currentRequestId: null });
      RequestTracker.completeRequest(context.requestId);
    } catch (error) {
      console.error('Failed to check first launch:', error);
      set({ 
        error: error as string, 
        isLoading: false,
        isFirstLaunch: false,
        currentRequestId: null
      });
      RequestTracker.completeRequest(context.requestId);
    }
  },

  initializeDatabase: async () => {
    const context = RequestTracker.createContext('initializeDatabase', 'AppStore');
    
    try {
      set({ isLoading: true, error: null, currentRequestId: context.requestId });
      
      const success: boolean = await invoke('initialize_database', {
        request_id: context.requestId
      });
      
      if (success) {
        const healthCheck: boolean = await invoke('database_health_check', {
          request_id: context.requestId
        });
        
        set({ 
          isDatabaseInitialized: healthCheck, 
          isLoading: false,
          currentRequestId: null
        });
      } else {
        throw new Error('Database initialization failed');
      }
      
      RequestTracker.completeRequest(context.requestId);
    } catch (error) {
      set({ 
        error: error as string, 
        isLoading: false,
        isDatabaseInitialized: false,
        currentRequestId: null
      });
      RequestTracker.completeRequest(context.requestId);
    }
  },

  setFirstLaunchComplete: () => set({ isFirstLaunch: false }),
  
  setLoading: (loading: boolean) => set({ isLoading: loading }),
  
  clearError: () => set({ error: null }),
  
  acceptEula: () => set({ eulaAccepted: true }),

  // Request tracking actions
  startRequest: (operation: string, component: string) => {
    const context = RequestTracker.createContext(operation, component);
    set({ currentRequestId: context.requestId });
    get().updateActiveRequests();
    return context;
  },

  completeRequest: (requestId: string) => {
    RequestTracker.completeRequest(requestId);
    const currentId = get().currentRequestId;
    if (currentId === requestId) {
      set({ currentRequestId: null });
    }
    get().updateActiveRequests();
  },

  getActiveRequests: () => {
    return RequestTracker.getContext('') ? [RequestTracker.getContext('')].filter(Boolean) as RequestContext[] : [];
  },

  updateActiveRequests: () => {
    // This would update the active requests list from the RequestTracker
    // For now, we'll keep it simple since the RequestTracker manages the active requests
    RequestTracker.getPerformanceStats().sampleCount;
    // We don't expose all internal active requests to avoid performance issues
    // Instead, we just track the current request ID
  },
}));

export default useAppStore;