import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, within, act } from '@testing-library/react';
import { App } from 'antd';
import BranchManagement from './BranchManagement';
import { AssetInfo } from '../types/assets';
import { BranchInfo } from '../types/branches';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock stores
const mockAuthStore = {
  token: 'test-token'
};

const mockAssetStore = {
  versions: []
};

const mockBranchStore = {
  branchVersions: {},
  fetchBranchVersions: vi.fn()
};

vi.mock('../store/auth', () => ({
  default: vi.fn(() => mockAuthStore)
}));

vi.mock('../store/assets', () => ({
  default: vi.fn(() => mockAssetStore)
}));

vi.mock('../store/branches', () => ({
  default: vi.fn(() => mockBranchStore)
}));

// Mock child components
vi.mock('./BranchCard', () => ({
  default: ({ branch, onViewDetails, onSelectBranch, onImportVersion, onExportLatestVersion, onViewHistory, onPromoteToSilver }: any) => (
    <div data-testid={`branch-card-${branch.id}`}>
      <h3>{branch.name}</h3>
      <p>{branch.description}</p>
      {onViewDetails && <button onClick={() => onViewDetails(branch)}>View Details</button>}
      {onSelectBranch && <button onClick={() => onSelectBranch(branch)}>Select</button>}
      {onImportVersion && <button onClick={() => onImportVersion(branch)}>Import Version</button>}
      {onExportLatestVersion && <button onClick={() => onExportLatestVersion(branch)}>Export</button>}
      {onViewHistory && <button onClick={() => onViewHistory(branch)}>View History</button>}
      {onPromoteToSilver && <button onClick={() => onPromoteToSilver(branch)}>Promote to Silver</button>}
    </div>
  )
}));

vi.mock('./ImportVersionToBranchModal', () => ({
  default: ({ visible, onCancel, onSuccess, branch }: any) => (
    visible ? (
      <div data-testid="import-modal">
        <h3>Import to {branch.name}</h3>
        <button onClick={onCancel}>Cancel</button>
        <button onClick={onSuccess}>Import</button>
      </div>
    ) : null
  )
}));

vi.mock('./BranchVersionHistory', () => ({
  default: ({ branch }: any) => (
    <div data-testid="branch-history">
      <h3>History for {branch.name}</h3>
    </div>
  )
}));

vi.mock('./ExportConfirmationModal', () => ({
  default: ({ visible, onCancel, onSuccess }: any) => (
    visible ? (
      <div data-testid="export-modal">
        <button onClick={onCancel}>Cancel</button>
        <button onClick={() => onSuccess('/path/to/export')}>Export</button>
      </div>
    ) : null
  )
}));

vi.mock('./PromoteBranchToSilverModal', () => ({
  default: ({ visible, onCancel, onSuccess, branch }: any) => (
    visible ? (
      <div data-testid="promote-modal">
        <h3>Promote {branch.name} to Silver</h3>
        <button onClick={onCancel}>Cancel</button>
        <button onClick={onSuccess}>Promote</button>
      </div>
    ) : null
  )
}));

// Test data
const testAsset: AssetInfo = {
  id: 1,
  name: 'Test Asset',
  description: 'Test Description',
  latest_version: 'v1.0.0',
  latest_version_notes: 'Initial version',
  version_count: 3,
  created_at: '2024-01-01T00:00:00Z',
  created_by_username: 'testuser',
  config_format: 'JSON'
};

const testBranches: BranchInfo[] = [
  {
    id: 1,
    name: 'Feature Branch 1',
    description: 'Testing new features',
    asset_id: 1,
    parent_version_id: 1,
    parent_version_number: 'v1.0.0',
    parent_version_status: 'Active',
    is_active: true,
    created_by_username: 'user1',
    created_at: '2024-01-02T00:00:00Z',
    updated_at: '2024-01-02T00:00:00Z',
    latest_version: 'b1.1.0',
    latest_version_notes: 'Latest update',
    version_count: 2
  },
  {
    id: 2,
    name: 'Feature Branch 2',
    description: 'Another test branch',
    asset_id: 1,
    parent_version_id: 2,
    parent_version_number: 'v2.0.0',
    parent_version_status: 'Archived',
    is_active: false,
    created_by_username: 'user2',
    created_at: '2024-01-03T00:00:00Z',
    updated_at: '2024-01-03T00:00:00Z',
    latest_version: null,
    latest_version_notes: null,
    version_count: 0
  }
];

