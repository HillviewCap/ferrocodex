import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { 
  BulkImportSession, 
  BulkImportSessionDetails, 
  CreateBulkImportSessionRequest,
  ValidationResults,
  ProcessingOptions,
  ProgressStatus,
  ImportTemplate,
  ImportTemplateConfig,
  BulkOperationStats,
  ValidationSummary,
  BulkImportStatus
} from '../types/bulk';

interface BulkImportState {
  // Sessions
  sessions: BulkImportSession[];
  currentSession: BulkImportSessionDetails | null;
  currentProgress: ProgressStatus | null;
  
  // Templates
  templates: ImportTemplate[];
  
  // Stats
  stats: BulkOperationStats | null;
  
  // UI State
  isLoading: boolean;
  error: string | null;
  
  // Actions
  createSession: (request: CreateBulkImportSessionRequest) => Promise<void>;
  loadSessions: () => Promise<void>;
  loadSessionDetails: (sessionId: number) => Promise<void>;
  deleteSession: (sessionId: number) => Promise<void>;
  
  uploadFile: (sessionId: number, filePath: string) => Promise<ValidationSummary>;
  validateData: (sessionId: number) => Promise<ValidationResults>;
  startProcessing: (sessionId: number, options: ProcessingOptions) => Promise<void>;
  pauseProcessing: (sessionId: number) => Promise<void>;
  resumeProcessing: (sessionId: number) => Promise<void>;
  cancelProcessing: (sessionId: number) => Promise<void>;
  
  getProgress: (sessionId: number) => Promise<ProgressStatus>;
  
  // Template actions
  createTemplate: (config: ImportTemplateConfig) => Promise<void>;
  loadTemplates: (templateType: string) => Promise<void>;
  deleteTemplate: (templateId: number) => Promise<void>;
  generateTemplateCSV: (assetType: string, metadataSchema?: string) => Promise<string>;
  
  // Stats actions
  loadStats: () => Promise<void>;
  
  // Utility actions
  setError: (error: string | null) => void;
  clearError: () => void;
  reset: () => void;
}

