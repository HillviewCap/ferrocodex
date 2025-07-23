import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import FirmwareManagement from '../FirmwareManagement';
import { AssetInfo } from '../../types/assets';
import useAuthStore from '../../store/auth';
import useFirmwareStore from '../../store/firmware';

// Mock the stores
vi.mock('../../store/auth');
vi.mock('../../store/firmware');

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn()
}));

// Import mocked modules
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';

// Mock child components to simplify testing
vi.mock('../FirmwareUploadModal', () => ({
  default: vi.fn(({ visible, onCancel, onSuccess, assetId }) => {
    if (!visible) return null;
    
    // Simulate successful upload flow
    const handleUpload = async () => {
      const mockStore = (useFirmwareStore as any)();
      await mockStore.uploadFirmware({
        asset_id: assetId,
        vendor: 'Test Vendor',
        model: 'Test Model',
        version: '1.0.0',
        notes: 'Test firmware',
        file_path: '/test/firmware.bin'
      });
      onSuccess();
    };
    
    return (
      <div data-testid="firmware-upload-modal">
        <button onClick={handleUpload}>Simulate Upload</button>
        <button onClick={onCancel}>Cancel</button>
      </div>
    );
  })
}));

vi.mock('../FirmwareVersionList', () => ({
  default: vi.fn(({ versions, onDelete }) => {
    if (versions.length === 0) {
      return <div>No firmware versions available</div>;
    }
    
    return (
      <div data-testid="firmware-version-list">
        {versions.map((v: any) => (
          <div key={v.id} data-testid={`firmware-${v.id}`}>
            <span>{v.version}</span>
            <button onClick={() => {
              const mockStore = (useFirmwareStore as any)();
              mockStore.deleteFirmware(v.id).then(() => onDelete());
            }}>Delete</button>
          </div>
        ))}
      </div>
    );
  })
}));

