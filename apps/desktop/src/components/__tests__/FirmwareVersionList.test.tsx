import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import FirmwareVersionList from '../FirmwareVersionList';
import { FirmwareVersionInfo } from '../../types/firmware';
import useAuthStore from '../../store/auth';
import useFirmwareStore from '../../store/firmware';

// Mock the stores
vi.mock('../../store/auth');
vi.mock('../../store/firmware');

// Mock Ant Design components
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  
  // Create Modal mock
  const ModalMock = vi.fn(({ children, ...props }) => {
    return props.open ? <div data-testid="modal" {...props}>{children}</div> : null;
  });
  
  // Add static methods
  ModalMock.confirm = vi.fn(({ onOk }) => {
    // Immediately call onOk for testing
    if (onOk) onOk();
  });
  ModalMock.info = vi.fn();
  ModalMock.success = vi.fn();
  ModalMock.error = vi.fn();
  ModalMock.warning = vi.fn();
  
  return {
    ...actual,
    Modal: ModalMock,
    message: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn()
    }
  };
});

describe('FirmwareVersionList', () => {
  const mockFirmwareVersions: FirmwareVersionInfo[] = [
    {
      id: 1,
      asset_id: 1,
      author_id: 1,
      author_username: 'engineer',
      vendor: 'Siemens',
      model: 'S7-1500',
      version: '2.8.1',
      notes: 'Stable firmware version',
      status: 'Golden',
      file_path: '/path/to/firmware1.enc',
      file_hash: 'abc123def456789012345678901234567890123456789012345678901234567890',
      file_size: 1048576, // 1MB
      created_at: '2023-01-01T12:00:00Z'
    },
    {
      id: 2,
      asset_id: 1,
      author_id: 2,
      author_username: 'admin',
      vendor: null,
      model: null,
      version: '1.0.0',
      notes: null,
      status: 'Draft',
      file_path: '/path/to/firmware2.enc',
      file_hash: 'def456789012345678901234567890123456789012345678901234567890abcdef',
      file_size: 2097152, // 2MB
      created_at: '2023-01-02T12:00:00Z'
    }
  ];

  const mockAuthStore = {
    user: { id: 1, username: 'engineer', role: 'Engineer' as const, created_at: '2023-01-01', is_active: true }
  };

  const mockFirmwareStore = {
    deleteFirmware: vi.fn()
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as any).mockReturnValue(mockAuthStore);
    (useFirmwareStore as any).mockReturnValue(mockFirmwareStore);
  });

  it('renders firmware versions correctly', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    // Check first firmware
    expect(screen.getByText('Siemens - S7-1500')).toBeInTheDocument();
    expect(screen.getByText('2.8.1')).toBeInTheDocument();
    expect(screen.getByText('Stable firmware version')).toBeInTheDocument();
    expect(screen.getByText('engineer')).toBeInTheDocument();
    
    // Check second firmware
    expect(screen.getByText('Generic')).toBeInTheDocument();
    expect(screen.getByText('1.0.0')).toBeInTheDocument();
    expect(screen.getByText('admin')).toBeInTheDocument();
  });

  it('formats file size correctly', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    expect(screen.getByText('1 MB')).toBeInTheDocument();
    expect(screen.getByText('2 MB')).toBeInTheDocument();
  });

  it('shows truncated file hash', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    expect(screen.getByText('abc123de...')).toBeInTheDocument();
    expect(screen.getByText('def45678...')).toBeInTheDocument();
  });

  it('displays status badges correctly', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    expect(screen.getByText('Golden')).toBeInTheDocument();
    expect(screen.getByText('Draft')).toBeInTheDocument();
  });

  it('shows encrypted indicator', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    const encryptedTexts = screen.getAllByText('Encrypted');
    expect(encryptedTexts).toHaveLength(2);
  });

  it('shows empty state when no versions', () => {
    render(<FirmwareVersionList versions={[]} />);
    
    expect(screen.getByText('No firmware versions available')).toBeInTheDocument();
  });

  it('sorts versions by created date (newest first)', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    const versionTexts = screen.getAllByText(/^\d+\.\d+\.\d+$/);
    expect(versionTexts[0]).toHaveTextContent('1.0.0'); // Newer one first
    expect(versionTexts[1]).toHaveTextContent('2.8.1');
  });

  it('shows delete option for engineer role', () => {
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    const moreButtons = screen.getAllByRole('button');
    expect(moreButtons.length).toBeGreaterThan(0);
  });

  it('hides delete option for non-engineer role', () => {
    (useAuthStore as any).mockReturnValue({
      ...mockAuthStore,
      user: { ...mockAuthStore.user, role: 'Administrator' }
    });
    
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    // Should not show the more options buttons (with MoreOutlined icon)
    const moreButtons = screen.queryAllByLabelText('more');
    expect(moreButtons.length).toBe(0);
  });

  it('formats relative time correctly', () => {
    const recentVersion = {
      ...mockFirmwareVersions[0],
      created_at: new Date(Date.now() - 5 * 60 * 1000).toISOString() // 5 minutes ago
    };
    
    render(<FirmwareVersionList versions={[recentVersion]} />);
    
    expect(screen.getByText('5 minutes ago')).toBeInTheDocument();
  });

  it('handles firmware without vendor/model', () => {
    const genericVersion = {
      ...mockFirmwareVersions[0],
      vendor: null,
      model: null
    };
    
    render(<FirmwareVersionList versions={[genericVersion]} />);
    
    expect(screen.getByText('Generic')).toBeInTheDocument();
  });

  it('does not show notes section when notes are null', () => {
    render(<FirmwareVersionList versions={[mockFirmwareVersions[1]]} />);
    
    expect(screen.queryByText('Stable firmware version')).not.toBeInTheDocument();
  });

  describe('Delete functionality', () => {
    it('shows delete confirmation modal when delete clicked', async () => {
      const onDelete = vi.fn();
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      // Click the first more button
      const moreButtons = screen.getAllByRole('button');
      fireEvent.click(moreButtons[0]);
      
      // Click delete in dropdown
      const deleteOption = await screen.findByText('Delete');
      fireEvent.click(deleteOption);
      
      // Verify modal content
      expect(screen.getByText('Delete Firmware?')).toBeInTheDocument();
      expect(screen.getByText('Are you sure you want to delete this firmware version?')).toBeInTheDocument();
      expect(screen.getByText('2.8.1')).toBeInTheDocument();
      expect(screen.getByText('Siemens')).toBeInTheDocument();
      expect(screen.getByText('S7-1500')).toBeInTheDocument();
    });

    it('calls deleteFirmware when confirmed', async () => {
      const onDelete = vi.fn();
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      // Click more button and delete
      const moreButtons = screen.getAllByRole('button');
      fireEvent.click(moreButtons[0]);
      const deleteOption = await screen.findByText('Delete');
      fireEvent.click(deleteOption);
      
      // Confirm deletion
      expect(mockFirmwareStore.deleteFirmware).toHaveBeenCalledWith(1);
    });

    it('shows loading state during deletion', async () => {
      const onDelete = vi.fn();
      
      // Mock deleteFirmware to simulate delay
      mockFirmwareStore.deleteFirmware.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));
      
      const { container } = render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      // Trigger delete
      const moreButtons = screen.getAllByRole('button');
      fireEvent.click(moreButtons[0]);
      const deleteOption = await screen.findByText('Delete');
      fireEvent.click(deleteOption);
      
      // Check for loading state on the card
      const firstCard = container.querySelector('.ant-card-loading');
      expect(firstCard).toBeInTheDocument();
    });

    it('calls onDelete callback after successful deletion', async () => {
      const onDelete = vi.fn();
      mockFirmwareStore.deleteFirmware.mockResolvedValue(undefined);
      
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      // Click more button and delete
      const moreButtons = screen.getAllByRole('button');
      fireEvent.click(moreButtons[0]);
      const deleteOption = await screen.findByText('Delete');
      fireEvent.click(deleteOption);
      
      // Wait for deletion to complete
      await waitFor(() => {
        expect(onDelete).toHaveBeenCalled();
      });
    });

    it('shows error message on deletion failure', async () => {
      const onDelete = vi.fn();
      const errorMessage = 'Failed to delete firmware';
      mockFirmwareStore.deleteFirmware.mockRejectedValue(new Error(errorMessage));
      
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      // Click more button and delete
      const moreButtons = screen.getAllByRole('button');
      fireEvent.click(moreButtons[0]);
      const deleteOption = await screen.findByText('Delete');
      fireEvent.click(deleteOption);
      
      // Should not call onDelete on failure
      await waitFor(() => {
        expect(onDelete).not.toHaveBeenCalled();
      });
    });

    it('does not show delete option for non-engineer users', () => {
      (useAuthStore as any).mockReturnValue({
        ...mockAuthStore,
        user: { ...mockAuthStore.user, role: 'Administrator' }
      });
      
      render(<FirmwareVersionList versions={mockFirmwareVersions} />);
      
      // Should not have any dropdown buttons
      const buttons = screen.queryAllByRole('button');
      expect(buttons.length).toBe(0);
    });
  });
});