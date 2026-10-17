import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ExportRecoveryPackageModal from '../ExportRecoveryPackageModal';
import { message } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock @tauri-apps/plugin-dialog
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn()
}));

// Mock antd message
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: {
      success: vi.fn(),
      error: vi.fn()
    }
  };
});

// Mock the auth store
vi.mock('../../store/auth', () => ({
  default: vi.fn(() => ({
    token: 'test-token'
  }))
}));

const mockConfiguration = {
  id: 1,
  asset_id: 1,
  version_number: 'v1',
  file_name: 'config.json',
  file_size: 1024,
  content_hash: 'hash123',
  author: 1,
  author_username: 'user1',
  notes: 'Test config',
  status: 'Golden' as const,
  firmware_version_id: 1,
  created_at: '2024-01-01T00:00:00Z'
};

const mockFirmware = {
  id: 1,
  asset_id: 1,
  author_id: 1,
  author_username: 'user1',
  vendor: 'Test Vendor',
  model: 'Test Model',
  version: '1.0.0',
  notes: 'Test firmware',
  status: 'Golden' as const,
  file_path: '/path/to/firmware',
  file_hash: 'hash456',
  file_size: 2048,
  created_at: '2024-01-01T00:00:00Z'
};

const mockManifest = '/export/path/recovery_manifest.json';

describe('ExportRecoveryPackageModal', () => {
  const mockOnCancel = vi.fn();
  const mockOnSuccess = vi.fn();
  const mockInvoke = vi.mocked(invoke);
  const mockDialog = vi.mocked(dialog);

  // The modal calls get_firmware_list and get_export_options on open, so mock by command
  // rather than by call order
  const mockCommands = (exportResult: () => Promise<unknown> = () => Promise.resolve(mockManifest)) => {
    mockInvoke.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'get_firmware_list':
          return Promise.resolve([mockFirmware]);
        case 'get_export_options':
          return Promise.resolve({ includeVault: false, vaultAvailable: false, vaultSecretCount: 0 });
        case 'export_complete_recovery':
          return exportResult();
        default:
          return Promise.reject(new Error(`Unexpected command: ${cmd}`));
      }
    });
  };

  // The chosen path shows on the picker button and in a "Selected Directory" alert
  const expectSelectedPath = (path: string) => {
    expect(screen.getByText('Selected Directory')).toBeInTheDocument();
    expect(screen.getAllByText(path)).toHaveLength(2);
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockCommands();
  });

  it('should render the modal with package contents', async () => {
    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    expect(screen.getByText('Export Recovery Package')).toBeInTheDocument();
    expect(screen.getByText('Package Contents')).toBeInTheDocument();
    expect(screen.getByText('Configuration')).toBeInTheDocument();
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.getByText('config.json')).toBeInTheDocument();
  });

  it('should fetch and display firmware information', async () => {
    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_firmware_list', {
        token: 'test-token',
        assetId: 1
      });
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
      expect(screen.getByText('Test Vendor')).toBeInTheDocument();
      expect(screen.getByText('Test Model')).toBeInTheDocument();
    });
  });

  it('should show warning when no firmware is linked', () => {
    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={{ ...mockConfiguration, firmware_version_id: undefined }}
        linkedFirmwareId={undefined}
      />
    );

    expect(screen.getByText('No firmware linked')).toBeInTheDocument();
    expect(screen.getByText(/Link a firmware version first/)).toBeInTheDocument();
  });

  it('should open directory selection dialog', async () => {
    const user = userEvent.setup();
    mockDialog.open.mockResolvedValueOnce('/export/path');

    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    await user.click(screen.getByText('Click to select export directory...'));

    expect(mockDialog.open).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: 'Select Export Directory'
    });

    await waitFor(() => {
      expectSelectedPath('/export/path');
    });
  });

  it('should export recovery package successfully', async () => {
    const user = userEvent.setup();
    mockDialog.open.mockResolvedValueOnce('/export/path');

    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    // Select directory
    await user.click(screen.getByText('Click to select export directory...'));
    
    await waitFor(() => {
      expectSelectedPath('/export/path');
    });

    // Click export
    await user.click(screen.getByRole('button', { name: /Export Package/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('export_complete_recovery', {
        app: null,
        token: 'test-token',
        assetId: 1,
        configVersionId: 1,
        firmwareVersionId: 1,
        exportDirectory: '/export/path',
        includeVault: false
      });
      expect(message.success).toHaveBeenCalledWith('Recovery package exported successfully!');
      expect(mockOnSuccess).toHaveBeenCalledWith('/export/path/recovery_manifest.json');
    });
  });

  it('should not export when no firmware is linked', async () => {
    const user = userEvent.setup();
    
    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={{ ...mockConfiguration, firmware_version_id: undefined }}
        linkedFirmwareId={undefined}
      />
    );

    // The button stays disabled, so the export can't be started at all
    const exportButton = screen.getByRole('button', { name: /Export Package/i });
    expect(exportButton).toBeDisabled();
    await user.click(exportButton);
    expect(mockInvoke).not.toHaveBeenCalledWith('export_complete_recovery', expect.anything());
  });

  it('should not export when no directory is selected', async () => {
    const user = userEvent.setup();
    
    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    // The button stays disabled until a directory is chosen
    const exportButton = screen.getByRole('button', { name: /Export Package/i });
    expect(exportButton).toBeDisabled();
    await user.click(exportButton);
    expect(mockInvoke).not.toHaveBeenCalledWith('export_complete_recovery', expect.anything());
  });

  it('should handle export error', async () => {
    const user = userEvent.setup();
    mockDialog.open.mockResolvedValueOnce('/export/path');
    mockCommands(() => Promise.reject(new Error('Export failed')));

    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    // Select directory and export
    await user.click(screen.getByText('Click to select export directory...'));
    await waitFor(() => {
      expectSelectedPath('/export/path');
    });
    await user.click(screen.getByRole('button', { name: /Export Package/i }));

    await waitFor(() => {
      expect(message.error).toHaveBeenCalledWith('Failed to export recovery package: Export failed');
      expect(mockOnSuccess).not.toHaveBeenCalled();
    });
  });

  it('should disable export button when exporting', async () => {
    const user = userEvent.setup();
    mockDialog.open.mockResolvedValueOnce('/export/path');
    
    // Make export hang
    mockCommands(() => new Promise(() => {}));

    render(
      <ExportRecoveryPackageModal
        visible={true}
        onCancel={mockOnCancel}
        onSuccess={mockOnSuccess}
        assetId={1}
        assetName="Test Asset"
        configuration={mockConfiguration}
        linkedFirmwareId={1}
      />
    );

    // Select directory
    await user.click(screen.getByText('Click to select export directory...'));
    await waitFor(() => {
      expectSelectedPath('/export/path');
    });

    const exportButton = screen.getByRole('button', { name: /Export Package/i });
    
    // Click export
    await user.click(exportButton);

    // Button should be loading/disabled
    expect(exportButton).toHaveAttribute('disabled');
  });
});