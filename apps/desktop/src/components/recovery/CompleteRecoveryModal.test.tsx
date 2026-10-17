import React from 'react';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { CompleteRecoveryModal } from './CompleteRecoveryModal';
import { ConfigurationVersionInfo, FirmwareVersionInfo } from '../../types/recovery';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// Mock auth store
vi.mock('../../store/auth', () => ({
  default: vi.fn(() => ({
    token: 'mock-token',
  })),
}));

// Mock antd message
vi.mock('antd', async () => {
  const antd = await vi.importActual('antd');
  return {
    ...antd,
    message: {
      success: vi.fn(),
      error: vi.fn(),
    },
  };
});

// Ant Design's Select shows its placeholder as text and renders each dropdown in a portal.
// Open the select found by its placeholder and click the option in the one open dropdown.
const chooseOption = async (placeholder: string, optionText: string) => {
  const select = screen.getByText(placeholder).closest('.ant-select') as HTMLElement;
  fireEvent.mouseDown(select.querySelector('.ant-select-selector')!);
  const dropdown = await waitFor(() => {
    const open = document.querySelectorAll('.ant-select-dropdown:not(.ant-select-dropdown-hidden)');
    expect(open).toHaveLength(1);
    return open[0] as HTMLElement;
  });
  fireEvent.click(within(dropdown).getByText(optionText));
  await waitFor(() => expect(dropdown).toHaveClass('ant-select-dropdown-hidden'));
};

const getExportButton = () => screen.getByRole('button', { name: /Export Recovery Package/ });

// Select config 1.0.0 + firmware 2.0.0 (linked), pick a directory, and wait for export to be enabled
const fillExportForm = async () => {
  await chooseOption('Select configuration version', '1.0.0');
  await chooseOption('Select firmware version', '2.0.0');
  fireEvent.click(screen.getByText('Browse'));
  await waitFor(() => expect(getExportButton()).not.toBeDisabled());
};

