import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import ExportRecoveryPackageModal from './ExportRecoveryPackageModal';
import useAuthStore from '../store/auth';
import { ConfigurationVersionInfo } from '../types/assets';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock('./ExportPassphraseFields', () => ({
  default: () => <div data-testid="export-passphrase-fields" />,
}));

const configuration: ConfigurationVersionInfo = {
  id: 7,
  asset_id: 1,
  version_number: 'v1',
  file_name: 'config.json',
  file_size: 10,
  content_hash: 'abc',
  author: 1,
  author_username: 'admin',
  notes: '',
  status: 'Approved',
  created_at: '2024-01-01T00:00:00Z',
} as ConfigurationVersionInfo;

const renderModal = () =>
  render(
    <ExportRecoveryPackageModal
      visible
      onCancel={vi.fn()}
      onSuccess={vi.fn()}
      assetId={1}
      assetName="PLC-001"
      configuration={configuration}
    />
  );

describe('ExportRecoveryPackageModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', isAuthenticated: true });
  });

  it('shows vault options from the snake_case response the backend sends', async () => {
    // Mirrors the Rust ExportOptions struct, which has no serde rename.
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_export_options') {
        return { include_vault: true, vault_available: true, vault_secret_count: 3 };
      }
      throw new Error(`Unexpected command: ${cmd}`);
    });

    renderModal();

    expect(await screen.findByText('3 secrets')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_export_options', { token: 'test-token', assetId: 1 });
    const checkbox = screen.getByRole('checkbox', { name: /Include Identity Vault/ });
    expect(checkbox).toBeChecked();
    expect(screen.getByTestId('export-passphrase-fields')).toBeInTheDocument();
  });

  it('hides vault options when no vault is available', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_export_options') {
        return { include_vault: false, vault_available: false, vault_secret_count: 0 };
      }
      throw new Error(`Unexpected command: ${cmd}`);
    });

    renderModal();

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_export_options', expect.anything()));
    expect(screen.queryByText(/Include Identity Vault/)).not.toBeInTheDocument();
    expect(screen.queryByText(/secrets/)).not.toBeInTheDocument();
  });
});
