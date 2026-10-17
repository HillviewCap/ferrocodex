import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import VaultExportButton from './VaultExportButton';
import { VaultInfo } from '../types/vault';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }));

const vault: VaultInfo = {
  vault: { id: 5, asset_id: 2, name: 'PLC Vault', description: 'Credentials', created_by: 1, created_at: '2024-01-01', updated_at: '2024-01-01' },
  secrets: [
    { id: 1, vault_id: 5, secret_type: 'Password', label: 'admin', encrypted_value: 'x', created_at: '2024-01-01', updated_at: '2024-01-01' },
    { id: 2, vault_id: 5, secret_type: 'IpAddress', label: 'mgmt', encrypted_value: 'y', created_at: '2024-01-01', updated_at: '2024-01-01' },
  ],
  secret_count: 2,
};

const mockAccess = (has_access: boolean, is_administrator: boolean) => {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case 'check_vault_access':
        return { has_access, permissions: [], is_administrator };
      case 'export_vault':
        return '{"vault":"data"}';
      case 'write_file':
        return undefined;
      default:
        throw new Error(`Unexpected command: ${cmd}`);
    }
  });
};

describe('VaultExportButton', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
    });
    vi.mocked(save).mockResolvedValue('/tmp/PLC_Vault_vault_export.json');
  });

  it('opens the export modal with the security and administrator alerts and exports', async () => {
    mockAccess(true, true);
    const onExportComplete = vi.fn();
    render(<VaultExportButton vault={vault} onExportComplete={onExportComplete} />);

    const trigger = screen.getByRole('button', { name: /Export Vault/ });
    await waitFor(() => expect(trigger).toBeEnabled());
    expect(invoke).toHaveBeenCalledWith('check_vault_access', { token: 'test-token', vaultId: 5, permissionType: 'Export' });

    fireEvent.click(trigger);
    expect(await screen.findByText('Security Warning')).toBeInTheDocument();
    expect(screen.getByText('Administrator Export')).toBeInTheDocument();
    expect(screen.getByText('Vault to Export:')).toBeInTheDocument();
    expect(screen.getByText('PLC Vault')).toBeInTheDocument();
    expect(screen.getByText('2 secrets • 2 items')).toBeInTheDocument();

    const buttons = screen.getAllByRole('button', { name: /Export Vault/ });
    fireEvent.click(buttons[buttons.length - 1]);

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('export_vault', { token: 'test-token', vaultId: 5 }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('write_file', { path: '/tmp/PLC_Vault_vault_export.json', contents: '{"vault":"data"}' }),
    );
    expect(await screen.findByText('Vault Exported')).toBeInTheDocument();
    expect(onExportComplete).toHaveBeenCalled();
  });

  it('disables the button when the user cannot export', async () => {
    mockAccess(false, false);
    render(<VaultExportButton vault={vault} buttonText="Export" />);

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('check_vault_access', expect.anything()));
    expect(screen.getByRole('button', { name: /Export/ })).toBeDisabled();
    expect(screen.queryByText('Security Warning')).not.toBeInTheDocument();
  });
});
