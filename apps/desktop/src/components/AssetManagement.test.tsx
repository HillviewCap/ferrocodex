import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import AssetManagement from './AssetManagement';
import { AssetInfo } from '../types/assets';

// Mock stores
const mockAssetStore = {
  currentView: 'dashboard' as 'dashboard' | 'history',
  selectedAsset: null as AssetInfo | null,
  navigateToHistory: vi.fn(),
  navigateToDashboard: vi.fn(),
};

vi.mock('../store/assets', () => ({
  default: vi.fn(() => mockAssetStore)
}));

// Mock child components
vi.mock('./ImportConfigurationModal', () => ({
  default: ({ visible }: any) => (visible ? <div data-testid="import-modal" /> : null)
}));

vi.mock('./ConfigurationHistoryView', () => ({
  default: ({ asset, onBack }: any) => (
    <div data-testid="history-view">
      <h2>History for {asset.name}</h2>
      <button onClick={onBack}>Back</button>
    </div>
  )
}));

vi.mock('./hierarchy/AssetHierarchyView', () => ({
  AssetHierarchyView: ({ onNavigateToHistory }: any) => (
    <div data-testid="hierarchy-view">
      <button onClick={() => onNavigateToHistory(testAsset)}>Open History</button>
    </div>
  )
}));

const testAsset: AssetInfo = {
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

describe('AssetManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    mockAssetStore.currentView = 'dashboard';
    mockAssetStore.selectedAsset = null;
  });

  it('renders the asset hierarchy view on the dashboard', () => {
    render(<AssetManagement />);

    expect(screen.getByTestId('hierarchy-view')).toBeInTheDocument();
    expect(screen.queryByTestId('history-view')).not.toBeInTheDocument();
    expect(screen.queryByTestId('import-modal')).not.toBeInTheDocument();
  });

  it('navigates to history when the hierarchy view requests it', () => {
    render(<AssetManagement />);

    fireEvent.click(screen.getByText('Open History'));

    expect(mockAssetStore.navigateToHistory).toHaveBeenCalledWith(testAsset);
  });

  it('renders history view when currentView is history', () => {
    mockAssetStore.currentView = 'history';
    mockAssetStore.selectedAsset = testAsset;

    render(<AssetManagement />);

    expect(screen.getByTestId('history-view')).toBeInTheDocument();
    expect(screen.getByText('History for Selected Asset')).toBeInTheDocument();
    expect(screen.queryByTestId('hierarchy-view')).not.toBeInTheDocument();
  });

  it('falls back to the hierarchy view when history is selected without an asset', () => {
    mockAssetStore.currentView = 'history';

    render(<AssetManagement />);

    expect(screen.getByTestId('hierarchy-view')).toBeInTheDocument();
    expect(screen.queryByTestId('history-view')).not.toBeInTheDocument();
  });

  it('navigates back to dashboard when back button is clicked in history view', () => {
    mockAssetStore.currentView = 'history';
    mockAssetStore.selectedAsset = testAsset;

    render(<AssetManagement />);

    fireEvent.click(screen.getByText('Back'));

    expect(mockAssetStore.navigateToDashboard).toHaveBeenCalled();
  });
});