describe('Firmware Management Integration Tests', () => {
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

  const mockFirmwareVersion = {
    id: 1,
    asset_id: 1,
    author_id: 1,
    author_username: 'engineer',
    vendor: 'Test Vendor',
    model: 'Test Model',
    version: '1.0.0',
    notes: 'Test firmware',
    status: 'Draft' as const,
    file_path: '/encrypted/firmware/1/1.enc',
    file_hash: 'abc123def456789',
    file_size: 1048576,
    created_at: '2023-01-01T00:00:00Z'
  };

  const mockAuthStore = {
    user: { 
      id: 1, 
      username: 'engineer', 
      role: 'Engineer' as const, 
      created_at: '2023-01-01', 
      is_active: true 
    }
  };

  let mockFirmwareStore: any;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Initialize mock firmware store
    mockFirmwareStore = {
      firmwareVersions: { 1: [] },
      isLoading: false,
      error: null,
      loadFirmwareVersions: vi.fn().mockResolvedValue(undefined),
      uploadFirmware: vi.fn().mockResolvedValue(undefined),
      deleteFirmware: vi.fn().mockResolvedValue(undefined),
      clearError: vi.fn()
    };
    
    (useAuthStore as any).mockReturnValue(mockAuthStore);
    (useFirmwareStore as any).mockReturnValue(mockFirmwareStore);
  });

  describe('Complete Upload → List → Delete Workflow', () => {
    it('should successfully upload firmware, list it, and delete it', async () => {
      // Start with no firmware
      render(<FirmwareManagement asset={mockAsset} />);
      
      // Verify empty state
      expect(screen.getByText('No Firmware Files')).toBeInTheDocument();
      expect(screen.getByText('Upload firmware files to enable complete system recovery')).toBeInTheDocument();
      
      // Step 1: Upload firmware
      const uploadButton = screen.getByRole('button', { name: /upload firmware/i });
      fireEvent.click(uploadButton);
      
      // Verify upload modal appears
      expect(screen.getByTestId('firmware-upload-modal')).toBeInTheDocument();
      
      // Simulate successful upload
      mockFirmwareStore.uploadFirmware.mockImplementation(async () => {
        // Add the new firmware to the store
        mockFirmwareStore.firmwareVersions[1] = [mockFirmwareVersion];
      });
      
      fireEvent.click(screen.getByText('Simulate Upload'));
      
      await waitFor(() => {
        expect(mockFirmwareStore.uploadFirmware).toHaveBeenCalledWith({
          asset_id: 1,
          vendor: 'Test Vendor',
          model: 'Test Model',
          version: '1.0.0',
          notes: 'Test firmware',
          file_path: '/test/firmware.bin'
        });
      });
      
      // Step 2: Verify firmware appears in list
      await waitFor(() => {
        expect(mockFirmwareStore.loadFirmwareVersions).toHaveBeenCalledWith(1);
      });
      
      // Re-render with the updated firmware list
      mockFirmwareStore.firmwareVersions[1] = [mockFirmwareVersion];
      const { rerender } = render(<FirmwareManagement asset={mockAsset} />);
      
      // Verify firmware is displayed
      expect(screen.getByTestId('firmware-version-list')).toBeInTheDocument();
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
      
      // Step 3: Delete the firmware
      mockFirmwareStore.deleteFirmware.mockImplementation(async (id: number) => {
        // Remove the firmware from the store
        mockFirmwareStore.firmwareVersions[1] = mockFirmwareStore.firmwareVersions[1].filter(
          (f: any) => f.id !== id
        );
      });
      
      const deleteButton = screen.getByText('Delete');
      fireEvent.click(deleteButton);
      
      await waitFor(() => {
        expect(mockFirmwareStore.deleteFirmware).toHaveBeenCalledWith(1);
      });
      
      // Step 4: Verify firmware is removed and empty state returns
      rerender(<FirmwareManagement asset={mockAsset} />);
      
      await waitFor(() => {
        expect(screen.getByText('No Firmware Files')).toBeInTheDocument();
      });
    });

    it('should handle errors during upload workflow', async () => {
      render(<FirmwareManagement asset={mockAsset} />);
      
      // Click upload
      fireEvent.click(screen.getByRole('button', { name: /upload firmware/i }));
      
      // Simulate upload failure
      mockFirmwareStore.uploadFirmware.mockRejectedValue(new Error('Upload failed'));
      
      fireEvent.click(screen.getByText('Simulate Upload'));
      
      await waitFor(() => {
        expect(mockFirmwareStore.uploadFirmware).toHaveBeenCalled();
      });
      
      // Verify we're still in empty state (upload failed)
      expect(screen.getByText('No Firmware Files')).toBeInTheDocument();
    });

    it('should handle multiple firmware versions', async () => {
      const multipleFirmware = [
        { ...mockFirmwareVersion, id: 1, version: '1.0.0' },
        { ...mockFirmwareVersion, id: 2, version: '2.0.0' },
        { ...mockFirmwareVersion, id: 3, version: '3.0.0' }
      ];
      
      mockFirmwareStore.firmwareVersions[1] = multipleFirmware;
      
      render(<FirmwareManagement asset={mockAsset} />);
      
      // Verify all versions are displayed
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
      expect(screen.getByText('2.0.0')).toBeInTheDocument();
      expect(screen.getByText('3.0.0')).toBeInTheDocument();
      
      // Delete middle version
      mockFirmwareStore.deleteFirmware.mockImplementation(async (id: number) => {
        mockFirmwareStore.firmwareVersions[1] = mockFirmwareStore.firmwareVersions[1].filter(
          (f: any) => f.id !== id
        );
      });
      
      const deleteButtons = screen.getAllByText('Delete');
      fireEvent.click(deleteButtons[1]); // Delete version 2.0.0
      
      await waitFor(() => {
        expect(mockFirmwareStore.deleteFirmware).toHaveBeenCalledWith(2);
      });
      
      // Re-render and verify only versions 1 and 3 remain
      const { rerender } = render(<FirmwareManagement asset={mockAsset} />);
      mockFirmwareStore.firmwareVersions[1] = [
        multipleFirmware[0],
        multipleFirmware[2]
      ];
      rerender(<FirmwareManagement asset={mockAsset} />);
      
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
      expect(screen.queryByText('2.0.0')).not.toBeInTheDocument();
      expect(screen.getByText('3.0.0')).toBeInTheDocument();
    });

    it('should refresh list after delete callback', async () => {
      mockFirmwareStore.firmwareVersions[1] = [mockFirmwareVersion];
      
      render(<FirmwareManagement asset={mockAsset} />);
      
      // Setup delete to trigger reload
      let deleteCallback: (() => void) | undefined;
      mockFirmwareStore.deleteFirmware.mockImplementation(async () => {
        mockFirmwareStore.firmwareVersions[1] = [];
        if (deleteCallback) deleteCallback();
      });
      
      // Capture the onDelete callback
      const FirmwareVersionList = vi.mocked(
        await import('../FirmwareVersionList')
      ).default;
      
      expect(FirmwareVersionList).toHaveBeenCalledWith(
        expect.objectContaining({
          onDelete: expect.any(Function)
        }),
        expect.anything()
      );
      
      deleteCallback = FirmwareVersionList.mock.calls[0][0].onDelete;
      
      // Click delete
      fireEvent.click(screen.getByText('Delete'));
      
      await waitFor(() => {
        expect(mockFirmwareStore.loadFirmwareVersions).toHaveBeenCalledTimes(2); // Initial + after delete
      });
    });
  });

  describe('Permission-based Workflow', () => {
    it('should not allow upload for non-engineer users', () => {
      (useAuthStore as any).mockReturnValue({
        ...mockAuthStore,
        user: { ...mockAuthStore.user, role: 'Administrator' }
      });
      
      render(<FirmwareManagement asset={mockAsset} />);
      
      // Upload button should not be visible
      expect(screen.queryByRole('button', { name: /upload firmware/i })).not.toBeInTheDocument();
    });

    it('should show different empty state message for non-engineers', () => {
      (useAuthStore as any).mockReturnValue({
        ...mockAuthStore,
        user: { ...mockAuthStore.user, role: 'Administrator' }
      });
      
      render(<FirmwareManagement asset={mockAsset} />);
      
      expect(screen.getByText('No firmware files have been uploaded for this asset')).toBeInTheDocument();
    });
  });
});