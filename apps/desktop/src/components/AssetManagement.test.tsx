import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { message } from 'antd';
import AssetManagement from './AssetManagement';
import { AssetInfo } from '../types/assets';

// Mock the modules
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: {
      error: vi.fn(),
      success: vi.fn(),
      info: vi.fn()
    }
  };
});

// Mock stores
const mockAuthStore = {
  token: 'test-token'
};

const mockAssetStore = {
  assets: [] as AssetInfo[],
  isLoading: false,
  error: null as string | null,
  fetchAssets: vi.fn(),
  clearError: vi.fn(),
  currentView: 'dashboard' as 'dashboard' | 'history',
  selectedAsset: null as AssetInfo | null,
  navigateToHistory: vi.fn(),
  navigateToDashboard: vi.fn(),
  goldenVersions: {} as Record<number, string | null>,
  fetchGoldenVersion: vi.fn(() => Promise.resolve())
};

vi.mock('../store/auth', () => ({
  default: vi.fn(() => mockAuthStore)
}));

vi.mock('../store/assets', () => ({
  default: vi.fn(() => mockAssetStore)
}));

// Mock child components
vi.mock('./ImportConfigurationModal', () => ({
  default: ({ visible, onCancel, onSuccess }: any) => (
    visible ? (
      <div data-testid="import-modal">
        <button onClick={onCancel}>Cancel</button>
        <button onClick={() => onSuccess({ id: 1, name: 'New Asset' })}>Import</button>
      </div>
    ) : null
  )
}));

vi.mock('./ConfigurationHistoryView', () => ({
  default: ({ asset, onBack }: any) => (
    <div data-testid="history-view">
      <h2>History for {asset.name}</h2>
      <button onClick={onBack}>Back</button>
    </div>
  )
}));

