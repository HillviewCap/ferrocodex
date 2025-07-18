import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { UserInfo } from './auth';

interface UserManagementState {
  users: UserInfo[];
  isLoading: boolean;
  error: string | null;
  isCreatingUser: boolean;
  createUserError: string | null;
}

interface UserManagementActions {
  fetchUsers: (token: string) => Promise<void>;
  createEngineerUser: (token: string, username: string, password: string) => Promise<void>;
  deactivateUser: (token: string, userId: number) => Promise<void>;
  reactivateUser: (token: string, userId: number) => Promise<void>;
  clearError: () => void;
  clearCreateUserError: () => void;
  setLoading: (loading: boolean) => void;
}

const useUserManagementStore = create<UserManagementState & UserManagementActions>()(
  (set, get) => ({
    // State
    users: [],
    isLoading: false,
    error: null,
    isCreatingUser: false,
    createUserError: null,

    // Actions
    fetchUsers: async (token: string) => {
      try {
        set({ isLoading: true, error: null });
        
        const users: UserInfo[] = await invoke('list_users', { token });
        
        set({
          users,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        set({
          users: [],
          isLoading: false,
          error: error as string,
        });
        throw error;
      }
    },

    createEngineerUser: async (token: string, username: string, password: string) => {
      try {
        set({ isCreatingUser: true, createUserError: null });
        
        const newUser: UserInfo = await invoke('create_engineer_user', {
          token,
          username,
          initialPassword: password,
        });

        // Add the new user to the list
        const currentUsers = get().users;
        set({
          users: [newUser, ...currentUsers],
          isCreatingUser: false,
          createUserError: null,
        });
      } catch (error) {
        set({
          isCreatingUser: false,
          createUserError: error as string,
        });
        throw error;
      }
    },

    deactivateUser: async (token: string, userId: number) => {
      try {
        set({ isLoading: true, error: null });
        
        await invoke('deactivate_user', { token, userId });
        
        // Update the user in the list
        const currentUsers = get().users;
        const updatedUsers = currentUsers.map(user => 
          user.id === userId ? { ...user, is_active: false } : user
        );
        
        set({
          users: updatedUsers,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        set({
          isLoading: false,
          error: error as string,
        });
        throw error;
      }
    },

    reactivateUser: async (token: string, userId: number) => {
      try {
        set({ isLoading: true, error: null });
        
        await invoke('reactivate_user', { token, userId });
        
        // Update the user in the list
        const currentUsers = get().users;
        const updatedUsers = currentUsers.map(user => 
          user.id === userId ? { ...user, is_active: true } : user
        );
        
        set({
          users: updatedUsers,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        set({
          isLoading: false,
          error: error as string,
        });
        throw error;
      }
    },

    clearError: () => set({ error: null }),
    clearCreateUserError: () => set({ createUserError: null }),
    setLoading: (loading: boolean) => set({ isLoading: loading }),
  })
);

export default useUserManagementStore;