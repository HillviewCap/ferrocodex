import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Modal, message } from 'antd';
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
      assetId: 1,
      authorId: 1,
      authorUsername: 'engineer',
      vendor: 'Siemens',
      model: 'S7-1500',
      version: '2.8.1',
      notes: 'Stable firmware version',
      status: 'Golden',
      filePath: '/path/to/firmware1.enc',
      fileHash: 'abc123def456789012345678901234567890123456789012345678901234567890',
      fileSize: 1048576, // 1MB
      createdAt: '2023-01-01T12:00:00Z'
    },
    {
      id: 2,
      assetId: 1,
      authorId: 2,
      authorUsername: 'admin',
      vendor: null,
      model: null,
      version: '1.0.0',
      notes: null,
      status: 'Draft',
      filePath: '/path/to/firmware2.enc',
      fileHash: 'def456789012345678901234567890123456789012345678901234567890abcdef',
      fileSize: 2097152, // 2MB
      createdAt: '2023-01-02T12:00:00Z'
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

  // Opens the actions dropdown on the card whose title matches cardTitle
  const openActionsMenu = (cardTitle: string) => {
    const card = screen.getByText(cardTitle).closest('.ant-card') as HTMLElement;
    fireEvent.click(within(card).getByRole('button', { name: /more/i }));
  };

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

  it('hides delete option for non-engineer role', async () => {
    (useAuthStore as any).mockReturnValue({
      ...mockAuthStore,
      user: { ...mockAuthStore.user, role: 'Administrator' }
    });
    
    render(<FirmwareVersionList versions={mockFirmwareVersions} />);
    
    // The actions menu is available to everyone, but without Delete
    openActionsMenu('Siemens - S7-1500');
    expect(await screen.findByText('View Analysis')).toBeInTheDocument();
    expect(screen.queryByText('Delete')).not.toBeInTheDocument();
  });

  it('formats relative time correctly', () => {
    const recentVersion = {
      ...mockFirmwareVersions[0],
      createdAt: new Date(Date.now() - 5 * 60 * 1000).toISOString() // 5 minutes ago
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
      
      openActionsMenu('Siemens - S7-1500');
      fireEvent.click(await screen.findByText('Delete'));
      
      // Modal.confirm is mocked, so render the content it was given
      expect(Modal.confirm).toHaveBeenCalledWith(
        expect.objectContaining({ title: 'Delete Firmware?', okText: 'Delete' })
      );
      const { content } = vi.mocked(Modal.confirm).mock.calls[0][0];
      const modal = within(render(<>{content}</>).container);
      expect(modal.getByText('Are you sure you want to delete this firmware version?')).toBeInTheDocument();
      expect(modal.getByText('2.8.1')).toBeInTheDocument();
      expect(modal.getByText('Siemens')).toBeInTheDocument();
      expect(modal.getByText('S7-1500')).toBeInTheDocument();
    });

    it('calls deleteFirmware when confirmed', async () => {
      const onDelete = vi.fn();
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      openActionsMenu('Siemens - S7-1500');
      fireEvent.click(await screen.findByText('Delete'));
      
      // Confirm deletion
      expect(mockFirmwareStore.deleteFirmware).toHaveBeenCalledWith(1);
    });

    it('shows loading state during deletion', async () => {
      const onDelete = vi.fn();
      
      // Mock deleteFirmware to simulate delay
      mockFirmwareStore.deleteFirmware.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));
      
      const { container } = render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      openActionsMenu('Siemens - S7-1500');
      fireEvent.click(await screen.findByText('Delete'));
      
      // Check for loading state on the card
      const firstCard = container.querySelector('.ant-card-loading');
      expect(firstCard).toBeInTheDocument();
    });

    it('calls onDelete callback after successful deletion', async () => {
      const onDelete = vi.fn();
      mockFirmwareStore.deleteFirmware.mockResolvedValue(undefined);
      
      render(<FirmwareVersionList versions={mockFirmwareVersions} onDelete={onDelete} />);
      
      openActionsMenu('Siemens - S7-1500');
      fireEvent.click(await screen.findByText('Delete'));
      
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
      
      openActionsMenu('Siemens - S7-1500');
      fireEvent.click(await screen.findByText('Delete'));
      
      await waitFor(() => {
        expect(message.error).toHaveBeenCalledWith(`Failed to delete firmware: ${errorMessage}`);
      });
      // Should not call onDelete on failure
      expect(onDelete).not.toHaveBeenCalled();
    });

    it('does not show delete option for non-engineer users', async () => {
      (useAuthStore as any).mockReturnValue({
        ...mockAuthStore,
        user: { ...mockAuthStore.user, role: 'Administrator' }
      });
      
      render(<FirmwareVersionList versions={mockFirmwareVersions} />);
      
      openActionsMenu('Generic');
      expect(await screen.findByText('View History')).toBeInTheDocument();
      expect(screen.queryByText('Delete')).not.toBeInTheDocument();
      expect(mockFirmwareStore.deleteFirmware).not.toHaveBeenCalled();
    });
  });
});