describe('AssetManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    mockAssetStore.assets = [];
    mockAssetStore.isLoading = false;
    mockAssetStore.error = null;
    mockAssetStore.currentView = 'dashboard';
    mockAssetStore.selectedAsset = null;
    mockAssetStore.goldenVersions = {};
  });

  it('renders loading state when isLoading is true', () => {
    mockAssetStore.isLoading = true;
    render(<AssetManagement />);
    
    expect(screen.getByText('Loading assets...')).toBeInTheDocument();
  });

  it('renders empty state when no assets exist', () => {
    render(<AssetManagement />);
    
    expect(screen.getByText('No Configuration Assets')).toBeInTheDocument();
    expect(screen.getByText('Get started by importing your first configuration file')).toBeInTheDocument();
  });

  it('fetches assets on mount when token is available', async () => {
    render(<AssetManagement />);
    
    await waitFor(() => {
      expect(mockAssetStore.fetchAssets).toHaveBeenCalledWith('test-token');
    });
  });

  it('renders asset cards when assets exist', () => {
    const testAssets: AssetInfo[] = [
      {
        id: 1,
        name: 'Test Asset 1',
        description: 'Test Description 1',
        latest_version: 'v1.0.0',
        latest_version_notes: 'Initial version',
        version_count: 3,
        created_at: '2024-01-01T00:00:00Z',
        created_by_username: 'testuser',
        config_format: 'JSON'
      },
      {
        id: 2,
        name: 'Test Asset 2',
        description: 'Test Description 2',
        latest_version: 'v2.1.0',
        latest_version_notes: 'Updated version',
        version_count: 5,
        created_at: '2024-01-02T00:00:00Z',
        created_by_username: 'anotheruser',
        config_format: 'XML'
      }
    ];
    
    mockAssetStore.assets = testAssets;
    render(<AssetManagement />);
    
    expect(screen.getByText('Test Asset 1')).toBeInTheDocument();
    expect(screen.getByText('Test Asset 2')).toBeInTheDocument();
    expect(screen.getByText('3 versions')).toBeInTheDocument();
    expect(screen.getByText('5 versions')).toBeInTheDocument();
  });

  it('displays golden version indicator when asset has golden version', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Golden Asset',
      description: 'Has golden version',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Golden',
      version_count: 3,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    mockAssetStore.goldenVersions = { 1: 'v1.0.0' };
    
    render(<AssetManagement />);
    
    expect(screen.getByText('GOLDEN')).toBeInTheDocument();
  });

  it('opens import modal when import button is clicked', () => {
    render(<AssetManagement />);
    
    const importButton = screen.getAllByText('Import Configuration')[0];
    fireEvent.click(importButton);
    
    expect(screen.getByTestId('import-modal')).toBeInTheDocument();
  });

  it('closes import modal and shows success message on successful import', async () => {
    render(<AssetManagement />);
    
    // Open modal
    fireEvent.click(screen.getAllByText('Import Configuration')[0]);
    
    // Click import in modal
    fireEvent.click(screen.getByText('Import'));
    
    await waitFor(() => {
      expect(screen.queryByTestId('import-modal')).not.toBeInTheDocument();
      expect(message.success).toHaveBeenCalledWith('Configuration imported successfully!');
      expect(mockAssetStore.fetchAssets).toHaveBeenCalledWith('test-token');
    });
  });

  it('navigates to history view when asset card is clicked', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    const card = screen.getByText('Test Asset').closest('.ant-card');
    fireEvent.click(card!);
    
    expect(mockAssetStore.navigateToHistory).toHaveBeenCalledWith(testAsset);
  });

  it('renders history view when currentView is history', () => {
    const selectedAsset: AssetInfo = {
      id: 1,
      name: 'Selected Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.currentView = 'history';
    mockAssetStore.selectedAsset = selectedAsset;
    
    render(<AssetManagement />);
    
    expect(screen.getByTestId('history-view')).toBeInTheDocument();
    expect(screen.getByText('History for Selected Asset')).toBeInTheDocument();
  });

  it('navigates back to dashboard when back button is clicked in history view', () => {
    const selectedAsset: AssetInfo = {
      id: 1,
      name: 'Selected Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.currentView = 'history';
    mockAssetStore.selectedAsset = selectedAsset;
    
    render(<AssetManagement />);
    
    fireEvent.click(screen.getByText('Back'));
    
    expect(mockAssetStore.navigateToDashboard).toHaveBeenCalled();
  });

  it('shows info message when add version action is clicked', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    // Find the add button by its icon key
    const addButton = screen.getByRole('img', { name: 'plus' }).closest('span');
    fireEvent.click(addButton!);
    
    expect(mockAssetStore.navigateToHistory).toHaveBeenCalledWith(testAsset);
    expect(message.info).toHaveBeenCalledWith('Navigate to Version History to import a new version');
  });

  it('displays error message when error exists', async () => {
    mockAssetStore.error = 'Failed to fetch assets';
    render(<AssetManagement />);
    
    await waitFor(() => {
      expect(message.error).toHaveBeenCalledWith('Failed to fetch assets');
      expect(mockAssetStore.clearError).toHaveBeenCalled();
    });
  });

  it('fetches golden versions for all assets', async () => {
    const testAssets: AssetInfo[] = [
      {
        id: 1,
        name: 'Asset 1',
        description: 'Test',
        latest_version: 'v1.0.0',
        latest_version_notes: 'Notes',
        version_count: 1,
        created_at: '2024-01-01T00:00:00Z',
        created_by_username: 'testuser',
        config_format: 'JSON'
      },
      {
        id: 2,
        name: 'Asset 2',
        description: 'Test',
        latest_version: 'v1.0.0',
        latest_version_notes: 'Notes',
        version_count: 1,
        created_at: '2024-01-01T00:00:00Z',
        created_by_username: 'testuser',
        config_format: 'JSON'
      }
    ];
    
    mockAssetStore.assets = testAssets;
    render(<AssetManagement />);
    
    await waitFor(() => {
      expect(mockAssetStore.fetchGoldenVersion).toHaveBeenCalledTimes(2);
      expect(mockAssetStore.fetchGoldenVersion).toHaveBeenCalledWith('test-token', 1);
      expect(mockAssetStore.fetchGoldenVersion).toHaveBeenCalledWith('test-token', 2);
    });
  });

  it('handles golden version fetch errors gracefully', async () => {
    const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    
    mockAssetStore.fetchGoldenVersion.mockRejectedValueOnce('Network error');
    
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Asset 1',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    await waitFor(() => {
      expect(consoleWarnSpy).toHaveBeenCalledWith(
        'Failed to fetch golden version for asset 1:',
        'Network error'
      );
    });
    
    consoleWarnSpy.mockRestore();
  });

  it('formats dates correctly', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-15T10:30:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    // The exact format may vary by locale, so just check that it contains the date parts
    const dateElement = screen.getByText(/Jan.*15.*2024/);
    expect(dateElement).toBeInTheDocument();
  });

  it('handles invalid date formats gracefully', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: 'invalid-date',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    const { container } = render(<AssetManagement />);
    
    // The formatDate function will return "Invalid Date" for invalid date strings
    const dateElements = container.querySelectorAll('.ant-typography-secondary');
    const dateText = Array.from(dateElements).find(el => el.textContent?.includes('Invalid Date'));
    expect(dateText).toBeTruthy();
  });

  it('displays version count correctly for single version', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    expect(screen.getByText('1 version')).toBeInTheDocument();
  });

  it('prevents event propagation when action buttons are clicked', () => {
    const testAsset: AssetInfo = {
      id: 1,
      name: 'Test Asset',
      description: 'Test',
      latest_version: 'v1.0.0',
      latest_version_notes: 'Notes',
      version_count: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by_username: 'testuser',
      config_format: 'JSON'
    };
    
    mockAssetStore.assets = [testAsset];
    render(<AssetManagement />);
    
    // Click the history button
    const historyButton = screen.getByRole('img', { name: 'history' }).closest('span');
    fireEvent.click(historyButton!);
    
    // Should call navigateToHistory once (from button click, not card click)
    expect(mockAssetStore.navigateToHistory).toHaveBeenCalledTimes(1);
  });
});