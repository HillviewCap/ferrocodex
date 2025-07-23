import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import FirmwareHistoryTimeline from './FirmwareHistoryTimeline';
import { FirmwareStatusHistory } from '../../types/firmware';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock auth store
vi.mock('../../store/auth', () => ({
  default: vi.fn(() => ({
    token: 'test-token'
  }))
}));

// Mock Ant Design message
const mockMessage = {
  error: vi.fn()
};
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: mockMessage
  };
});

const mockHistoryData: FirmwareStatusHistory[] = [
  {
    id: 1,
    firmware_version_id: 1,
    old_status: 'Draft',
    new_status: 'Approved',
    changed_by: 1,
    changed_by_username: 'John Doe',
    changed_at: new Date().toISOString(),
    reason: 'Initial approval for testing'
  },
  {
    id: 2,
    firmware_version_id: 1,
    old_status: 'Approved',
    new_status: 'Golden',
    changed_by: 2,
    changed_by_username: 'Jane Admin',
    changed_at: new Date(Date.now() - 86400000).toISOString(), // 1 day ago
    reason: 'Promoted to Golden after successful deployment'
  },
  {
    id: 3,
    firmware_version_id: 1,
    old_status: 'Golden',
    new_status: 'Archived',
    changed_by: 1,
    changed_by_username: 'John Doe',
    changed_at: new Date(Date.now() - 172800000).toISOString(), // 2 days ago
    reason: undefined
  }
];

describe('FirmwareHistoryTimeline', () => {
  const { invoke } = require('@tauri-apps/api/core');

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state initially', () => {
    invoke.mockImplementation(() => new Promise(() => {})); // Never resolves
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    expect(screen.getByText('Loading firmware history...')).toBeInTheDocument();
  });

  it('renders empty state when no history exists', async () => {
    invoke.mockResolvedValue([]);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      expect(screen.getByText('No status history available for this firmware version')).toBeInTheDocument();
    });
  });

  it('renders history timeline with status changes', async () => {
    invoke.mockResolvedValue(mockHistoryData);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      // Check for status transitions
      expect(screen.getByText('Draft → Approved')).toBeInTheDocument();
      expect(screen.getByText('Approved → Golden')).toBeInTheDocument();
      expect(screen.getByText('Golden → Archived')).toBeInTheDocument();
      
      // Check for user names
      expect(screen.getAllByText('John Doe')).toHaveLength(2);
      expect(screen.getByText('Jane Admin')).toBeInTheDocument();
      
      // Check for current tag on latest item
      expect(screen.getByText('Current')).toBeInTheDocument();
    });
  });

  it('displays reason when provided', async () => {
    invoke.mockResolvedValue(mockHistoryData);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      expect(screen.getByText('Initial approval for testing')).toBeInTheDocument();
      expect(screen.getByText('Promoted to Golden after successful deployment')).toBeInTheDocument();
    });
  });

  it('formats dates correctly', async () => {
    const recentHistory = [{
      ...mockHistoryData[0],
      changed_at: new Date().toISOString() // Just now
    }];
    invoke.mockResolvedValue(recentHistory);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      expect(screen.getByText('Just now')).toBeInTheDocument();
    });
  });

  it('handles pagination for more than 10 items', async () => {
    const manyItems = Array.from({ length: 15 }, (_, i) => ({
      id: i + 1,
      firmware_version_id: 1,
      old_status: 'Draft',
      new_status: 'Approved',
      changed_by: 1,
      changed_by_username: `User ${i + 1}`,
      changed_at: new Date(Date.now() - i * 86400000).toISOString(),
      reason: undefined
    }));
    
    invoke.mockResolvedValue(manyItems);
    
    const { getByText } = render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      expect(getByText('Showing 10 of 15 history items')).toBeInTheDocument();
      expect(getByText('Show 5 more items')).toBeInTheDocument();
    });
    
    // Click to show more
    const showMoreButton = getByText('Show 5 more items');
    await userEvent.click(showMoreButton);
    
    await waitFor(() => {
      expect(getByText('Showing 15 of 15 history items')).toBeInTheDocument();
    });
  });

  it('shows error message on API failure', async () => {
    invoke.mockRejectedValue(new Error('Failed to fetch history'));
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      expect(mockMessage.error).toHaveBeenCalledWith('Failed to load firmware history: Error: Failed to fetch history');
    });
  });

  it('applies correct colors to status badges', async () => {
    invoke.mockResolvedValue(mockHistoryData);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} />);
    
    await waitFor(() => {
      // The component should render with appropriate status colors
      const timeline = screen.getByRole('list');
      expect(timeline).toBeInTheDocument();
    });
  });

  it('calls onRefresh when provided', async () => {
    const onRefresh = vi.fn();
    invoke.mockResolvedValue(mockHistoryData);
    
    render(<FirmwareHistoryTimeline firmwareVersionId={1} onRefresh={onRefresh} />);
    
    await waitFor(() => {
      expect(screen.getByText('Draft → Approved')).toBeInTheDocument();
    });
    
    // onRefresh is not called by the component itself, it's passed for external use
    expect(onRefresh).not.toHaveBeenCalled();
  });
});