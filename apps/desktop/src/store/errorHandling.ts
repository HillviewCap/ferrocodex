import { create } from 'zustand';
import { 
  ProcessedErrorInfo,
  ErrorProcessingContext,
  RecoveryAction,
  processErrorWithContext,
  setErrorContext,
  clearErrorContext
} from '../utils/errorHandling';
import { showEnhancedErrorNotification } from '../components/error/EnhancedErrorNotification';

interface ErrorState {
  // Current error context
  currentContext: ErrorProcessingContext;
  
  // Recent errors for tracking and analysis
  recentErrors: ProcessedErrorInfo[];
  
  // Active recovery workflows
  activeRecoveries: Map<string, {
    errorId: string;
    actions: RecoveryAction[];
    startTime: Date;
    status: 'in_progress' | 'completed' | 'failed';
  }>;
  
  // Error display preferences
  showTechnicalDetails: boolean;
  autoShowRecoveryActions: boolean;
  notificationDuration: number;
}

interface ErrorActions {
  // Context management
  setContext: (context: Partial<ErrorProcessingContext>) => void;
  updateContext: (updates: Partial<ErrorProcessingContext>) => void;
  clearContext: (sections?: Array<keyof ErrorProcessingContext>) => void;
  
  // Error processing and display
  handleError: (error: any, options?: {
    showNotification?: boolean;
    showModal?: boolean;
    duration?: number;
  }) => ProcessedErrorInfo;
  
  // Error history
  addError: (errorInfo: ProcessedErrorInfo) => void;
  clearErrorHistory: () => void;
  getRecentErrors: (limit?: number) => ProcessedErrorInfo[];
  
  // Recovery management
  startRecovery: (errorId: string, actions: RecoveryAction[]) => string;
  completeRecovery: (recoveryId: string, success: boolean) => void;
  
  // Settings
  setShowTechnicalDetails: (show: boolean) => void;
  setAutoShowRecoveryActions: (show: boolean) => void;
  setNotificationDuration: (duration: number) => void;
  
  // Utilities
  getErrorStats: () => {
    total: number;
    byDomain: Record<string, number>;
    bySeverity: Record<string, number>;
    byRecoverable: { recoverable: number; nonRecoverable: number };
  };
}

type ErrorStore = ErrorState & ErrorActions;

