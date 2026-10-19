import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import useUserManagementStore from './userManagement';

describe('userManagement store', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useUserManagementStore.setState({
      users: [{ id: 3, username: 'engineer2', role: 'Engineer', created_at: '2023-01-03', is_active: false }],
      isLoading: false,
      error: null,
    });
  });

  it('reactivateUser sends the temporary password and marks the user active', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await useUserManagementStore.getState().reactivateUser('tok', 3, 'Temp-Pass-456');

    expect(invoke).toHaveBeenCalledWith('reactivate_user', { token: 'tok', userId: 3, newPassword: 'Temp-Pass-456' });
    expect(useUserManagementStore.getState().users[0].is_active).toBe(true);
  });

  it('resetUserPassword invokes admin_reset_user_password', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await useUserManagementStore.getState().resetUserPassword('tok', 3, 'Temp-Pass-456');

    expect(invoke).toHaveBeenCalledWith('admin_reset_user_password', { token: 'tok', userId: 3, newPassword: 'Temp-Pass-456' });
  });

  it('resetUserPassword records the error and rethrows', async () => {
    vi.mocked(invoke).mockRejectedValue('Administrators cannot reset their own password this way');

    await expect(useUserManagementStore.getState().resetUserPassword('tok', 1, 'Temp-Pass-456')).rejects.toBeTruthy();
    expect(useUserManagementStore.getState().error).toBe('Administrators cannot reset their own password this way');
  });
});
