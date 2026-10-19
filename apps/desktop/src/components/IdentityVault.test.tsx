import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
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
      username: 'DOMAIN\\Operator',
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
        case 'add_vault_secret':
        case 'update_vault_secret':
          return null;
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

  it('shows a secret username without decrypting anything', async () => {
    renderVault();

    expect(await screen.findByText('DOMAIN\\Operator')).toBeInTheDocument();
    // Only the secret with a username shows one, and it can be copied
    expect(screen.getAllByText('Username:')).toHaveLength(1);
    const row = screen.getByText('Admin login').closest('.ant-list-item') as HTMLElement;
    expect(within(row).getByRole('button', { name: /copy/i })).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('decrypt_vault_secret', expect.anything());
  });

  it('sends the username when adding a secret', async () => {
    renderVault();
    await screen.findByText('PLC-001 Identity Vault');

    fireEvent.click(screen.getByRole('button', { name: /Add Secret/ }));
    const modal = (await screen.findByText('Secret Type')).closest('.ant-modal') as HTMLElement;
    // No username field until the type is one that has an account
    expect(within(modal).queryByLabelText('Username (optional)')).not.toBeInTheDocument();

    fireEvent.mouseDown(within(modal).getByLabelText('Secret Type'));
    const vpnOption = await waitFor(() => {
      const option = Array.from(document.querySelectorAll('.ant-select-item-option'))
        .find(el => el.textContent?.trim() === 'VPN Key');
      expect(option).toBeTruthy();
      return option as HTMLElement;
    });
    fireEvent.click(vpnOption);

    fireEvent.change(within(modal).getByLabelText('Label'), { target: { value: 'Site VPN' } });
    fireEvent.change(await within(modal).findByLabelText('Username (optional)'), { target: { value: ' plc@site ' } });
    fireEvent.change(within(modal).getByLabelText('Secret Value'), { target: { value: 'vpn-key-data' } });
    fireEvent.click(within(modal).getByRole('button', { name: 'Add Secret' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('add_vault_secret', {
      token: 'test-token',
      secretRequest: {
        vault_id: 3,
        secret_type: 'VpnKey',
        label: 'Site VPN',
        username: 'plc@site',
        value: 'vpn-key-data',
        author_id: 1,
      },
    }));
  });

  const openEditModal = async (label: string) => {
    renderVault();
    const row = (await screen.findByText(label)).closest('.ant-list-item') as HTMLElement;
    fireEvent.click(row.querySelector('.anticon-edit')!.closest('button')!);
    return (await screen.findByRole('button', { name: 'Update Secret' })).closest('.ant-modal') as HTMLElement;
  };

  const sentUpdate = () => {
    const call = vi.mocked(invoke).mock.calls.find(([cmd]) => cmd === 'update_vault_secret');
    return (call?.[1] as { request: Record<string, unknown> }).request;
  };

  it('clears a secret username on edit', async () => {
    const modal = await openEditModal('Admin login');
    const username = within(modal).getByLabelText('Username (optional, leave empty to remove)');
    expect(username).toHaveValue('DOMAIN\\Operator');

    fireEvent.change(username, { target: { value: '' } });
    fireEvent.click(within(modal).getByRole('button', { name: 'Update Secret' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('update_vault_secret', expect.anything()));
    expect(sentUpdate()).toEqual(expect.objectContaining({ secret_id: 11, username: '' }));
  });

  it('leaves the username out of an edit that does not change it', async () => {
    const modal = await openEditModal('Admin login');

    fireEvent.change(within(modal).getByLabelText('Label'), { target: { value: 'HMI login' } });
    fireEvent.click(within(modal).getByRole('button', { name: 'Update Secret' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('update_vault_secret', expect.anything()));
    expect(sentUpdate()).toEqual(expect.objectContaining({ secret_id: 11, label: 'HMI login' }));
    expect(sentUpdate().username).toBeUndefined();
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