describe('CompleteRecoveryModal', () => {
  const mockConfigVersions: ConfigurationVersionInfo[] = [
    {
      id: 1,
      asset_id: 1,
      author_id: 1,
      author_username: 'testuser',
      version_number: '1.0.0',
      file_name: 'config.json',
      notes: 'Test config',
      status: 'Golden',
      firmware_version_id: 1,
      file_size: 1024,
      content_hash: 'abc123',
      branch_id: null,
      is_silver: false,
      promoted_from_branch_id: null,
      promoted_from_version_id: null,
      status_changed_at: null,
      status_changed_by: null,
      created_at: '2024-01-01T00:00:00Z',
    },
    {
      id: 2,
      asset_id: 1,
      author_id: 1,
      author_username: 'testuser',
      version_number: '2.0.0',
      file_name: 'config.json',
      notes: 'Updated config',
      status: 'Silver',
      firmware_version_id: 2,
      file_size: 2048,
      content_hash: 'def456',
      branch_id: null,
      is_silver: true,
      promoted_from_branch_id: null,
      promoted_from_version_id: null,
      status_changed_at: null,
      status_changed_by: null,
      created_at: '2024-01-02T00:00:00Z',
    },
  ];

  const mockFirmwareVersions: FirmwareVersionInfo[] = [
    {
      id: 1,
      asset_id: 1,
      author_id: 1,
      author_username: 'testuser',
      vendor: 'Test Vendor',
      model: 'Test Model',
      version: '2.0.0',
      notes: 'Test firmware',
      status: 'Golden',
      file_path: '1/1.enc',
      file_hash: 'firmware123',
      file_size: 4096,
      created_at: '2024-01-01T00:00:00Z',
    },
    {
      id: 2,
      asset_id: 1,
      author_id: 1,
      author_username: 'testuser',
      vendor: 'Test Vendor',
      model: 'Test Model',
      version: '3.0.0',
      notes: 'Updated firmware',
      status: 'Approved',
      file_path: '1/2.enc',
      file_hash: 'firmware456',
      file_size: 8192,
      created_at: '2024-01-02T00:00:00Z',
    },
  ];

  const defaultProps = {
    visible: true,
    onClose: vi.fn(),
    assetId: 1,
    assetName: 'Test Asset',
    configurationVersions: mockConfigVersions,
    firmwareVersions: mockFirmwareVersions,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders modal with correct title', () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    expect(screen.getByText(/Complete Recovery Export - Test Asset/)).toBeInTheDocument();
  });

  it('shows configuration and firmware version selectors', () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    expect(screen.getByText('Configuration Version:')).toBeInTheDocument();
    expect(screen.getByText('Firmware Version:')).toBeInTheDocument();
    expect(screen.getByText('Select configuration version')).toBeInTheDocument();
    expect(screen.getByText('Select firmware version')).toBeInTheDocument();
  });

  it('shows export directory selector', () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    expect(screen.getByText('Export Directory:')).toBeInTheDocument();
    expect(screen.getByText('Browse')).toBeInTheDocument();
  });

  it('disables export button when required fields are not selected', () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    const exportButton = screen.getByRole('button', { name: /Export Recovery Package/ });
    expect(exportButton).toBeDisabled();
  });

  it('enables export button when all fields are selected', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(open).mockResolvedValue('/test/export/path');

    render(<CompleteRecoveryModal {...defaultProps} />);
    
    await chooseOption('Select configuration version', '1.0.0');
    await chooseOption('Select firmware version', '2.0.0');
    expect(getExportButton()).toBeDisabled(); // still needs a directory

    fireEvent.click(screen.getByText('Browse'));

    await waitFor(() => {
      expect(getExportButton()).not.toBeDisabled();
    });
  });

  it('shows compatibility indicator for linked versions', async () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    // Select linked configuration and firmware (config 1 has firmware_version_id: 1)
    await chooseOption('Select configuration version', '1.0.0');
    await chooseOption('Select firmware version', '2.0.0');

    await waitFor(() => {
      expect(screen.getByText(/Versions are compatible/)).toBeInTheDocument();
    });
  });

  it('shows non-compatibility warning for unlinked versions', async () => {
    render(<CompleteRecoveryModal {...defaultProps} />);
    
    // Select non-linked configuration and firmware
    await chooseOption('Select configuration version', '1.0.0');
    await chooseOption('Select firmware version', '3.0.0'); // firmware id 2, not linked to config 1

    await waitFor(() => {
      expect(screen.getByText(/Versions are not linked/)).toBeInTheDocument();
    });
  });

  it('calls invoke with correct parameters on export', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');
    
    vi.mocked(open).mockResolvedValue('/test/export/path');
    vi.mocked(invoke).mockResolvedValue({
      asset_id: 1,
      export_date: '2024-01-01T00:00:00Z',
      exported_by: 'testuser',
      configuration: {
        version_id: 1,
        version_number: '1.0.0',
        filename: 'Test_Asset_config_v1.0.0.json',
        checksum: 'abc123',
        file_size: 1024,
      },
      firmware: {
        version_id: 1,
        version: '2.0.0',
        filename: 'Test_Asset_firmware_v2.0.0.bin',
        checksum: 'def456',
        vendor: 'Test Vendor',
        model: 'Test Model',
        file_size: 4096,
      },
      compatibility_verified: true,
    });

    render(<CompleteRecoveryModal {...defaultProps} />);
    
    await fillExportForm();
    fireEvent.click(getExportButton());

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('export_complete_recovery', {
        token: 'mock-token',
        assetId: 1,
        configVersionId: 1,
        firmwareVersionId: 1,
        exportDirectory: '/test/export/path',
      });
    });
  });

  it('shows success message on successful export', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');
    const { message } = await import('antd');
    
    vi.mocked(open).mockResolvedValue('/test/export/path');
    vi.mocked(invoke).mockResolvedValue({
      asset_id: 1,
      export_date: '2024-01-01T00:00:00Z',
      exported_by: 'testuser',
      configuration: {
        version_id: 1,
        version_number: '1.0.0',
        filename: 'Test_Asset_config_v1.0.0.json',
        checksum: 'abc123',
        file_size: 1024,
      },
      firmware: {
        version_id: 1,
        version: '2.0.0',
        filename: 'Test_Asset_firmware_v2.0.0.bin',
        checksum: 'def456',
        vendor: 'Test Vendor',
        model: 'Test Model',
        file_size: 4096,
      },
      compatibility_verified: true,
    });

    render(<CompleteRecoveryModal {...defaultProps} />);
    
    await fillExportForm();
    fireEvent.click(getExportButton());

    await waitFor(() => {
      expect(message.success).toHaveBeenCalledWith('Complete recovery package exported successfully!');
    });
  });

  it('shows error message on failed export', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');
    const { message } = await import('antd');
    
    vi.mocked(open).mockResolvedValue('/test/export/path');
    vi.mocked(invoke).mockRejectedValue(new Error('Export failed'));

    render(<CompleteRecoveryModal {...defaultProps} />);
    
    await fillExportForm();
    fireEvent.click(getExportButton());

    await waitFor(() => {
      expect(message.error).toHaveBeenCalledWith('Export failed: Export failed');
    });
  });

  it('auto-selects single options', () => {
    const propsWithSingleOptions = {
      ...defaultProps,
      configurationVersions: [mockConfigVersions[0]],
      firmwareVersions: [mockFirmwareVersions[0]],
    };

    render(<CompleteRecoveryModal {...propsWithSingleOptions} />);
    
    // Should auto-select the single options
    expect(screen.queryByText('Select configuration version')).not.toBeInTheDocument();
    expect(screen.queryByText('Select firmware version')).not.toBeInTheDocument();
    expect(screen.getByText('Selected: 1.0.0 + 2.0.0')).toBeInTheDocument();
  });

  it('calls onClose when cancel is clicked', () => {
    const onClose = vi.fn();
    render(<CompleteRecoveryModal {...defaultProps} onClose={onClose} />);
    
    const cancelButton = screen.getByRole('button', { name: /Cancel/ });
    fireEvent.click(cancelButton);
    
    expect(onClose).toHaveBeenCalled();
  });
});