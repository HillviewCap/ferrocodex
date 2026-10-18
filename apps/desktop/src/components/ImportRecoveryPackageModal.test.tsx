import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import ImportRecoveryPackageModal from './ImportRecoveryPackageModal';
import { AssetInfo } from '../types/assets';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

const assets: AssetInfo[] = [
  {
    id: 1,
    name: 'PLC-001',
    description: '',
    asset_type: 'Device',
    parent_id: null,
    sort_order: 0,
    created_by: 1,
    created_by_username: 'admin',
    created_at: '2024-01-01T00:00:00Z',
    version_count: 2,
    latest_version: 'v2',
    latest_version_notes: null,
  },
];

const manifest = {
  assetId: 1,
  exportDate: '2024-03-01T12:00:00Z',
  exportedBy: 'admin',
  configuration: { versionId: 2, versionNumber: '2', filename: 'plc-config.json', checksum: 'a', fileSize: 10 },
  firmware: { versionId: 3, version: '4.2.3', filename: 'fw.bin', checksum: 'b', vendor: 'Siemens', model: 'S7-1200', fileSize: 20 },
  vault: { vaultId: 4, vaultName: 'PLC vault', filename: 'vault.enc', checksum: 'c', secretCount: 3, fileSize: 5, encrypted: true },
  compatibilityVerified: true,
};

describe('ImportRecoveryPackageModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(dialog.open).mockResolvedValue('/bundles/plc-001');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'validate_bundle_integrity') return manifest;
      if (cmd === 'import_recovery_bundle') return manifest;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders the bundle-selection step with its alert', () => {
    render(<ImportRecoveryPackageModal visible onCancel={vi.fn()} onSuccess={vi.fn()} assets={assets} />);

    expect(screen.getAllByText('Import Recovery Bundle').length).toBe(2); // modal title + alert title
    expect(screen.getByText(/Select a recovery bundle directory that was previously exported/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Select Recovery Bundle Directory/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('validates the bundle, shows its contents and imports it', async () => {
    const onSuccess = vi.fn();
    render(<ImportRecoveryPackageModal visible onCancel={vi.fn()} onSuccess={onSuccess} assets={assets} />);

    fireEvent.click(screen.getByRole('button', { name: /Select Recovery Bundle Directory/ }));

    expect(await screen.findByText('Bundle Contents')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('validate_bundle_integrity', { token: 'test-token', bundlePath: '/bundles/plc-001' });
    expect(screen.getByText('Exported By:')).toBeInTheDocument();
    expect(screen.getByText('Configuration')).toBeInTheDocument();
    expect(screen.getByText('v2')).toBeInTheDocument();
    expect(screen.getByText('Firmware')).toBeInTheDocument();
    expect(screen.getByText('v4.2.3')).toBeInTheDocument();
    expect(screen.getByText('Siemens S7-1200')).toBeInTheDocument();
    expect(screen.getByText('Identity Vault')).toBeInTheDocument();
    expect(screen.getByText('3 secrets')).toBeInTheDocument();
    expect(screen.getByText(/verified compatible/)).toBeInTheDocument();
    expect(screen.getByText('Import Identity Vault')).toBeInTheDocument();
    expect(screen.getByLabelText('Target Asset')).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText('Export passphrase'), { target: { value: 'substation-7 recovery' } });
    const importButton = screen.getByRole('button', { name: /Import Bundle/ });
    expect(importButton).toBeEnabled();
    fireEvent.click(importButton);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('import_recovery_bundle', {
        app: null,
        token: 'test-token',
        bundlePath: '/bundles/plc-001',
        targetAssetId: 1,
        importVault: true,
        vaultPassphrase: 'substation-7 recovery',
      });
    });
    expect(await screen.findByText('Import Successful!')).toBeInTheDocument();
    // Footer "Close" plus the modal's own close (X) button
    expect(screen.getAllByRole('button', { name: 'Close' }).length).toBeGreaterThanOrEqual(2);
  });

  it('requires the export passphrase to import the vault', async () => {
    render(<ImportRecoveryPackageModal visible onCancel={vi.fn()} onSuccess={vi.fn()} assets={assets} />);

    fireEvent.click(screen.getByRole('button', { name: /Select Recovery Bundle Directory/ }));
    await screen.findByText('Bundle Contents');
    fireEvent.click(screen.getByRole('button', { name: /Import Bundle/ }));

    expect(await screen.findByText('Please enter the export passphrase')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('import_recovery_bundle', expect.anything());
  });

  it('shows the error result when validation fails', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('manifest missing'));
    render(<ImportRecoveryPackageModal visible onCancel={vi.fn()} onSuccess={vi.fn()} assets={assets} />);

    fireEvent.click(screen.getByRole('button', { name: /Select Recovery Bundle Directory/ }));

    expect(await screen.findByText('Import Failed')).toBeInTheDocument();
    expect(screen.getByText('Bundle validation failed: manifest missing')).toBeInTheDocument();
  });
});
