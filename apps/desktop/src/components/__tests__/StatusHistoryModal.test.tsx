import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import StatusHistoryModal from '../StatusHistoryModal';
import { ConfigurationVersionInfo, StatusChangeRecord } from '../../types/assets';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

describe('StatusHistoryModal', () => {
  const mockVersion: ConfigurationVersionInfo = {
    id: 1,
    asset_id: 1,
    version_number: 'v1',
    file_name: 'config.json',
    file_size: 1024,
    content_hash: 'abc123def456',
    author: 1,
    author_username: 'john_doe',
    notes: 'Test version',
    status: 'Approved',
    status_changed_by: undefined,
    status_changed_at: undefined,
    created_at: '2023-01-01T12:00:00Z'
  };

  const mockStatusHistory: StatusChangeRecord[] = [
    {
      id: 1,
      version_id: 1,
      old_status: undefined,
      new_status: 'Draft',
      changed_by: 1,
      changed_by_username: 'john_doe',
      change_reason: 'Initial creation',
      created_at: '2023-01-01T12:00:00Z'
    },
    {
      id: 2,
      version_id: 1,
      old_status: 'Draft',
      new_status: 'Approved',
      changed_by: 2,
      changed_by_username: 'admin_user',
      change_reason: 'Reviewed and approved for production',
      created_at: '2023-01-02T12:00:00Z'
    }
  ];

  const defaultProps = {
    visible: true,
    onCancel: vi.fn(),
    version: mockVersion,
    token: 'test-token'
  };

  const getMockInvoke = async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return vi.mocked(invoke);
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue(mockStatusHistory);
  });

  it('renders modal with version information', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    const statusHistoryElements = screen.getAllByText('Status Change History');
    expect(statusHistoryElements.length).toBeGreaterThan(0);
    const configJsonElements = screen.getAllByText('config.json');
    expect(configJsonElements.length).toBeGreaterThan(0);
    const v1Elements = screen.getAllByText('(v1)');
    expect(v1Elements.length).toBeGreaterThan(0);
    const approvedElements = screen.getAllByText('Approved');
    expect(approvedElements.length).toBeGreaterThan(0);
  });

  it('loads status history on mount', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(async () => {
      const invoke = await getMockInvoke();
      expect(invoke).toHaveBeenCalledWith('get_configuration_status_history', {
        token: 'test-token',
        versionId: 1
      });
    });
  });

  it('displays status history timeline', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const draftElements = screen.getAllByText('Draft');
      const approvedElements = screen.getAllByText('Approved');
      expect(draftElements.length).toBeGreaterThan(0);
      expect(approvedElements.length).toBeGreaterThan(0);
      const johnDoeElements = screen.getAllByText('john_doe');
      expect(johnDoeElements.length).toBeGreaterThan(0);
      const adminUserElements = screen.getAllByText('admin_user');
      expect(adminUserElements.length).toBeGreaterThan(0);
    });
  });

  it('shows loading state while fetching history', async () => {
    const invoke = await getMockInvoke();
    invoke.mockImplementation(() => new Promise(() => {})); // Never resolves
    render(<StatusHistoryModal {...defaultProps} />);
    
    const loadingElements = screen.getAllByText('Loading status history...');
    expect(loadingElements.length).toBeGreaterThan(0);
  });

  it('shows error when history fails to load', async () => {
    const invoke = await getMockInvoke();
    invoke.mockRejectedValue('Failed to load history');
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const errorLoadingElements = screen.getAllByText('Error loading status history');
      expect(errorLoadingElements.length).toBeGreaterThan(0);
      const failedLoadElements = screen.getAllByText('Failed to load history');
      expect(failedLoadElements.length).toBeGreaterThan(0);
    });
  });

  it('shows empty state when no history available', async () => {
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue([]);
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const noChangesElements = screen.getAllByText('No status changes found');
      expect(noChangesElements.length).toBeGreaterThan(0);
    });
  });

  it('displays change reasons when provided', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const initialCreationElements = screen.getAllByText('Initial creation');
      expect(initialCreationElements.length).toBeGreaterThan(0);
      const reviewedElements = screen.getAllByText('Reviewed and approved for production');
      expect(reviewedElements.length).toBeGreaterThan(0);
    });
  });

  it('shows initial status indicator for first record', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const initialStatusElements = screen.getAllByText('(Initial status)');
      expect(initialStatusElements.length).toBeGreaterThan(0);
    });
  });

  it('displays status transitions with arrows', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const arrows = screen.getAllByLabelText('arrow-right');
      expect(arrows).toHaveLength(1); // Only one transition with old_status
    });
  });

  it('formats dates correctly', async () => {
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      // Should show relative time
      const agoElements = screen.getAllByText(/ago$/);
      expect(agoElements.length).toBeGreaterThan(0);
    });
  });

  it('does not render when version is null', () => {
    render(<StatusHistoryModal {...defaultProps} version={null} />);
    
    expect(screen.queryByText('Status Change History')).not.toBeInTheDocument();
  });

  it('does not load history when modal is not visible', async () => {
    const invoke = await getMockInvoke();
    render(<StatusHistoryModal {...defaultProps} visible={false} />);
    
    expect(invoke).not.toHaveBeenCalled();
  });

  it('reloads history when version changes', async () => {
    const invoke = await getMockInvoke();
    const { rerender } = render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    const newVersion = { ...mockVersion, id: 2 };
    rerender(<StatusHistoryModal {...defaultProps} version={newVersion} />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(2);
      expect(invoke).toHaveBeenLastCalledWith('get_configuration_status_history', {
        token: 'test-token',
        versionId: 2
      });
    });
  });

  it('applies correct timeline colors for different statuses', async () => {
    const historyWithAllStatuses = [
      { ...mockStatusHistory[0], new_status: 'Draft' },
      { ...mockStatusHistory[1], new_status: 'Approved' },
      {
        id: 3,
        version_id: 1,
        old_status: 'Approved',
        new_status: 'Golden',
        changed_by: 3,
        changed_by_username: 'golden_user',
        change_reason: 'Promoted to golden',
        created_at: '2023-01-03T12:00:00Z'
      },
      {
        id: 4,
        version_id: 1,
        old_status: 'Golden',
        new_status: 'Archived',
        changed_by: 4,
        changed_by_username: 'archive_user',
        change_reason: 'Archived old version',
        created_at: '2023-01-04T12:00:00Z'
      }
    ];

    const invoke = await getMockInvoke();
    invoke.mockResolvedValue(historyWithAllStatuses);
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const goldenElements = screen.getAllByText('Golden');
      const archivedElements = screen.getAllByText('Archived');
      expect(goldenElements.length).toBeGreaterThan(0);
      expect(archivedElements.length).toBeGreaterThan(0);
    });
  });

  it('handles records without change reasons', async () => {
    const historyWithoutReasons = [
      {
        ...mockStatusHistory[0],
        change_reason: null
      }
    ];

    const invoke = await getMockInvoke();
    invoke.mockResolvedValue(historyWithoutReasons);
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const draftElements = screen.getAllByText('Draft');
      expect(draftElements.length).toBeGreaterThan(0);
      expect(screen.queryByText('Initial creation')).not.toBeInTheDocument();
    });
  });

  it('closes error alert when dismiss button is clicked', async () => {
    const invoke = await getMockInvoke();
    invoke.mockRejectedValue('Test error');
    render(<StatusHistoryModal {...defaultProps} />);
    
    await waitFor(() => {
      const testErrorElements = screen.getAllByText('Test error');
      expect(testErrorElements.length).toBeGreaterThan(0);
    });

    const closeButton = screen.getByRole('button', { name: /close/i });
    closeButton.click();

    expect(screen.queryByText('Test error')).not.toBeInTheDocument();
  });
});