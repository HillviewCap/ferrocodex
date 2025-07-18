import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

export interface UserInfo {
  id: number;
  username: string;
  role: 'Administrator' | 'Engineer';
  created_at: string;
  is_active: boolean;
}

export interface LoginResponse {
  token: string;
  user: UserInfo;
}

interface AuthState {
  user: UserInfo | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

interface AuthActions {
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  createAdminAccount: (username: string, password: string) => Promise<void>;
  checkSession: () => Promise<void>;
  clearError: () => void;
  setLoading: (loading: boolean) => void;
}

const useAuthStore = create<AuthState & AuthActions>()(
  persist(
    (set, get) => ({
      // State
      user: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,

      // Actions
      login: async (username: string, password: string) => {
        try {
          set({ isLoading: true, error: null });
          
          const response: LoginResponse = await invoke('login', {
            username,
            password,
          });

          set({
            user: response.user,
            token: response.token,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
        } catch (error) {
          set({
            user: null,
            token: null,
            isAuthenticated: false,
            isLoading: false,
            error: error as string,
          });
          throw error;
        }
      },

      logout: async () => {
        const { token } = get();
        try {
          if (token) {
            await invoke('logout', { token });
          }
        } catch (error) {
          console.error('Logout error:', error);
        } finally {
          set({
            user: null,
            token: null,
            isAuthenticated: false,
            isLoading: false,
            error: null,
          });
        }
      },

      createAdminAccount: async (username: string, password: string) => {
        try {
          set({ isLoading: true, error: null });
          
          const response: LoginResponse = await invoke('create_admin_account', {
            username,
            password,
          });

          set({
            user: response.user,
            token: response.token,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
        } catch (error) {
          set({
            user: null,
            token: null,
            isAuthenticated: false,
            isLoading: false,
            error: error as string,
          });
          throw error;
        }
      },

      checkSession: async () => {
        const { token } = get();
        if (!token) {
          set({ isAuthenticated: false, user: null });
          return;
        }

        try {
          const user: UserInfo = await invoke('check_session', { token });
          set({
            user,
            isAuthenticated: true,
            error: null,
          });
        } catch (error) {
          set({
            user: null,
            token: null,
            isAuthenticated: false,
            error: error as string,
          });
        }
      },

      clearError: () => set({ error: null }),

      setLoading: (loading: boolean) => set({ isLoading: loading }),
    }),
    {
      name: 'auth-storage',
      partialize: (state) => ({
        token: state.token,
        user: state.user,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);

export default useAuthStore;