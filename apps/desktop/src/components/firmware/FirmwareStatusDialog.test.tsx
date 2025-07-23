import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import FirmwareStatusDialog from './FirmwareStatusDialog';
import { FirmwareStatus } from '../../types/firmware';

describe('FirmwareStatusDialog', () => {
  const mockOnConfirm = vi.fn();
  const mockOnCancel = vi.fn();

  const defaultProps = {
    visible: true,
    currentStatus: 'Draft' as FirmwareStatus,
    availableTransitions: ['Approved', 'Archived'] as FirmwareStatus[],
    onConfirm: mockOnConfirm,
    onCancel: mockOnCancel,
    isPromotingToGolden: false
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders dialog with current status', () => {
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    expect(screen.getByText('Change Firmware Status')).toBeInTheDocument();
    expect(screen.getByText('Draft')).toBeInTheDocument();
  });

  it('shows available transitions in dropdown', async () => {
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    const selectInput = screen.getByRole('combobox');
    await userEvent.click(selectInput);
    
    expect(screen.getByText('Approved')).toBeInTheDocument();
    expect(screen.getByText('Archived')).toBeInTheDocument();
  });

  it('shows warning for Golden promotion', () => {
    render(
      <FirmwareStatusDialog 
        {...defaultProps}
        availableTransitions={['Golden'] as FirmwareStatus[]}
        isPromotingToGolden={true}
      />
    );
    
    expect(screen.getByText('Golden Promotion Warning')).toBeInTheDocument();
    expect(screen.getByText(/will automatically archive any existing Golden versions/)).toBeInTheDocument();
  });

  it('requires reason for Golden promotion', async () => {
    render(
      <FirmwareStatusDialog 
        {...defaultProps}
        availableTransitions={['Golden'] as FirmwareStatus[]}
        isPromotingToGolden={true}
      />
    );
    
    // Try to submit without reason
    const okButton = screen.getByText('Change Status');
    await userEvent.click(okButton);
    
    // Should show validation error
    await waitFor(() => {
      expect(screen.getByText('Reason is required for Golden promotion')).toBeInTheDocument();
    });
  });

  it('submits form with status and reason', async () => {
    mockOnConfirm.mockResolvedValue(undefined);
    
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    // Select new status
    const selectInput = screen.getByRole('combobox');
    await userEvent.click(selectInput);
    await userEvent.click(screen.getByText('Approved'));
    
    // Enter reason
    const reasonInput = screen.getByPlaceholderText(/provide a reason/i);
    await userEvent.type(reasonInput, 'Testing approval process');
    
    // Submit
    const okButton = screen.getByText('Change Status');
    await userEvent.click(okButton);
    
    await waitFor(() => {
      expect(mockOnConfirm).toHaveBeenCalledWith('Approved', 'Testing approval process');
    });
  });

  it('cancels dialog', async () => {
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    const cancelButton = screen.getByText('Cancel');
    await userEvent.click(cancelButton);
    
    expect(mockOnCancel).toHaveBeenCalled();
  });

  it('shows status descriptions', async () => {
    render(
      <FirmwareStatusDialog 
        {...defaultProps}
        availableTransitions={['Golden', 'Approved', 'Archived'] as FirmwareStatus[]}
      />
    );
    
    // Select Golden status
    const selectInput = screen.getByRole('combobox');
    await userEvent.click(selectInput);
    await userEvent.click(screen.getByText('Golden'));
    
    expect(screen.getByText(/golden standard version/i)).toBeInTheDocument();
  });

  it('enforces character limit on reason field', async () => {
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    const reasonInput = screen.getByPlaceholderText(/provide a reason/i);
    const longText = 'a'.repeat(600); // Exceeds 500 char limit
    
    await userEvent.type(reasonInput, longText);
    
    // Should show character count
    expect(screen.getByText(/500 \/ 500/)).toBeInTheDocument();
  });

  it('handles form submission errors gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockOnConfirm.mockRejectedValue(new Error('Network error'));
    
    render(<FirmwareStatusDialog {...defaultProps} />);
    
    // Select status and submit
    const selectInput = screen.getByRole('combobox');
    await userEvent.click(selectInput);
    await userEvent.click(screen.getByText('Approved'));
    
    const okButton = screen.getByText('Change Status');
    await userEvent.click(okButton);
    
    // Dialog should remain open
    await waitFor(() => {
      expect(screen.getByText('Change Firmware Status')).toBeInTheDocument();
    });
    
    consoleSpy.mockRestore();
  });
});