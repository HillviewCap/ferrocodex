import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import FirmwareUploadModal from '../FirmwareUploadModal';
import useFirmwareStore from '../../store/firmware';

// Mock the dependencies
vi.mock('../../store/firmware');
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn()
}));

// Import mocked modules
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';

describe('FirmwareUploadModal', () => {
  const mockProps = {
    visible: true,
    onCancel: vi.fn(),
    onSuccess: vi.fn(),
    assetId: 1
  };

  const mockFirmwareStore = {
    uploadFirmware: vi.fn().mockResolvedValue(undefined),
    uploadProgress: null,
    resetUploadProgress: vi.fn()
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useFirmwareStore as any).mockReturnValue(mockFirmwareStore);
  });

  it('renders modal with initial file selection step', () => {
    render(<FirmwareUploadModal {...mockProps} />);
    
    expect(screen.getByText('Upload Firmware')).toBeInTheDocument();
    expect(screen.getByText('Click to select firmware file')).toBeInTheDocument();
    expect(screen.getByText('Support for binary firmware files (max 2GB)')).toBeInTheDocument();
  });

  it('shows security information', () => {
    render(<FirmwareUploadModal {...mockProps} />);
    
    expect(screen.getByText('Firmware files are stored encrypted')).toBeInTheDocument();
    expect(screen.getByText(/All firmware files are encrypted before storage/)).toBeInTheDocument();
  });

  it('does not render when visible is false', () => {
    render(<FirmwareUploadModal {...mockProps} visible={false} />);
    
    expect(screen.queryByText('Upload Firmware')).not.toBeInTheDocument();
  });

  it('calls onCancel when cancel button clicked', () => {
    render(<FirmwareUploadModal {...mockProps} />);
    
    fireEvent.click(screen.getByText('Cancel'));
    expect(mockProps.onCancel).toHaveBeenCalled();
  });

  it('handles file selection', async () => {
    const mockFile = '/path/to/firmware.bin';
    const mockMetadata = {
      name: 'firmware.bin',
      size: 1048576, // 1MB
      content_type: 'application/octet-stream',
      hash: 'abc123'
    };
    
    (dialog.open as any).mockResolvedValue(mockFile);
    (invoke as any).mockResolvedValue(mockMetadata);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    // Click the file selection area
    const dropArea = screen.getByText('Click to select firmware file').parentElement;
    fireEvent.click(dropArea!);
    
    await waitFor(() => {
      expect(dialog.open).toHaveBeenCalledWith({
        multiple: false,
        filters: [{
          name: 'Firmware Files',
          extensions: ['bin', 'hex', 'img', 'rom', 'fw', 'elf', 'dfu', 'upd', 'dat', '*']
        }]
      });
    });
  });

  it('validates file size limit', async () => {
    const mockFile = '/path/to/large.bin';
    const mockMetadata = {
      name: 'large.bin',
      size: 3 * 1024 * 1024 * 1024, // 3GB (exceeds 2GB limit)
      content_type: 'application/octet-stream',
      hash: 'abc123'
    };
    
    (dialog.open as any).mockResolvedValue(mockFile);
    (invoke as any).mockResolvedValue(mockMetadata);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    const dropArea = screen.getByText('Click to select firmware file').parentElement;
    fireEvent.click(dropArea!);
    
    // Should not proceed to next step due to size validation
    await waitFor(() => {
      expect(screen.getByText('Click to select firmware file')).toBeInTheDocument();
    });
  });

  it('displays file details after selection', async () => {
    const mockFile = '/path/to/firmware.bin';
    const mockMetadata = {
      name: 'firmware.bin',
      size: 1048576, // 1MB
      content_type: 'application/octet-stream',
      hash: 'abc123'
    };
    
    (dialog.open as any).mockResolvedValue(mockFile);
    (invoke as any).mockResolvedValue(mockMetadata);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    const dropArea = screen.getByText('Click to select firmware file').parentElement;
    fireEvent.click(dropArea!);
    
    await waitFor(() => {
      expect(screen.getByText('firmware.bin')).toBeInTheDocument();
      expect(screen.getByText('1 MB')).toBeInTheDocument();
    });
  });

  it('shows form fields in details step', async () => {
    // Set up to reach the details step
    const mockFile = '/path/to/firmware.bin';
    const mockMetadata = {
      name: 'firmware.bin',
      size: 1048576,
      content_type: 'application/octet-stream',
      hash: 'abc123'
    };
    
    (dialog.open as any).mockResolvedValue(mockFile);
    (invoke as any).mockResolvedValue(mockMetadata);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    const dropArea = screen.getByText('Click to select firmware file').parentElement;
    fireEvent.click(dropArea!);
    
    await waitFor(() => {
      expect(screen.getByLabelText('Vendor (Optional)')).toBeInTheDocument();
      expect(screen.getByLabelText('Model (Optional)')).toBeInTheDocument();
      expect(screen.getByLabelText('Version')).toBeInTheDocument();
      expect(screen.getByLabelText('Notes (Optional)')).toBeInTheDocument();
    });
  });

  it('shows upload progress', () => {
    const progressStore = {
      ...mockFirmwareStore,
      uploadProgress: { progress: 50, status: 'uploading' as const }
    };
    (useFirmwareStore as any).mockReturnValue(progressStore);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    expect(screen.getByText('Uploading Firmware...')).toBeInTheDocument();
  });

  it('shows success state', () => {
    const successStore = {
      ...mockFirmwareStore,
      uploadProgress: { progress: 100, status: 'complete' as const }
    };
    (useFirmwareStore as any).mockReturnValue(successStore);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    expect(screen.getByText('Upload Complete!')).toBeInTheDocument();
    expect(screen.getByText('Firmware has been uploaded and encrypted successfully')).toBeInTheDocument();
  });

  it('shows error state', () => {
    const errorStore = {
      ...mockFirmwareStore,
      uploadProgress: { 
        progress: 0, 
        status: 'error' as const,
        message: 'Upload failed due to network error'
      }
    };
    (useFirmwareStore as any).mockReturnValue(errorStore);
    
    render(<FirmwareUploadModal {...mockProps} />);
    
    expect(screen.getByText('Upload Failed')).toBeInTheDocument();
    expect(screen.getByText('Upload failed due to network error')).toBeInTheDocument();
  });

  it('resets state when modal is closed', () => {
    render(<FirmwareUploadModal {...mockProps} />);
    
    fireEvent.click(screen.getByText('Cancel'));
    
    expect(mockFirmwareStore.resetUploadProgress).toHaveBeenCalled();
    expect(mockProps.onCancel).toHaveBeenCalled();
  });

  describe('Form validation', () => {
    beforeEach(async () => {
      // Setup to reach the form step
      const mockFile = '/path/to/firmware.bin';
      const mockMetadata = {
        name: 'firmware.bin',
        size: 1048576,
        content_type: 'application/octet-stream',
        hash: 'abc123'
      };
      
      (dialog.open as any).mockResolvedValue(mockFile);
      (invoke as any).mockResolvedValue(mockMetadata);
    });

    it('validates vendor field length', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      // Select file first
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Vendor (Optional)')).toBeInTheDocument();
      });
      
      // Enter vendor name that's too long (>100 chars)
      const vendorInput = screen.getByLabelText('Vendor (Optional)');
      const longVendor = 'A'.repeat(101);
      fireEvent.change(vendorInput, { target: { value: longVendor } });
      fireEvent.blur(vendorInput);
      
      await waitFor(() => {
        expect(screen.getByText('Vendor name cannot exceed 100 characters')).toBeInTheDocument();
      });
    });

    it('validates model field length', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Model (Optional)')).toBeInTheDocument();
      });
      
      const modelInput = screen.getByLabelText('Model (Optional)');
      const longModel = 'B'.repeat(101);
      fireEvent.change(modelInput, { target: { value: longModel } });
      fireEvent.blur(modelInput);
      
      await waitFor(() => {
        expect(screen.getByText('Model name cannot exceed 100 characters')).toBeInTheDocument();
      });
    });

    it('validates version field is required', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Version')).toBeInTheDocument();
      });
      
      // Try to submit without version
      const uploadButton = screen.getByText('Upload Firmware');
      fireEvent.click(uploadButton);
      
      await waitFor(() => {
        expect(screen.getByText('Please enter firmware version')).toBeInTheDocument();
      });
    });

    it('validates version field pattern', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Version')).toBeInTheDocument();
      });
      
      const versionInput = screen.getByLabelText('Version');
      fireEvent.change(versionInput, { target: { value: 'version with spaces!' } });
      fireEvent.blur(versionInput);
      
      await waitFor(() => {
        expect(screen.getByText(/Version must be.*contain only letters, numbers/)).toBeInTheDocument();
      });
    });

    it('validates notes field length', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Notes (Optional)')).toBeInTheDocument();
      });
      
      const notesInput = screen.getByLabelText('Notes (Optional)');
      const longNotes = 'C'.repeat(1001);
      fireEvent.change(notesInput, { target: { value: longNotes } });
      fireEvent.blur(notesInput);
      
      await waitFor(() => {
        expect(screen.getByText('Notes cannot exceed 1000 characters')).toBeInTheDocument();
      });
    });

    it('accepts valid form data', async () => {
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Version')).toBeInTheDocument();
      });
      
      // Fill valid data
      fireEvent.change(screen.getByLabelText('Vendor (Optional)'), { target: { value: 'Siemens' } });
      fireEvent.change(screen.getByLabelText('Model (Optional)'), { target: { value: 'S7-1500' } });
      fireEvent.change(screen.getByLabelText('Version'), { target: { value: '2.8.1' } });
      fireEvent.change(screen.getByLabelText('Notes (Optional)'), { target: { value: 'Stable release' } });
      
      const uploadButton = screen.getByText('Upload Firmware');
      fireEvent.click(uploadButton);
      
      await waitFor(() => {
        expect(mockFirmwareStore.uploadFirmware).toHaveBeenCalledWith({
          asset_id: 1,
          vendor: 'Siemens',
          model: 'S7-1500',
          version: '2.8.1',
          notes: 'Stable release',
          file_path: '/path/to/firmware.bin'
        });
      });
    });
  });

  describe('Error scenarios', () => {
    it('handles file selection cancellation', async () => {
      (dialog.open as any).mockResolvedValue(null);
      
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        // Should stay on file selection step
        expect(screen.getByText('Click to select firmware file')).toBeInTheDocument();
      });
    });

    it('handles file metadata error gracefully', async () => {
      (dialog.open as any).mockResolvedValue('/path/to/file.bin');
      (invoke as any).mockRejectedValue(new Error('Cannot read file'));
      
      render(<FirmwareUploadModal {...mockProps} />);
      
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        // Should stay on file selection step
        expect(screen.getByText('Click to select firmware file')).toBeInTheDocument();
      });
    });

    it('handles upload failure with error message', async () => {
      const mockFile = '/path/to/firmware.bin';
      const mockMetadata = {
        name: 'firmware.bin',
        size: 1048576,
        content_type: 'application/octet-stream',
        hash: 'abc123'
      };
      
      (dialog.open as any).mockResolvedValue(mockFile);
      (invoke as any).mockResolvedValue(mockMetadata);
      mockFirmwareStore.uploadFirmware.mockRejectedValue(new Error('Network error'));
      
      render(<FirmwareUploadModal {...mockProps} />);
      
      // Select file
      const dropArea = screen.getByText('Click to select firmware file').parentElement;
      fireEvent.click(dropArea!);
      
      await waitFor(() => {
        expect(screen.getByLabelText('Version')).toBeInTheDocument();
      });
      
      // Fill form and submit
      fireEvent.change(screen.getByLabelText('Version'), { target: { value: '1.0.0' } });
      fireEvent.click(screen.getByText('Upload Firmware'));
      
      // Should not close modal on error
      await waitFor(() => {
        expect(mockProps.onSuccess).not.toHaveBeenCalled();
      });
    });
  });
});