describe('BranchManagement', () => {
  const mockMessage = {
    error: vi.fn(),
    success: vi.fn(),
    warning: vi.fn()
  };

  const renderWithApp = (component: React.ReactElement) => {
    return render(
      <App message={mockMessage as any}>
        {component}
      </App>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockBranchStore.branchVersions = {};
    mockAssetStore.versions = [];
    (invoke as any).mockResolvedValue(testBranches);
  });

  it('renders with asset information', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByText('Branch Management')).toBeInTheDocument();
      expect(screen.getByText(`Manage configuration branches for ${testAsset.name}`)).toBeInTheDocument();
    });
  });

  it('fetches branches on mount', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_branches', {
        token: 'test-token',
        assetId: testAsset.id
      });
    });
  });

  it('displays branch statistics', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      // Find the statistics cards
      const cards = screen.getAllByRole('article'); // Ant Design cards have role="article"
      expect(cards).toHaveLength(3);
      
      // Check for the statistic values
      const activeCount = screen.getByText('Active Branches').closest('div')?.parentElement;
      expect(activeCount).toHaveTextContent('1'); // 1 active branch in test data
      
      const totalCount = screen.getByText('Total Branches').closest('div')?.parentElement;
      expect(totalCount).toHaveTextContent('2'); // 2 total branches in test data
      
      const versionCount = screen.getByText('Total Versions').closest('div')?.parentElement;
      expect(versionCount).toHaveTextContent('3'); // 3 versions from asset
    });
  });

  it('renders branch cards', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
      expect(screen.getByTestId('branch-card-2')).toBeInTheDocument();
      expect(screen.getByText('Feature Branch 1')).toBeInTheDocument();
      expect(screen.getByText('Feature Branch 2')).toBeInTheDocument();
    });
  });

  it('filters branches by search term', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const searchInput = screen.getByPlaceholderText('Search branches by name, description, or creator...');
    fireEvent.change(searchInput, { target: { value: 'Feature Branch 1' } });
    
    expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    expect(screen.queryByTestId('branch-card-2')).not.toBeInTheDocument();
  });

  it('filters branches by active status', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
      expect(screen.getByTestId('branch-card-2')).toBeInTheDocument();
    });
    
    const activeFilterButton = screen.getByText('Active Only');
    fireEvent.click(activeFilterButton);
    
    expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    expect(screen.queryByTestId('branch-card-2')).not.toBeInTheDocument();
  });

  it('filters branches by archived parent status', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
      expect(screen.getByTestId('branch-card-2')).toBeInTheDocument();
    });
    
    const archivedFilterButton = screen.getByText('Hide Archived Parents');
    fireEvent.click(archivedFilterButton);
    
    expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    expect(screen.queryByTestId('branch-card-2')).not.toBeInTheDocument();
  });

  it('refreshes branches when refresh button is clicked', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1);
    });
    
    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(2);
    });
  });

  it('calls onCreateBranch when create button is clicked', async () => {
    const onCreateBranch = vi.fn();
    renderWithApp(<BranchManagement asset={testAsset} onCreateBranch={onCreateBranch} />);
    
    const createButton = screen.getByText('Create Branch');
    fireEvent.click(createButton);
    
    expect(onCreateBranch).toHaveBeenCalled();
  });

  it('opens import modal when import version is clicked', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const card1 = screen.getByTestId('branch-card-1');
    const importButton = within(card1).getByText('Import Version');
    fireEvent.click(importButton);
    
    await waitFor(() => {
      expect(screen.getByTestId('import-modal')).toBeInTheDocument();
      expect(screen.getByText('Import to Feature Branch 1')).toBeInTheDocument();
    });
  });

  it('refreshes after successful import', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    // Open import modal
    const card1 = screen.getByTestId('branch-card-1');
    fireEvent.click(within(card1).getByText('Import Version'));
    
    // Click import in modal
    fireEvent.click(screen.getByText('Import'));
    
    await waitFor(() => {
      expect(mockBranchStore.fetchBranchVersions).toHaveBeenCalledWith('test-token', 1);
      expect(invoke).toHaveBeenCalledTimes(2); // Initial load + refresh
    });
  });

  it('opens branch history modal', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const card1 = screen.getByTestId('branch-card-1');
    fireEvent.click(within(card1).getByText('View History'));
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-history')).toBeInTheDocument();
      expect(screen.getByText('History for Feature Branch 1')).toBeInTheDocument();
    });
  });

  it('exports branch with versions', async () => {
    mockBranchStore.branchVersions = {
      1: [{
        version_id: 1,
        branch_id: 1,
        version_number: 'v1.0.0',
        branch_version_number: 'b1.1.0',
        file_name: 'config.json',
        file_size: 1024,
        created_at: '2024-01-01T00:00:00Z',
        author_username: 'testuser',
        notes: 'Test notes',
        is_branch_latest: true
      }]
    };
    
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const card1 = screen.getByTestId('branch-card-1');
    fireEvent.click(within(card1).getByText('Export'));
    
    await waitFor(() => {
      expect(screen.getByTestId('export-modal')).toBeInTheDocument();
    });
    
    fireEvent.click(screen.getByText('Export'));
    
    await waitFor(() => {
      expect(mockMessage.success).toHaveBeenCalledWith(
        'Branch version exported successfully to /path/to/export'
      );
    });
  });

  it('shows warning when exporting branch without versions', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-2')).toBeInTheDocument();
    });
    
    const card2 = screen.getByTestId('branch-card-2');
    fireEvent.click(within(card2).getByText('Export'));
    
    await waitFor(() => {
      expect(mockMessage.warning).toHaveBeenCalledWith(
        'No versions found for this branch to export'
      );
    });
  });

  it('opens promote to silver modal', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const card1 = screen.getByTestId('branch-card-1');
    fireEvent.click(within(card1).getByText('Promote to Silver'));
    
    await waitFor(() => {
      expect(screen.getByTestId('promote-modal')).toBeInTheDocument();
      expect(screen.getByText('Promote Feature Branch 1 to Silver')).toBeInTheDocument();
    });
  });

  it('shows success message after promotion', async () => {
    const onVersionHistoryChange = vi.fn();
    renderWithApp(
      <BranchManagement 
        asset={testAsset} 
        onVersionHistoryChange={onVersionHistoryChange} 
      />
    );
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    // Open promote modal
    const card1 = screen.getByTestId('branch-card-1');
    fireEvent.click(within(card1).getByText('Promote to Silver'));
    
    // Click promote in modal
    fireEvent.click(screen.getByText('Promote'));
    
    await waitFor(() => {
      expect(mockMessage.success).toHaveBeenCalledWith(
        'Branch promoted to Silver status successfully!'
      );
      expect(onVersionHistoryChange).toHaveBeenCalled();
    });
  });

  it('toggles between list and tree view', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const treeViewButton = screen.getByText('Tree View');
    fireEvent.click(treeViewButton);
    
    expect(screen.getByText('This view shows branches organized by their parent configuration version')).toBeInTheDocument();
    expect(screen.getByLabelText('Branch relationship tree')).toBeInTheDocument();
  });

  it('displays empty state when no branches exist', async () => {
    (invoke as any).mockResolvedValue([]);
    
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByText('No Branches Yet')).toBeInTheDocument();
      expect(screen.getByText(/This asset has no branches yet/)).toBeInTheDocument();
    });
  });

  it('displays filtered empty state when search returns no results', async () => {
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const searchInput = screen.getByPlaceholderText('Search branches by name, description, or creator...');
    fireEvent.change(searchInput, { target: { value: 'nonexistent' } });
    
    expect(screen.getByText('No Matching Branches')).toBeInTheDocument();
    expect(screen.getByText(/No branches match your current search/)).toBeInTheDocument();
  });

  it('handles API errors gracefully', async () => {
    (invoke as any).mockRejectedValue('Network error');
    
    renderWithApp(<BranchManagement asset={testAsset} />);
    
    await waitFor(() => {
      expect(mockMessage.error).toHaveBeenCalledWith('Failed to fetch branches: Network error');
    });
  });

  it('hides create button when showCreateButton is false', () => {
    renderWithApp(<BranchManagement asset={testAsset} showCreateButton={false} />);
    
    expect(screen.queryByText('Create Branch')).not.toBeInTheDocument();
  });

  it('hides select actions when showSelectActions is false', async () => {
    renderWithApp(<BranchManagement asset={testAsset} showSelectActions={false} />);
    
    await waitFor(() => {
      expect(screen.getByTestId('branch-card-1')).toBeInTheDocument();
    });
    
    const card1 = screen.getByTestId('branch-card-1');
    expect(within(card1).queryByText('Select')).not.toBeInTheDocument();
  });
});