const useBulkImportStore = create<BulkImportState>()(
  persist(
    (set, get) => ({
      // Initial state
      sessions: [],
      currentSession: null,
      currentProgress: null,
      templates: [],
      stats: null,
      isLoading: false,
      error: null,

      // Session actions
      createSession: async (request: CreateBulkImportSessionRequest) => {
        set({ isLoading: true, error: null });
        try {
          const session = await invoke<BulkImportSession>('create_bulk_import_session', {
            sessionName: request.session_name,
            importType: request.import_type,
            templatePath: request.template_path,
          });
          
          const sessions = [...get().sessions, session];
          set({ sessions, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to create session';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      loadSessions: async () => {
        set({ isLoading: true, error: null });
        try {
          const sessions = await invoke<BulkImportSession[]>('get_bulk_import_sessions');
          set({ sessions, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load sessions';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      loadSessionDetails: async (sessionId: number) => {
        set({ isLoading: true, error: null });
        try {
          const sessionDetails = await invoke<BulkImportSessionDetails | null>('get_bulk_import_session_details', {
            sessionId,
          });
          set({ currentSession: sessionDetails, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load session details';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      deleteSession: async (sessionId: number) => {
        set({ isLoading: true, error: null });
        try {
          await invoke('delete_bulk_import_session', { sessionId });
          const sessions = get().sessions.filter(s => s.id !== sessionId);
          set({ sessions, isLoading: false });
          
          // Clear current session if it was deleted
          if (get().currentSession?.session.id === sessionId) {
            set({ currentSession: null });
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to delete session';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      // File upload and processing actions
      uploadFile: async (sessionId: number, filePath: string) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationSummary>('upload_bulk_import_file', {
            sessionId,
            filePath,
          });
          
          // Refresh session details after upload
          await get().loadSessionDetails(sessionId);
          
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to upload file';
          set({ error: errorMessage });
          throw error;
        }
      },

      validateData: async (sessionId: number) => {
        set({ error: null });
        try {
          const result = await invoke<ValidationResults>('validate_bulk_import_data', {
            sessionId,
          });
          
          // Refresh session details after validation
          await get().loadSessionDetails(sessionId);
          
          return result;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to validate data';
          set({ error: errorMessage });
          throw error;
        }
      },

      startProcessing: async (sessionId: number, options: ProcessingOptions) => {
        set({ error: null });
        try {
          await invoke('start_bulk_import_processing', {
            sessionId,
            options,
          });
          
          // Start polling for progress
          get().getProgress(sessionId);
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to start processing';
          set({ error: errorMessage });
          throw error;
        }
      },

      pauseProcessing: async (sessionId: number) => {
        set({ error: null });
        try {
          await invoke('pause_bulk_import', { sessionId });
          
          // Refresh session details
          await get().loadSessionDetails(sessionId);
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to pause processing';
          set({ error: errorMessage });
          throw error;
        }
      },

      resumeProcessing: async (sessionId: number) => {
        set({ error: null });
        try {
          await invoke('resume_bulk_import', { sessionId });
          
          // Start polling for progress again
          get().getProgress(sessionId);
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to resume processing';
          set({ error: errorMessage });
          throw error;
        }
      },

      cancelProcessing: async (sessionId: number) => {
        set({ error: null });
        try {
          await invoke('cancel_bulk_import', { sessionId });
          
          // Refresh session details
          await get().loadSessionDetails(sessionId);
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to cancel processing';
          set({ error: errorMessage });
          throw error;
        }
      },

      getProgress: async (sessionId: number) => {
        try {
          const progress = await invoke<ProgressStatus>('get_bulk_import_progress', {
            sessionId,
          });
          
          set({ currentProgress: progress });
          
          // Continue polling if processing
          if (progress.status === 'Processing') {
            setTimeout(() => get().getProgress(sessionId), 2000);
          }
          
          return progress;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to get progress';
          set({ error: errorMessage });
          throw error;
        }
      },

      // Template actions
      createTemplate: async (config: ImportTemplateConfig) => {
        set({ isLoading: true, error: null });
        try {
          const template = await invoke<ImportTemplate>('create_import_template', {
            templateConfig: config,
          });
          
          const templates = [...get().templates, template];
          set({ templates, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to create template';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      loadTemplates: async (templateType: string) => {
        set({ isLoading: true, error: null });
        try {
          const templates = await invoke<ImportTemplate[]>('get_import_templates', {
            templateType,
          });
          set({ templates, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load templates';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      deleteTemplate: async (templateId: number) => {
        set({ isLoading: true, error: null });
        try {
          await invoke('delete_import_template', { templateId });
          const templates = get().templates.filter(t => t.id !== templateId);
          set({ templates, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to delete template';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      generateTemplateCSV: async (assetType: string, metadataSchema?: string) => {
        set({ error: null });
        try {
          const csvContent = await invoke<string>('generate_import_template_csv', {
            assetType,
            metadataSchema,
          });
          return csvContent;
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to generate template';
          set({ error: errorMessage });
          throw error;
        }
      },

      // Stats actions
      loadStats: async () => {
        set({ isLoading: true, error: null });
        try {
          const stats = await invoke<BulkOperationStats>('get_bulk_operation_stats');
          set({ stats, isLoading: false });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Failed to load stats';
          set({ error: errorMessage, isLoading: false });
          throw error;
        }
      },

      // Utility actions
      setError: (error: string | null) => {
        set({ error });
      },

      clearError: () => {
        set({ error: null });
      },

      reset: () => {
        set({
          sessions: [],
          currentSession: null,
          currentProgress: null,
          templates: [],
          stats: null,
          isLoading: false,
          error: null,
        });
      },
    }),
    {
      name: 'bulk-import-storage',
      partialize: (state) => ({
        // Only persist non-sensitive data
        sessions: state.sessions,
        templates: state.templates,
      }),
    }
  )
);

export default useBulkImportStore;