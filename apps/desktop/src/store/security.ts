import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import {
  SecurityValidationResult,
  SecurityMetrics,
  SecurityAlert,
  AuditEvent,
  SecurityHealthReport,
  ValidationStats,
  SecurityClassificationLevel
} from '../types/security';

interface SecurityState {
  // Validation state
  validationCache: Record<string, SecurityValidationResult>;
  lastValidation?: SecurityValidationResult;
  
  // Security metrics
  metrics: SecurityMetrics | null;
  healthReport: SecurityHealthReport | null;
  validationStats: ValidationStats | null;
  
  // Alerts and notifications
  alerts: SecurityAlert[];
  unreadAlertCount: number;
  
  // Audit log
  auditEvents: AuditEvent[];
  auditFilters: {
    startDate?: string;
    endDate?: string;
    eventType?: string;
    severity?: 'info' | 'warning' | 'critical';
  };
  
  // UI state
  showSecurityPanel: boolean;
  selectedClassification?: SecurityClassificationLevel;
  
  // Actions
  setValidationResult: (name: string, result: SecurityValidationResult) => void;
  clearValidationCache: () => void;
  setMetrics: (metrics: SecurityMetrics) => void;
  setHealthReport: (report: SecurityHealthReport) => void;
  setValidationStats: (stats: ValidationStats) => void;
  addAlert: (alert: Omit<SecurityAlert, 'id'>) => void;
  acknowledgeAlert: (alertId: string) => void;
  clearAlerts: () => void;
  addAuditEvent: (event: AuditEvent) => void;
  setAuditFilters: (filters: Partial<SecurityState['auditFilters']>) => void;
  clearAuditEvents: () => void;
  toggleSecurityPanel: () => void;
  setSelectedClassification: (level?: SecurityClassificationLevel) => void;
}

const useSecurityStore = create<SecurityState>()(
  persist(
    (set, get) => ({
      // Initial state
      validationCache: {},
      metrics: null,
      healthReport: null,
      validationStats: null,
      alerts: [],
      unreadAlertCount: 0,
      auditEvents: [],
      auditFilters: {},
      showSecurityPanel: false,
      
      // Actions
      setValidationResult: (name: string, result: SecurityValidationResult) => {
        set(state => ({
          validationCache: {
            ...state.validationCache,
            [name]: result
          },
          lastValidation: result
        }));
      },

      clearValidationCache: () => {
        set({ validationCache: {}, lastValidation: undefined });
      },

      setMetrics: (metrics: SecurityMetrics) => {
        set({ metrics });
      },

      setHealthReport: (healthReport: SecurityHealthReport) => {
        set({ healthReport });
      },

      setValidationStats: (validationStats: ValidationStats) => {
        set({ validationStats });
      },

      addAlert: (alertData) => {
        const alert: SecurityAlert = {
          ...alertData,
          id: `alert-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        };

        set(state => ({
          alerts: [alert, ...state.alerts],
          unreadAlertCount: state.unreadAlertCount + 1
        }));
      },

      acknowledgeAlert: (alertId: string) => {
        set(state => ({
          alerts: state.alerts.map(alert =>
            alert.id === alertId
              ? { ...alert, acknowledged: true }
              : alert
          ),
          unreadAlertCount: Math.max(0, state.unreadAlertCount - 1)
        }));
      },

      clearAlerts: () => {
        set({ alerts: [], unreadAlertCount: 0 });
      },

      addAuditEvent: (event: AuditEvent) => {
        set(state => ({
          auditEvents: [event, ...state.auditEvents].slice(0, 1000) // Keep last 1000 events
        }));
      },

      setAuditFilters: (filters) => {
        set(state => ({
          auditFilters: { ...state.auditFilters, ...filters }
        }));
      },

      clearAuditEvents: () => {
        set({ auditEvents: [] });
      },

      toggleSecurityPanel: () => {
        set(state => ({ showSecurityPanel: !state.showSecurityPanel }));
      },

      setSelectedClassification: (selectedClassification) => {
        set({ selectedClassification });
      }
    }),
    {
      name: 'security-storage',
      // Only persist certain parts of the state
      partialize: (state) => ({
        validationCache: state.validationCache,
        auditFilters: state.auditFilters,
        selectedClassification: state.selectedClassification,
        showSecurityPanel: state.showSecurityPanel
      })
    }
  )
);

// Selectors for computed values
export const useSecurityMetrics = () => {
  const metrics = useSecurityStore(state => state.metrics);
  return metrics;
};

export const useSecurityAlerts = () => {
  const alerts = useSecurityStore(state => state.alerts);
  const unreadCount = useSecurityStore(state => state.unreadAlertCount);
  
  return {
    alerts,
    unreadCount,
    criticalAlerts: alerts.filter(a => a.severity === 'critical' && !a.acknowledged),
    warningAlerts: alerts.filter(a => a.severity === 'warning' && !a.acknowledged),
    infoAlerts: alerts.filter(a => a.severity === 'info' && !a.acknowledged)
  };
};

export const useAuditEvents = () => {
  const auditEvents = useSecurityStore(state => state.auditEvents);
  const filters = useSecurityStore(state => state.auditFilters);
  
  // Apply filters
  const filteredEvents = auditEvents.filter(event => {
    if (filters.startDate && event.timestamp < filters.startDate) return false;
    if (filters.endDate && event.timestamp > filters.endDate) return false;
    if (filters.eventType && event.eventType !== filters.eventType) return false;
    return true;
  });
  
  return {
    events: filteredEvents,
    totalCount: auditEvents.length,
    filteredCount: filteredEvents.length
  };
};

export const useValidationCache = () => {
  const cache = useSecurityStore(state => state.validationCache);
  const lastValidation = useSecurityStore(state => state.lastValidation);
  
  return {
    cache,
    lastValidation,
    getCachedResult: (name: string) => cache[name],
    hasCachedResult: (name: string) => name in cache
  };
};

export const useSecurityHealth = () => {
  const healthReport = useSecurityStore(state => state.healthReport);
  const metrics = useSecurityStore(state => state.metrics);
  
  const getOverallHealthStatus = () => {
    if (!healthReport || !metrics) return 'unknown';
    
    const criticalAlerts = useSecurityStore.getState().alerts
      .filter(a => a.severity === 'critical' && !a.acknowledged).length;
    
    if (criticalAlerts > 0) return 'critical';
    if (healthReport.systemSecurityLevel === 'Low') return 'warning';
    if (healthReport.validationSuccessRate < 85) return 'warning';
    
    return healthReport.systemSecurityLevel === 'High' ? 'excellent' : 'good';
  };
  
  return {
    healthReport,
    metrics,
    overallStatus: getOverallHealthStatus()
  };
};

export default useSecurityStore;