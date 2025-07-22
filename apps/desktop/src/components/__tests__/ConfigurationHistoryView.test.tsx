import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ConfigurationHistoryView from '../ConfigurationHistoryView';
import { AssetInfo } from '../../types/assets';

// Mock the stores
vi.mock('../../store/auth', () => ({
  default: vi.fn(() => ({
    token: 'mock-token'
  }))
}));

vi.mock('../../store/assets', () => ({
  default: vi.fn(() => ({
    versions: [],
    versionsLoading: false,
    error: null,
    fetchVersions: vi.fn(),
    clearError: vi.fn()
  }))
}));

// Mock the child components
vi.mock('../VersionHistoryList', () => ({
  default: ({ versions }: { versions: any[] }) => (
    <div data-testid="version-history-list">
      {versions.map(v => (
        <div key={v.id}>{v.version_number}</div>
      ))}
    </div>
  )
}));

describe('ConfigurationHistoryView', () => {
  const mockAsset: AssetInfo = {
    id: 1,
    name: 'Test Asset',
    description: 'Test asset description',
    created_by: 1,
    created_at: '2023-01-01T12:00:00Z',
    version_count: 3,
    latest_version: 'v3'
  };

  const mockOnBack = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders asset information correctly', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const testAssetElements = screen.getAllByText('Test Asset');
    expect(testAssetElements.length).toBeGreaterThan(0);
    const descriptionElements = screen.getAllByText('Test asset description');
    expect(descriptionElements.length).toBeGreaterThan(0);
    const versionsElements = screen.getAllByText('3 versions');
    expect(versionsElements.length).toBeGreaterThan(0);
    const latestElements = screen.getAllByText('Latest: v3');
    expect(latestElements.length).toBeGreaterThan(0);
  });

  it('shows breadcrumb navigation', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const configAssetsElements = screen.getAllByText('Configuration Assets');
    expect(configAssetsElements.length).toBeGreaterThan(0);
    const versionHistoryElements = screen.getAllByText('Version History');
    expect(versionHistoryElements.length).toBeGreaterThan(0);
  });

  it('calls onBack when back button is clicked', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const backButton = screen.getByText('Configuration Assets');
    fireEvent.click(backButton);
    
    expect(mockOnBack).toHaveBeenCalledTimes(1);
  });

  it('shows loading state when versions are loading', () => {
    const useAssetStore = vi.fn(() => ({
      versions: [],
      versionsLoading: true,
      error: null,
      fetchVersions: vi.fn(),
      clearError: vi.fn()
    }));

    vi.doMock('../../store/assets', () => ({
      default: useAssetStore
    }));

    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const loadingVersionsElements = screen.getAllByText('Loading version history...');
    expect(loadingVersionsElements.length).toBeGreaterThan(0);
  });

  it('shows empty state when no versions exist', () => {
    const useAssetStore = vi.fn(() => ({
      versions: [],
      versionsLoading: false,
      error: null,
      fetchVersions: vi.fn(),
      clearError: vi.fn()
    }));

    vi.doMock('../../store/assets', () => ({
      default: useAssetStore
    }));

    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const noVersionHistoryElements = screen.getAllByText('No Version History');
    expect(noVersionHistoryElements.length).toBeGreaterThan(0);
    const noVersionsYetElements = screen.getAllByText('This asset has no configuration versions yet');
    expect(noVersionsYetElements.length).toBeGreaterThan(0);
  });

  it('renders version history list when versions exist', () => {
    const mockVersions = [
      {
        id: 1,
        asset_id: 1,
        version_number: 'v1',
        file_name: 'config.json',
        file_size: 1024,
        content_hash: 'abc123',
        author: 1,
        author_username: 'john_doe',
        notes: 'Initial version',
        created_at: '2023-01-01T12:00:00Z'
      }
    ];

    const useAssetStore = vi.fn(() => ({
      versions: mockVersions,
      versionsLoading: false,
      error: null,
      fetchVersions: vi.fn(),
      clearError: vi.fn()
    }));

    vi.doMock('../../store/assets', () => ({
      default: useAssetStore
    }));

    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    expect(screen.getByTestId('version-history-list')).toBeInTheDocument();
  });

  it('displays formatted dates correctly', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const createdDateElements = screen.getAllByText(/Created: Jan 1, 2023/);
    expect(createdDateElements.length).toBeGreaterThan(0);
  });

  it('shows owner information', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    const ownerElements = screen.getAllByText('Owner: 1');
    expect(ownerElements.length).toBeGreaterThan(0);
  });

  it('handles asset with no description', () => {
    const assetWithoutDescription = { ...mockAsset, description: '' };
    render(<ConfigurationHistoryView asset={assetWithoutDescription} onBack={mockOnBack} />);
    
    const noDescriptionElements = screen.getAllByText('No description');
    expect(noDescriptionElements.length).toBeGreaterThan(0);
  });

  it('calls fetchVersions on mount', () => {
    const mockFetchVersions = vi.fn();
    const useAssetStore = vi.fn(() => ({
      versions: [],
      versionsLoading: false,
      error: null,
      fetchVersions: mockFetchVersions,
      clearError: vi.fn()
    }));

    vi.doMock('../../store/assets', () => ({
      default: useAssetStore
    }));

    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    expect(mockFetchVersions).toHaveBeenCalledWith('mock-token', 1);
  });
});