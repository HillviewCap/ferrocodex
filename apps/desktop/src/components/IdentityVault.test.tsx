import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { App } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import IdentityVault from './IdentityVault';
import { AssetInfo } from '../types/assets';
import { VaultInfo } from '../types/vault';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

// Collaborators with their own data loading are stubbed so the test covers the vault
// screen's own Tabs / Alert / Modal conversions.
vi.mock('./VaultAccessIndicator', () => ({ default: () => <span data-testid="vault-access-indicator" /> }));
vi.mock('./VaultExportButton', () => ({ default: () => <button type="button">Export</button> }));
vi.mock('./VaultPermissionManager', () => ({ default: () => <div data-testid="vault-permission-manager" /> }));
vi.mock('./PasswordGenerator', () => ({ default: () => null }));
vi.mock('./PasswordInput', () => ({ default: () => <input data-testid="password-input" /> }));
vi.mock('./PasswordHistory', () => ({ default: () => null }));

const asset: AssetInfo = {
  id: 7,
  name: 'PLC-001',
  description: 'Primary PLC',
  asset_type: 'Device',
  parent_id: null,
  sort_order: 0,
  created_by: 1,
  created_by_username: 'admin',
  created_at: '2024-01-01T00:00:00Z',
  version_count: 1,
  latest_version: 'v1',
  latest_version_notes: null,
};

const vaultInfo: VaultInfo = {
  vault: {
    id: 3,
    asset_id: 7,
    name: 'PLC-001 Identity Vault',
    description: 'Credentials for the line PLC',
    created_by: 1,
    created_at: '2024-01-02T00:00:00Z',
    updated_at: '2024-01-02T00:00:00Z',
  },
  secrets: [
    {
      id: 11,
      vault_id: 3,
      secret_type: 'Password',
      label: 'Admin login',
      encrypted_value: 'enc',
      created_at: '2024-01-03T00:00:00Z',
      updated_at: '2024-01-03T00:00:00Z',
      strength_score: 85,
    },
    {
      id: 12,
      vault_id: 3,
      secret_type: 'IpAddress',
      label: 'Management IP',
      encrypted_value: 'enc',
      created_at: '2024-01-03T00:00:00Z',
      updated_at: '2024-01-03T00:00:00Z',
    },
  ],
  secret_count: 2,
};

const history = [
  { id: 1, vault_id: 3, change_type: 'VaultCreated', author: 1, timestamp: '2024-01-02T00:00:00Z', notes: 'Vault created', changes_json: '' },
];

const renderVault = () =>
  render(
    <App>
      <IdentityVault asset={asset} />
    </App>,
  );

describe('IdentityVault', () => {
  useConsoleGuard({
    // antd 6 deprecates `List` in favour of `Listy` (a different API). Commit d052492
    // deliberately left the List migration for a follow-up, so that one deprecation is
    // tolerated here; anything else antd/React warns about still fails the test.
    ignore: [/\[antd: List\] The `List` component is deprecated/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_vault_by_asset_id':
          return vaultInfo;
        case 'check_vault_access':
          return { has_access: true };
        case 'get_vault_history':
          return history;
        default:
          throw new Error(`Unexpected command: ${cmd}`);
      }
    });
  });

  it('renders the vault header, security alert and secrets tab', async () => {
    renderVault();

    expect(await screen.findByText('PLC-001 Identity Vault')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_vault_by_asset_id', { token: 'test-token', assetId: 7 });
    expect(screen.getByText('Credentials for the line PLC')).toBeInTheDocument();
    expect(screen.getByText('2 secrets')).toBeInTheDocument();
    expect(screen.getByText('Security Notice')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Manage Permissions/ })).toBeInTheDocument();

    expect(screen.getByRole('tab', { name: /Secrets \(2\)/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /History/ })).toBeInTheDocument();
    expect(screen.getByText('Admin login')).toBeInTheDocument();
    expect(screen.getByText('Management IP')).toBeInTheDocument();
    expect(screen.getByText('Password')).toBeInTheDocument();
    expect(screen.getByText('IP Address')).toBeInTheDocument();
  });

  it('switches to the history tab and loads history', async () => {
    renderVault();
    await screen.findByText('PLC-001 Identity Vault');

    fireEvent.click(screen.getByRole('tab', { name: /History/ }));
    expect(await screen.findByText('Complete audit trail of all vault and secret changes.')).toBeInTheDocument();
    expect(screen.getByText('No history available')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Refresh History/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_vault_history', { token: 'test-token', vaultId: 3 }));
    expect(await screen.findByText('Vault created')).toBeInTheDocument();
  });

  it('opens the add-secret and permission modals', async () => {
    renderVault();
    await screen.findByText('PLC-001 Identity Vault');

    fireEvent.click(screen.getByRole('button', { name: /Add Secret/ }));
    expect(await screen.findByText('Secret Type')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Manage Permissions/ }));
    expect(await screen.findByText('Vault Permissions')).toBeInTheDocument();
    expect(screen.getByTestId('vault-permission-manager')).toBeInTheDocument();
  });

  it('offers to create a vault when none exists', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_vault_by_asset_id') return null;
      throw new Error(`Unexpected command: ${cmd}`);
    });
    renderVault();

    expect(await screen.findByText('No Identity Vault')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Create Identity Vault/ }));
    expect(await screen.findByLabelText('Vault Name')).toBeInTheDocument();
  });
});
