import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import FirmwareManagement from '../FirmwareManagement';
import { AssetInfo } from '../../types/assets';
import useAuthStore from '../../store/auth';
import useFirmwareStore from '../../store/firmware';

// Mock the stores
vi.mock('../../store/auth');
vi.mock('../../store/firmware');

// Mock the components
vi.mock('../FirmwareUploadModal', () => ({
  default: vi.fn(({ visible, onCancel, onSuccess }) => 
    visible ? (
      <div data-testid="firmware-upload-modal">
        <button onClick={onCancel}>Cancel</button>
        <button onClick={onSuccess}>Upload</button>
      </div>
    ) : null
  )
}));

vi.mock('../FirmwareVersionList', () => ({
  default: vi.fn(({ versions }) => (
    <div data-testid="firmware-version-list">
      {versions.map((v: any) => (
        <div key={v.id}>{v.version}</div>
      ))}
    </div>
  ))
}));

describe('FirmwareManagement', () => {
  const mockAsset: AssetInfo = {
    id: 1,
    name: 'Test Asset',
    description: 'Test Description',
    created_by: 1,
    created_by_username: 'admin',
    created_at: '2023-01-01T00:00:00Z',
    version_count: 5,
    latest_version: 'v5',
    latest_version_notes: 'Latest version'
  };

  const mockFirmwareVersions = [
    {
      id: 1,
      asset_id: 1,
      author_id: 1,
      author_username: 'engineer',
      vendor: 'Siemens',
      model: 'S7-1500',
      version: '2.8.1',
      notes: 'Stable firmware',
      status: 'Golden' as const,
      file_path: '/path/to/firmware1.enc',
      file_hash: 'abc123def456789',
      file_size: 1048576,
      created_at: '2023-01-01T00:00:00Z'
    }
  ];

  const mockAuthStore = {
    user: { id: 1, username: 'engineer', role: 'Engineer' as const, created_at: '2023-01-01', is_active: true }
  };

  const mockFirmwareStore = {
    firmwareVersions: { 1: mockFirmwareVersions },
    isLoading: false,
    error: null,
    loadFirmwareVersions: vi.fn().mockResolvedValue(undefined),
    clearError: vi.fn()
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as any).mockReturnValue(mockAuthStore);
    (useFirmwareStore as any).mockReturnValue(mockFirmwareStore);
  });

  it('renders firmware management component', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.getByText('Manage firmware files for crash recovery and maintenance procedures')).toBeInTheDocument();
  });

  it('shows upload button for engineer role', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.getByRole('button', { name: /upload firmware/i })).toBeInTheDocument();
  });

  it('hides upload button for non-engineer role', () => {
    (useAuthStore as any).mockReturnValue({
      ...mockAuthStore,
      user: { ...mockAuthStore.user, role: 'Administrator' }
    });
    
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.queryByRole('button', { name: /upload firmware/i })).not.toBeInTheDocument();
  });

  it('loads firmware versions on mount', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(mockFirmwareStore.loadFirmwareVersions).toHaveBeenCalledWith(1);
  });

  it('displays firmware versions when available', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.getByTestId('firmware-version-list')).toBeInTheDocument();
    expect(screen.getByText('2.8.1')).toBeInTheDocument();
  });

  it('shows empty state when no firmware versions', () => {
    (useFirmwareStore as any).mockReturnValue({
      ...mockFirmwareStore,
      firmwareVersions: { 1: [] }
    });
    
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.getByText('No Firmware Files')).toBeInTheDocument();
    expect(screen.getByText('Upload firmware files to enable complete system recovery')).toBeInTheDocument();
  });

  it('shows loading state', () => {
    (useFirmwareStore as any).mockReturnValue({
      ...mockFirmwareStore,
      isLoading: true
    });
    
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(screen.getByText('Loading firmware versions...')).toBeInTheDocument();
  });

  it('opens upload modal when upload button clicked', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    const uploadButton = screen.getByRole('button', { name: /upload firmware/i });
    fireEvent.click(uploadButton);
    
    expect(screen.getByTestId('firmware-upload-modal')).toBeInTheDocument();
  });

  it('closes upload modal on cancel', () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    // Open modal
    fireEvent.click(screen.getByRole('button', { name: /upload firmware/i }));
    expect(screen.getByTestId('firmware-upload-modal')).toBeInTheDocument();
    
    // Cancel modal
    fireEvent.click(screen.getByText('Cancel'));
    expect(screen.queryByTestId('firmware-upload-modal')).not.toBeInTheDocument();
  });

  it('handles upload success', async () => {
    render(<FirmwareManagement asset={mockAsset} />);
    
    // Open modal
    fireEvent.click(screen.getByRole('button', { name: /upload firmware/i }));
    
    // Trigger success
    fireEvent.click(screen.getByText('Upload'));
    
    await waitFor(() => {
      expect(screen.queryByTestId('firmware-upload-modal')).not.toBeInTheDocument();
    });
  });

  it('displays error message when error occurs', () => {
    const errorMessage = 'Failed to load firmware';
    (useFirmwareStore as any).mockReturnValue({
      ...mockFirmwareStore,
      error: errorMessage
    });
    
    render(<FirmwareManagement asset={mockAsset} />);
    
    expect(mockFirmwareStore.clearError).toHaveBeenCalled();
  });
});