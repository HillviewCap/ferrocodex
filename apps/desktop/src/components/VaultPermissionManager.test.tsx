import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import VaultPermissionManager from './VaultPermissionManager';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const users = [
  { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
  { id: 2, username: 'bob', role: 'Engineer', created_at: '2024-01-01', is_active: true },
  { id: 3, username: 'retired', role: 'Engineer', created_at: '2024-01-01', is_active: false },
];

const permissions = [
  { permission_id: 10, user_id: 2, vault_id: 5, permission_type: 'Read', granted_by: 1, granted_at: '2024-01-01T00:00:00Z', is_active: true },
  { permission_id: 11, user_id: 2, vault_id: 5, permission_type: 'Write', granted_by: 1, granted_at: '2024-01-02T00:00:00Z', expires_at: '2999-01-01T00:00:00Z', is_active: true },
  { permission_id: 12, user_id: 1, vault_id: 5, permission_type: 'Export', granted_by: 1, granted_at: '2024-01-02T00:00:00Z', is_active: false },
];

describe('VaultPermissionManager', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_vault_permissions') return permissions;
      if (cmd === 'list_users') return users;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('lists active permissions per user with revoke actions', async () => {
    render(<VaultPermissionManager vaultId={5} vaultName="PLC Vault" />);

    expect(screen.getByText('Vault Permissions - PLC Vault')).toBeInTheDocument();
    expect(await screen.findByText('bob')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_vault_permissions', { token: 'test-token', vaultId: 5 });
    expect(invoke).toHaveBeenCalledWith('list_users', { token: 'test-token' });

    const table = within(document.querySelector('.ant-table') as HTMLElement);
    expect(table.getByText('Read')).toBeInTheDocument();
    expect(table.getByText('Write')).toBeInTheDocument();
    expect(table.getByRole('button', { name: /Revoke Read/ })).toBeInTheDocument();
    expect(table.getByRole('button', { name: /Revoke Write/ })).toBeInTheDocument();
    // Inactive permissions are not listed
    expect(table.queryByText('admin')).not.toBeInTheDocument();
  });

  it('opens the grant modal with user, permission and expiry controls', async () => {
    const onClose = vi.fn();
    render(<VaultPermissionManager vaultId={5} vaultName="PLC Vault" onClose={onClose} />);
    await screen.findByText('bob');

    fireEvent.click(screen.getByRole('button', { name: /Grant Access/ }));
    expect(await screen.findByText('Grant Vault Access')).toBeInTheDocument();
    expect(screen.getByText('Select User:')).toBeInTheDocument();
    expect(screen.getByText('Permission Type:')).toBeInTheDocument();
    expect(screen.getByText('Set Expiration Date')).toBeInTheDocument();
    expect(screen.queryByText('Expires At:')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('switch'));
    expect(screen.getByText('Expires At:')).toBeInTheDocument();

    // The modal's icon-only close control is also named "Close"; pick the text button
    const closeButton = screen.getAllByRole('button', { name: 'Close' }).find(b => b.textContent === 'Close');
    fireEvent.click(closeButton as HTMLElement);
    expect(onClose).toHaveBeenCalled();
  });
});
