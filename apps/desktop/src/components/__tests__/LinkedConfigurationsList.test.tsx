import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LinkedConfigurationsList from '../LinkedConfigurationsList';
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

// Mock the asset store
vi.mock('../../store/assets', () => ({
  default: vi.fn(() => ({
    navigateToHistory: vi.fn()
  }))
}));

const mockLinkedConfigs = [
  {
    id: 1,
    asset_id: 1,
    version_number: 'v1',
    file_name: 'config1.json',
    file_size: 1024,
    content_hash: 'hash1',
    author: 1,
    author_username: 'user1',
    notes: 'Initial config',
    status: 'Draft' as const,
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: 2,
    asset_id: 1,
    version_number: 'v2',
    file_name: 'config2.json',
    file_size: 2048,
    content_hash: 'hash2',
    author: 1,
    author_username: 'user1',
    notes: 'Updated config',
    status: 'Golden' as const,
    created_at: '2024-01-02T00:00:00Z'
  }
];

describe('LinkedConfigurationsList', () => {
  const mockInvoke = vi.mocked(await import('@tauri-apps/api/core')).invoke;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render loading state initially', async () => {
    mockInvoke.mockImplementation(() => new Promise(() => {})); // Never resolves

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should display linked configurations', async () => {
    mockInvoke.mockResolvedValueOnce(mockLinkedConfigs);

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_configurations_by_firmware', {
        token: 'test-token',
        firmwareId: 1
      });
      expect(screen.getByText('2 linked configurations')).toBeInTheDocument();
      expect(screen.getByText('v1')).toBeInTheDocument();
      expect(screen.getByText('v2')).toBeInTheDocument();
    });
  });

  it('should display single configuration text correctly', async () => {
    mockInvoke.mockResolvedValueOnce([mockLinkedConfigs[0]]);

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('1 linked configuration')).toBeInTheDocument();
    });
  });

  it('should display no linked configurations message', async () => {
    mockInvoke.mockResolvedValueOnce([]);

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('No linked configurations')).toBeInTheDocument();
    });
  });

  it('should handle error when fetching configurations', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Failed to fetch'));

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      expect(message.error).toHaveBeenCalledWith('Failed to load linked configurations');
      expect(screen.getByText('No linked configurations')).toBeInTheDocument();
    });
  });

  it('should navigate to configuration history on click', async () => {
    const { navigateToHistory } = await import('../../store/assets').then(m => m.default());
    mockInvoke.mockResolvedValueOnce(mockLinkedConfigs);

    const user = userEvent.setup();

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('v1')).toBeInTheDocument();
    });

    await user.click(screen.getByText('v1'));

    expect(navigateToHistory).toHaveBeenCalledWith(expect.objectContaining({
      id: 1
    }));
  });

  it('should apply golden color for golden status configurations', async () => {
    mockInvoke.mockResolvedValueOnce(mockLinkedConfigs);

    render(
      <LinkedConfigurationsList
        firmwareId={1}
        assetId={1}
      />
    );

    await waitFor(() => {
      const v2Tag = screen.getByText('v2').closest('.ant-tag');
      expect(v2Tag).toHaveClass('ant-tag-gold');
    });
  });
});