import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from './auth';
import type {
  BeginKeyRotationResponse,
  KeyRotationFinalized,
  KeyRotationProgress,
  KeyRotationStatus,
} from '../types/keyRotation';

interface KeyRotationState {
  status: KeyRotationStatus | null;
  progress: KeyRotationProgress | null;
  // Items re-encrypted since this run started (progress bar numerator)
  processedTotal: number;
  finalizeResult: KeyRotationFinalized | null;
  isRunning: boolean;
  error: string | null;
}

interface KeyRotationActions {
  fetchStatus: (token: string) => Promise<KeyRotationStatus | null>;
  begin: (
    token: string,
    password: string,
    acknowledgeUsersWithoutKeys: boolean
  ) => Promise<BeginKeyRotationResponse>;
  runUntilDone: (token: string) => Promise<KeyRotationProgress | null>;
  // Asks a running loop to stop after the current batch
  stop: () => void;
  finalize: (token: string, discardUnreadable: boolean) => Promise<KeyRotationFinalized>;
  reset: () => void;
}

const initialState: KeyRotationState = {
  status: null,
  progress: null,
  processedTotal: 0,
  finalizeResult: null,
  isRunning: false,
  error: null,
};

let abortRequested = false;

const setAuthFlag = (inProgress: boolean) => {
  useAuthStore.getState?.().setKeyRotationInProgress?.(inProgress);
};

const useKeyRotationStore = create<KeyRotationState & KeyRotationActions>()((set, get) => ({
  ...initialState,

  fetchStatus: async (token) => {
    try {
      const status = await invoke<KeyRotationStatus>('get_key_rotation_status', { token });
      if (status && typeof status.in_progress === 'boolean') {
        set({ status });
        setAuthFlag(status.in_progress);
        return status;
      }
      return null;
    } catch (error) {
      console.error('Failed to fetch key rotation status:', error);
      return null;
    }
  },

  begin: async (token, password, acknowledgeUsersWithoutKeys) => {
    set({ error: null });
    try {
      const response = await invoke<BeginKeyRotationResponse>('begin_data_key_rotation', {
        token,
        password,
        acknowledgeUsersWithoutKeys,
      });
      set({ progress: null, processedTotal: 0, finalizeResult: null });
      setAuthFlag(true);
      return response;
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  runUntilDone: async (token) => {
    if (get().isRunning) return get().progress;
    abortRequested = false;
    set({ isRunning: true, error: null });
    let last: KeyRotationProgress | null = get().progress;
    try {
      while (!abortRequested) {
        const progress = await invoke<KeyRotationProgress>('continue_data_key_rotation', { token });
        last = progress;
        set((state) => ({
          progress,
          processedTotal: state.processedTotal + progress.processed,
        }));
        if (progress.done || progress.finalized) break;
      }
    } catch (error) {
      set({ error: String(error) });
    } finally {
      set({ isRunning: false });
    }
    // Refresh counts and failures; this also clears the login flag once idle
    await get().fetchStatus(token);
    return last;
  },

  stop: () => {
    abortRequested = true;
  },

  finalize: async (token, discardUnreadable) => {
    set({ error: null });
    try {
      const result = await invoke<KeyRotationFinalized>('finalize_data_key_rotation', {
        token,
        discardUnreadable,
      });
      set({ finalizeResult: result });
      setAuthFlag(false);
      await get().fetchStatus(token);
      return result;
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  reset: () => {
    abortRequested = true;
    set({ ...initialState });
  },
}));

export default useKeyRotationStore;
