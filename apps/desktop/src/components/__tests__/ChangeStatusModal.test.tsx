import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ChangeStatusModal from '../ChangeStatusModal';
import { ConfigurationVersionInfo } from '../../types/assets';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

describe('ChangeStatusModal', () => {
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
    status: 'Draft',
    status_changed_by: undefined,
    status_changed_at: undefined,
    created_at: '2023-01-01T12:00:00Z'
  };

  const defaultProps = {
    visible: true,
    onCancel: vi.fn(),
    onSuccess: vi.fn(),
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
    invoke.mockResolvedValue(['Approved', 'Archived']);
  });

  it('renders modal with version information', async () => {
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    expect(screen.getByText('Change Configuration Status')).toBeInTheDocument();
    expect(screen.getByText('config.json')).toBeInTheDocument();
    expect(screen.getByText('(v1)')).toBeInTheDocument();
    const draftElements = screen.getAllByText('Draft');
    expect(draftElements.length).toBeGreaterThan(0);
  });

  it('loads available status transitions on mount', async () => {
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(async () => {
      const invoke = await getMockInvoke();
      expect(invoke).toHaveBeenCalledWith('get_available_status_transitions', {
        token: 'test-token',
        versionId: 1
      });
    });
  });

  it('displays available status options after loading', async () => {
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      const selectStatusElements = screen.getAllByText('Select new status');
      expect(selectStatusElements.length).toBeGreaterThan(0);
    });

    const select = screen.getByRole('combobox');
    fireEvent.mouseDown(select);

    await waitFor(() => {
      const approvedElements = screen.getAllByText('Approved');
      const archivedElements = screen.getAllByText('Archived');
      expect(approvedElements.length).toBeGreaterThan(0);
      expect(archivedElements.length).toBeGreaterThan(0);
    });
  });

  it('shows loading state while fetching transitions', async () => {
    const invoke = await getMockInvoke();
    invoke.mockImplementation(() => new Promise(() => {})); // Never resolves
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    expect(screen.getByText('Loading available status transitions...')).toBeInTheDocument();
  });

  it('shows error when transitions fail to load', async () => {
    const invoke = await getMockInvoke();
    invoke.mockRejectedValue('Failed to load transitions');
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      expect(screen.getByText('Failed to load transitions')).toBeInTheDocument();
    });
  });

  it('shows no transitions message when none available', async () => {
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue([]);
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      expect(screen.getByText('No status changes available')).toBeInTheDocument();
    });
  });

  it('submits status change successfully', async () => {
    const invoke = await getMockInvoke();
    invoke
      .mockResolvedValueOnce(['Approved'])
      .mockResolvedValueOnce(undefined);

    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const select = screen.getByRole('combobox');
    await act(async () => {
      fireEvent.mouseDown(select);
    });
    
    await waitFor(() => {
      const approvedOption = screen.getAllByText('Approved')[0];
      act(() => {
        fireEvent.click(approvedOption);
      });
    });

    const reasonInput = screen.getByPlaceholderText('Provide a reason for this status change...');
    await act(async () => {
      fireEvent.change(reasonInput, { target: { value: 'Ready for production' } });
    });

    const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
    await act(async () => {
      fireEvent.click(submitButton);
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenNthCalledWith(2, 'update_configuration_status', {
        token: 'test-token',
        versionId: 1,
        newStatus: 'Approved',
        changeReason: 'Ready for production'
      });
    });

    expect(defaultProps.onSuccess).toHaveBeenCalled();
  });

  it('handles submission error', async () => {
    const invoke = await getMockInvoke();
    invoke
      .mockResolvedValueOnce(['Approved'])
      .mockRejectedValueOnce(new Error('Permission denied'));

    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const select = screen.getByRole('combobox');
    await act(async () => {
      fireEvent.mouseDown(select);
    });

    await waitFor(() => {
      const approvedOption = screen.getAllByText('Approved')[0];
      act(() => {
        fireEvent.click(approvedOption);
      });
    });

    const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
    await act(async () => {
      fireEvent.click(submitButton);
    });

    await waitFor(() => {
      const errorElements = screen.getAllByText('Permission denied');
      expect(errorElements.length).toBeGreaterThan(0);
    });

    expect(defaultProps.onSuccess).not.toHaveBeenCalled();
  });

  it('validates required status field', async () => {
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue(['Approved']);
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
      fireEvent.click(submitButton);
    });

    await waitFor(() => {
      const pleaseSelectElements = screen.getAllByText('Please select a new status');
      expect(pleaseSelectElements.length).toBeGreaterThan(0);
    });
  });

  it('validates reason length', async () => {
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue(['Approved']);
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    const reasonInput = screen.getByPlaceholderText('Provide a reason for this status change...');
    const longReason = 'a'.repeat(501);
    fireEvent.change(reasonInput, { target: { value: longReason } });

    const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText('Reason cannot exceed 500 characters')).toBeInTheDocument();
    });
  });

  it('cancels modal and resets form', async () => {
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(defaultProps.onCancel).toHaveBeenCalled();
  });

  it('does not render when version is null', async () => {
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} version={null} />);
    });
    
    expect(screen.queryByText('Change Configuration Status')).not.toBeInTheDocument();
  });

  it('disables submit button when no transitions available', async () => {
    const invoke = await getMockInvoke();
    invoke.mockResolvedValue([]);
    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
      expect(submitButton).toBeDisabled();
    });
  });

  it('shows loading state during submission', async () => {
    const invoke = await getMockInvoke();
    invoke
      .mockResolvedValueOnce(['Approved'])
      .mockImplementation(() => new Promise(() => {})); // Never resolves

    await act(async () => {
      render(<ChangeStatusModal {...defaultProps} />);
    });
    
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const select = screen.getByRole('combobox');
    await act(async () => {
      fireEvent.mouseDown(select);
    });

    await waitFor(() => {
      const approvedOption = screen.getAllByText('Approved')[0];
      act(() => {
        fireEvent.click(approvedOption);
      });
    });

    const submitButtons = screen.getAllByText('Update Status');
    const submitButton = submitButtons[0];
    await act(async () => {
      fireEvent.click(submitButton);
    });

    await waitFor(() => {
      expect(submitButton).toHaveAttribute('disabled');
    });
  });
});