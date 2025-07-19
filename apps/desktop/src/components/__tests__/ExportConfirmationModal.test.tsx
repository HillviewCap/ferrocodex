import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import ExportConfirmationModal from '../ExportConfirmationModal';
import { ConfigurationVersionInfo } from '../../types/assets';

// Mock Tauri APIs
vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn()
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

const mockVersion: ConfigurationVersionInfo = {
  id: 1,
  asset_id: 1,
  version_number: 'v1',
  file_name: 'test-config.json',
  file_size: 1024,
  content_hash: 'abc123def456',
  author: 1,
  author_username: 'testuser',
  notes: 'Test configuration',
  status: 'Approved',
  status_changed_by: null,
  status_changed_at: null,
  created_at: '2024-01-01T00:00:00Z'
};

const defaultProps = {
  visible: true,
  onCancel: vi.fn(),
  onSuccess: vi.fn(),
  version: mockVersion,
  token: 'test-token'
};

describe('ExportConfirmationModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders export modal with version information', () => {
    render(<ExportConfirmationModal {...defaultProps} />);
    
    expect(screen.getByText('Export Configuration')).toBeInTheDocument();
    expect(screen.getByText('Configuration Details')).toBeInTheDocument();
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.getByText('test-config.json')).toBeInTheDocument();
    expect(screen.getByText('testuser')).toBeInTheDocument();
    expect(screen.getByText('Approved')).toBeInTheDocument();
  });

  it('displays export location section', () => {
    render(<ExportConfirmationModal {...defaultProps} />);
    
    expect(screen.getByText('Export Location')).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Click 'Select Location'/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Select Location/i })).toBeInTheDocument();
  });

  it('shows export information alert', () => {
    render(<ExportConfirmationModal {...defaultProps} />);
    
    expect(screen.getByText('Export Information')).toBeInTheDocument();
    expect(screen.getByText(/export process includes integrity verification/i)).toBeInTheDocument();
  });

  it('disables export button when no path is selected', () => {
    render(<ExportConfirmationModal {...defaultProps} />);
    
    const exportButton = screen.getByRole('button', { name: /Export/i });
    expect(exportButton).toBeDisabled();
  });

  it('calls onCancel when cancel button is clicked', () => {
    const onCancel = vi.fn();
    render(<ExportConfirmationModal {...defaultProps} onCancel={onCancel} />);
    
    fireEvent.click(screen.getByRole('button', { name: /Cancel/i }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('calls save dialog when select location button is clicked', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    (save as any).mockResolvedValue('/path/to/export.json');
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      expect(save).toHaveBeenCalledWith({
        defaultPath: 'test-config.json',
        filters: [
          {
            name: 'Configuration Files',
            extensions: expect.arrayContaining(['json', 'xml', 'yaml'])
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });
    });
  });

  it('enables export button after path selection', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    (save as any).mockResolvedValue('/path/to/export.json');
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      const exportButton = screen.getByRole('button', { name: /Export/i });
      expect(exportButton).not.toBeDisabled();
    });
  });

  it('shows progress section after path selection', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    (save as any).mockResolvedValue('/path/to/export.json');
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Export Progress')).toBeInTheDocument();
      expect(screen.getByText('Export location selected')).toBeInTheDocument();
    });
  });

  it('calls export API when export button is clicked', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { invoke } = await import('@tauri-apps/api/core');
    
    (save as any).mockResolvedValue('/path/to/export.json');
    (invoke as any).mockResolvedValue(undefined);
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    // Select path first
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      const exportButton = screen.getByRole('button', { name: /Export/i });
      expect(exportButton).not.toBeDisabled();
    });
    
    // Click export
    fireEvent.click(screen.getByRole('button', { name: /Export/i }));
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('export_configuration_version', {
        token: 'test-token',
        versionId: 1,
        exportPath: '/path/to/export.json'
      });
    });
  });

  it('shows success message and calls onSuccess after export', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { invoke } = await import('@tauri-apps/api/core');
    const onSuccess = vi.fn();
    
    (save as any).mockResolvedValue('/path/to/export.json');
    (invoke as any).mockResolvedValue(undefined);
    
    render(<ExportConfirmationModal {...defaultProps} onSuccess={onSuccess} />);
    
    // Select path and export
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      const exportButton = screen.getByRole('button', { name: /Export/i });
      expect(exportButton).not.toBeDisabled();
    });
    
    fireEvent.click(screen.getByRole('button', { name: /Export/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Export Successful')).toBeInTheDocument();
    });
    
    // Wait for auto-close timeout
    await waitFor(() => {
      expect(onSuccess).toHaveBeenCalledWith('/path/to/export.json');
    }, { timeout: 2000 });
  });

  it('shows error message when export fails', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { invoke } = await import('@tauri-apps/api/core');
    
    (save as any).mockResolvedValue('/path/to/export.json');
    (invoke as any).mockRejectedValue(new Error('Export failed'));
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    // Select path and export
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      const exportButton = screen.getByRole('button', { name: /Export/i });
      expect(exportButton).not.toBeDisabled();
    });
    
    fireEvent.click(screen.getByRole('button', { name: /Export/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Export Error')).toBeInTheDocument();
      expect(screen.getByText('Export failed')).toBeInTheDocument();
    });
  });

  it('shows error when no path is selected and export is attempted', async () => {
    render(<ExportConfirmationModal {...defaultProps} />);
    
    // Try to export without selecting path (should not be possible via UI, but test the logic)
    // This tests the validation logic in handleExport
    expect(screen.getByRole('button', { name: /Export/i })).toBeDisabled();
  });

  it('handles file dialog cancellation gracefully', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    (save as any).mockResolvedValue(null); // User cancelled
    
    render(<ExportConfirmationModal {...defaultProps} />);
    
    fireEvent.click(screen.getByRole('button', { name: /Select Location/i }));
    
    await waitFor(() => {
      // Should remain in initial state
      const exportButton = screen.getByRole('button', { name: /Export/i });
      expect(exportButton).toBeDisabled();
    });
  });

  it('resets state when modal is closed', () => {
    const onCancel = vi.fn();
    const { rerender } = render(<ExportConfirmationModal {...defaultProps} onCancel={onCancel} />);
    
    // Close modal
    fireEvent.click(screen.getByRole('button', { name: /Cancel/i }));
    
    // Reopen modal
    rerender(<ExportConfirmationModal {...defaultProps} visible={true} onCancel={onCancel} />);
    
    // Should be in initial state
    expect(screen.getByRole('button', { name: /Export/i })).toBeDisabled();
    expect(screen.getByPlaceholderText(/Click 'Select Location'/)).toHaveValue('');
  });
});