const useErrorStore = create<ErrorStore>((set, get) => ({
  // Initial state
  currentContext: {},
  recentErrors: [],
  activeRecoveries: new Map(),
  showTechnicalDetails: false,
  autoShowRecoveryActions: true,
  notificationDuration: 4500,

  // Context management
  setContext: (context) => {
    const newContext = { ...context };
    set((state) => ({
      currentContext: { ...state.currentContext, ...newContext }
    }));
    setErrorContext(newContext);
  },

  updateContext: (updates) => {
    set((state) => {
      const newContext = { ...state.currentContext, ...updates };
      setErrorContext(updates);
      return { currentContext: newContext };
    });
  },

  clearContext: (sections) => {
    if (!sections) {
      set({ currentContext: {} });
      clearErrorContext();
    } else {
      set((state) => {
        const newContext = { ...state.currentContext };
        sections.forEach(section => {
          delete newContext[section];
        });
        clearErrorContext(sections);
        return { currentContext: newContext };
      });
    }
  },

  // Error processing and display
  handleError: (error, options = {}) => {
    const {
      showNotification = true,
      duration
    } = options;

    // Process the error with current context
    const errorInfo = processErrorWithContext(error);
    
    // Add to history
    get().addError(errorInfo);

    // Show notification if requested
    if (showNotification) {
      const notificationDuration = duration || get().notificationDuration;
      
      showEnhancedErrorNotification({
        errorInfo,
        duration: notificationDuration,
        onRecoveryComplete: (success) => {
          // Could trigger additional actions here
          console.log(`Recovery completed: ${success}`);
        }
      });
    }

    return errorInfo;
  },

  // Error history management
  addError: (errorInfo) => {
    set((state) => {
      const newErrors = [errorInfo, ...state.recentErrors].slice(0, 50); // Keep last 50 errors
      return { recentErrors: newErrors };
    });
  },

  clearErrorHistory: () => {
    set({ recentErrors: [] });
  },

  getRecentErrors: (limit = 10) => {
    return get().recentErrors.slice(0, limit);
  },

  // Recovery management
  startRecovery: (errorId, actions) => {
    const recoveryId = `recovery_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    set((state) => {
      const newRecoveries = new Map(state.activeRecoveries);
      newRecoveries.set(recoveryId, {
        errorId,
        actions,
        startTime: new Date(),
        status: 'in_progress'
      });
      return { activeRecoveries: newRecoveries };
    });

    return recoveryId;
  },

  completeRecovery: (recoveryId, success) => {
    set((state) => {
      const newRecoveries = new Map(state.activeRecoveries);
      const recovery = newRecoveries.get(recoveryId);
      if (recovery) {
        recovery.status = success ? 'completed' : 'failed';
        newRecoveries.set(recoveryId, recovery);
      }
      return { activeRecoveries: newRecoveries };
    });
  },

  // Settings
  setShowTechnicalDetails: (show) => {
    set({ showTechnicalDetails: show });
  },

  setAutoShowRecoveryActions: (show) => {
    set({ autoShowRecoveryActions: show });
  },

  setNotificationDuration: (duration) => {
    set({ notificationDuration: duration });
  },

  // Utilities
  getErrorStats: () => {
    const errors = get().recentErrors;
    
    const byDomain: Record<string, number> = {};
    const bySeverity: Record<string, number> = {};
    let recoverable = 0;
    let nonRecoverable = 0;

    errors.forEach(error => {
      // Count by domain
      byDomain[error.domain] = (byDomain[error.domain] || 0) + 1;
      
      // Count by severity
      bySeverity[error.severity] = (bySeverity[error.severity] || 0) + 1;
      
      // Count by recoverability
      if (error.isRecoverable) {
        recoverable++;
      } else {
        nonRecoverable++;
      }
    });

    return {
      total: errors.length,
      byDomain,
      bySeverity,
      byRecoverable: { recoverable, nonRecoverable }
    };
  }
}));

/**
 * Hook for easy error handling in components
 */
export const useErrorHandler = () => {
  const { handleError, updateContext } = useErrorStore();
  
  return {
    // Main error handling function
    reportError: (error: any, context?: Partial<ErrorProcessingContext>) => {
      if (context) {
        updateContext(context);
      }
      return handleError(error);
    },
    
    // Context helpers
    setUserContext: (user: { id?: number; role: 'Admin' | 'Engineer' | 'Operator'; permissions?: string[] }) => {
      updateContext({ user });
    },
    
    setOperationContext: (operation: { name: string; component: string; parameters?: Record<string, any> }) => {
      const operationWithTime = { ...operation, startTime: new Date() };
      updateContext({ operation: operationWithTime });
    },
    
    setSessionContext: (session: { sessionId?: string; startTime?: Date; expiresAt?: Date }) => {
      updateContext({ session });
    },
    
    // Simplified error reporting for common scenarios
    reportAuthError: (error: any) => {
      updateContext({ 
        operation: { name: 'authentication', component: 'AuthService' }
      });
      return handleError(error);
    },
    
    reportDataError: (error: any, operation: string) => {
      updateContext({ 
        operation: { name: operation, component: 'DataService' }
      });
      return handleError(error);
    },
    
    reportAssetError: (error: any, assetId?: string) => {
      updateContext({ 
        operation: { 
          name: 'asset_operation', 
          component: 'AssetService',
          parameters: assetId ? { assetId } : undefined
        }
      });
      return handleError(error);
    },
    
    reportUIError: (error: any, component: string) => {
      updateContext({ 
        operation: { name: 'ui_operation', component }
      });
      return handleError(error);
    }
  };
};

export default useErrorStore;