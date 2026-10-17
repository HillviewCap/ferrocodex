import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import VaultListWithAccess from './VaultListWithAccess';
import { VaultInfo } from '../types/vault';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('./VaultAccessIndicator', () => ({ default: () => <span data-testid="vault-access-indicator" /> }));

const engineer = { id: 2, username: 'eng', role: 'Engineer' as const, created_at: '2024-01-01', is_active: true };

const vault: VaultInfo = {
  vault: { id: 5, asset_id: 2, name: 'PLC Vault', description: 'Line 1 credentials', created_by: 1, created_at: '2024-01-01', updated_at: '2024-01-01' },
  secrets: [
    { id: 1, vault_id: 5, secret_type: 'Password', label: 'admin', encrypted_value: 'x', created_at: '2024-01-01', updated_at: '2024-01-01' },
    { id: 2, vault_id: 5, secret_type: 'IpAddress', label: 'mgmt', encrypted_value: 'y', created_at: '2024-01-01', updated_at: '2024-01-01' },
  ],
  secret_count: 2,
};

const pendingRequest = {
  request_id: 1,
  user_id: 2,
  vault_id: 5,
  requested_permission: 'Read' as const,
  requested_by: 2,
  status: 'Pending' as const,
  created_at: '2024-01-01',
  updated_at: '2024-01-01',
};

const mockCommands = (opts: { vault?: VaultInfo | null; hasAccess?: boolean; requests?: unknown[] }) => {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case 'get_vault_by_asset_id':
        return opts.vault === undefined ? vault : opts.vault;
      case 'check_vault_access':
        return { has_access: opts.hasAccess ?? true, permissions: [], is_administrator: false };
      case 'get_user_vault_permissions':
        return opts.requests ?? [];
      case 'create_permission_request':
        return undefined;
      default:
        throw new Error(`Unexpected command: ${cmd}`);
    }
  });
};

describe('VaultListWithAccess', () => {
  useConsoleGuard({
    // List migration was explicitly deferred by d052492; Listy is not a drop-in.
    ignore: [/\[antd: List\]/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', user: engineer });
  });

  it('renders an accessible vault with its secret summary', async () => {
    mockCommands({ hasAccess: true });
    const onVaultSelect = vi.fn();
    render(<VaultListWithAccess assetId={2} onVaultSelect={onVaultSelect} />);

    expect(await screen.findByText('PLC Vault')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_vault_by_asset_id', { token: 'test-token', assetId: 2 });
    expect(screen.getByText('Line 1 credentials')).toBeInTheDocument();
    expect(screen.getByText('1 Password')).toBeInTheDocument();
    expect(screen.getByText('1 IpAddress')).toBeInTheDocument();
    expect(screen.getByText('Total: 2 secrets')).toBeInTheDocument();
    expect(screen.getByTestId('vault-access-indicator')).toBeInTheDocument();

    fireEvent.click(screen.getByText('PLC Vault'));
    expect(onVaultSelect).toHaveBeenCalledWith(vault);
  });

  it('lets an engineer without access request it', async () => {
    mockCommands({ hasAccess: false });
    render(<VaultListWithAccess assetId={2} />);

    const request = await screen.findByRole('button', { name: /Request Access/ });
    expect(screen.queryByText('Total: 2 secrets')).not.toBeInTheDocument();

    fireEvent.click(request);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_permission_request', {
        token: 'test-token',
        request: { vault_id: 5, requested_permission: 'Read', requested_by: 2 },
      });
    });
    expect(await screen.findByText('Access Requested')).toBeInTheDocument();
  });

  it('shows a pending tag when a request is already open', async () => {
    mockCommands({ hasAccess: false, requests: [pendingRequest] });
    render(<VaultListWithAccess assetId={2} />);

    expect(await screen.findByText('Access Request Pending')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Request Access/ })).not.toBeInTheDocument();
  });

  it('shows the empty state when the asset has no vault', async () => {
    mockCommands({ vault: null });
    render(<VaultListWithAccess assetId={2} />);

    expect(await screen.findByText('No Identity Vault')).toBeInTheDocument();
  });
});
