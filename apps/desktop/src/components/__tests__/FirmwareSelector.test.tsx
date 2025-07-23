import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import FirmwareSelector from '../FirmwareSelector';
import { message } from 'antd';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock antd message
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn()
    }
  };
});

const mockFirmwareVersions = [
  {
    id: 1,
    asset_id: 1,
    author_id: 1,
    author_username: 'user1',
    vendor: 'Vendor A',
    model: 'Model X',
    version: '1.0.0',
    notes: 'Initial version',
    status: 'Draft' as const,
    file_path: '/path/to/firmware1',
    file_hash: 'hash1',
    file_size: 1024,
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: 2,
    asset_id: 1,
    author_id: 1,
    author_username: 'user1',
    vendor: 'Vendor B',
    model: 'Model Y',
    version: '2.0.0',
    notes: 'Updated version',
    status: 'Golden' as const,
    file_path: '/path/to/firmware2',
    file_hash: 'hash2',
    file_size: 2048,
    created_at: '2024-01-02T00:00:00Z'
  }
];

describe('FirmwareSelector', () => {
  const mockOnLink = vi.fn();
  const mockOnUnlink = vi.fn();
  const mockInvoke = vi.mocked(await import('@tauri-apps/api/core')).invoke;

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(mockFirmwareVersions);
  });

  it('should render the firmware selector', async () => {
    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    expect(screen.getByPlaceholderText('Select firmware version')).toBeInTheDocument();
    expect(screen.getByText('Link')).toBeInTheDocument();
  });

  it('should fetch and display firmware versions', async () => {
    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_firmware_list', {
        token: 'test-token',
        assetId: 1
      });
    });
  });

  it('should show unlink button when firmware is already linked', () => {
    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        currentFirmwareId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    expect(screen.getByText('Unlink')).toBeInTheDocument();
    expect(screen.queryByText('Link')).not.toBeInTheDocument();
  });

  it('should link firmware when link button is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(mockFirmwareVersions);
    mockInvoke.mockResolvedValueOnce(undefined); // link_firmware_to_configuration

    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    // Wait for firmware list to load
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Select firmware version')).toBeInTheDocument();
    });

    // Open dropdown
    await user.click(screen.getByPlaceholderText('Select firmware version'));

    // Select firmware
    await waitFor(() => {
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
    });
    await user.click(screen.getByText('1.0.0'));

    // Click link button
    await user.click(screen.getByText('Link'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('link_firmware_to_configuration', {
        token: 'test-token',
        configId: 1,
        firmwareId: 1
      });
      expect(message.success).toHaveBeenCalledWith('Firmware linked successfully');
      expect(mockOnLink).toHaveBeenCalledWith(1);
    });
  });

  it('should unlink firmware when unlink button is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(mockFirmwareVersions);
    mockInvoke.mockResolvedValueOnce(undefined); // unlink_firmware_from_configuration

    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        currentFirmwareId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    await user.click(screen.getByText('Unlink'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('unlink_firmware_from_configuration', {
        token: 'test-token',
        configId: 1
      });
      expect(message.success).toHaveBeenCalledWith('Firmware unlinked successfully');
      expect(mockOnUnlink).toHaveBeenCalled();
    });
  });

  it('should show warning when trying to link without selecting firmware', async () => {
    const user = userEvent.setup();
    
    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    await user.click(screen.getByText('Link'));

    expect(message.warning).toHaveBeenCalledWith('Please select a firmware version to link');
    expect(mockInvoke).not.toHaveBeenCalledWith('link_firmware_to_configuration', expect.any(Object));
  });

  it('should handle link error', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(mockFirmwareVersions);
    mockInvoke.mockRejectedValueOnce(new Error('Link failed'));

    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
      />
    );

    // Select firmware
    await user.click(screen.getByPlaceholderText('Select firmware version'));
    await waitFor(() => {
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
    });
    await user.click(screen.getByText('1.0.0'));

    // Try to link
    await user.click(screen.getByText('Link'));

    await waitFor(() => {
      expect(message.error).toHaveBeenCalledWith('Failed to link firmware: Link failed');
      expect(mockOnLink).not.toHaveBeenCalled();
    });
  });

  it('should be disabled when disabled prop is true', () => {
    render(
      <FirmwareSelector
        assetId={1}
        configId={1}
        token="test-token"
        onLink={mockOnLink}
        onUnlink={mockOnUnlink}
        disabled={true}
      />
    );

    expect(screen.getByPlaceholderText('Select firmware version')).toBeDisabled();
    expect(screen.getByText('Link')).toBeDisabled();
  });
});