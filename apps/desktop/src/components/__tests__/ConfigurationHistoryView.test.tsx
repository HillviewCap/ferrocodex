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
    
    expect(screen.getByText('Test Asset')).toBeInTheDocument();
    expect(screen.getByText('Test asset description')).toBeInTheDocument();
    expect(screen.getByText('3 versions')).toBeInTheDocument();
    expect(screen.getByText('Latest: v3')).toBeInTheDocument();
  });

  it('shows breadcrumb navigation', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    expect(screen.getByText('Configuration Assets')).toBeInTheDocument();
    expect(screen.getByText('Version History')).toBeInTheDocument();
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
    
    expect(screen.getByText('Loading version history...')).toBeInTheDocument();
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
    
    expect(screen.getByText('No Version History')).toBeInTheDocument();
    expect(screen.getByText('This asset has no configuration versions yet')).toBeInTheDocument();
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
    
    expect(screen.getByText(/Created: Jan 1, 2023/)).toBeInTheDocument();
  });

  it('shows owner information', () => {
    render(<ConfigurationHistoryView asset={mockAsset} onBack={mockOnBack} />);
    
    expect(screen.getByText('Owner: 1')).toBeInTheDocument();
  });

  it('handles asset with no description', () => {
    const assetWithoutDescription = { ...mockAsset, description: '' };
    render(<ConfigurationHistoryView asset={assetWithoutDescription} onBack={mockOnBack} />);
    
    expect(screen.getByText('No description')).toBeInTheDocument();
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