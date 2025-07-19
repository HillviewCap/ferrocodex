import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, expect, describe, it, beforeEach } from 'vitest';
import PromotionConfirmationModal from '../PromotionConfirmationModal';
import { ConfigurationVersionInfo } from '../../types/assets';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

const { invoke } = vi.mocked(await import('@tauri-apps/api/core'));

const mockVersion: ConfigurationVersionInfo = {
  id: 1,
  asset_id: 1,
  version_number: 'v2',
  file_name: 'config.json',
  file_size: 1024,
  content_hash: 'abc123',
  author: 1,
  author_username: 'john.doe',
  notes: 'Test configuration',
  status: 'Approved',
  status_changed_by: 1,
  status_changed_at: '2025-01-15T10:00:00Z',
  created_at: '2025-01-15T09:00:00Z'
};

const mockProps = {
  visible: true,
  onCancel: vi.fn(),
  onSuccess: vi.fn(),
  version: mockVersion,
  token: 'test-token'
};

describe('PromotionConfirmationModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_promotion_eligibility':
          return Promise.resolve(true);
        case 'get_golden_version':
          return Promise.resolve(null);
        case 'promote_to_golden':
          return Promise.resolve();
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });
  });

  it('renders modal with version information', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    expect(screen.getByText('Promote to Golden Image')).toBeInTheDocument();
    expect(screen.getByText('Version to Promote')).toBeInTheDocument();
    expect(screen.getByText('config.json')).toBeInTheDocument();
    expect(screen.getByText('v2')).toBeInTheDocument();
    expect(screen.getByText('john.doe')).toBeInTheDocument();
  });

  it('checks promotion eligibility on mount', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_promotion_eligibility', {
        token: 'test-token',
        versionId: 1
      });
    });
  });

  it('checks for existing golden version on mount', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_golden_version', {
        token: 'test-token',
        assetId: 1
      });
    });
  });

  it('displays eligibility success when version is eligible', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });
  });

  it('displays eligibility error when version is not eligible', async () => {
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_promotion_eligibility':
          return Promise.resolve(false);
        case 'get_golden_version':
          return Promise.resolve(null);
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });

    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Not Eligible for Promotion')).toBeInTheDocument();
    });

    // OK button should be disabled
    const okButton = screen.getByText('Promote to Golden').closest('button');
    expect(okButton).toBeDisabled();
  });

  it('shows existing golden version warning when present', async () => {
    const existingGolden: ConfigurationVersionInfo = {
      ...mockVersion,
      id: 2,
      version_number: 'v1',
      status: 'Golden'
    };
    
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_promotion_eligibility':
          return Promise.resolve(true);
        case 'get_golden_version':
          return Promise.resolve(existingGolden);
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });
    
    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Existing Golden Version Will Be Archived')).toBeInTheDocument();
      expect(screen.getByText('config.json (v1)')).toBeInTheDocument();
    });
  });

  it('displays promotion effects when eligible', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Promotion Effects')).toBeInTheDocument();
      expect(screen.getByText(/Version status will change to/)).toBeInTheDocument();
      expect(screen.getByText(/Version will be prominently displayed/)).toBeInTheDocument();
      expect(screen.getByText(/Audit trail will record/)).toBeInTheDocument();
    });
  });

  it('requires promotion reason for submission', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    // Wait for eligibility check
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });

    // Try to submit without reason
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(screen.getByText('Please provide a reason for this promotion')).toBeInTheDocument();
    });
  });

  it('successfully promotes version with valid reason', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    // Wait for eligibility check
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });

    // Fill in promotion reason
    const reasonTextarea = screen.getByPlaceholderText('Explain why this version should be promoted to Golden Image...');
    fireEvent.change(reasonTextarea, { target: { value: 'This version is stable and ready for production' } });
    
    // Submit promotion
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('promote_to_golden', {
        token: 'test-token',
        versionId: 1,
        promotionReason: 'This version is stable and ready for production'
      });
    });
    
    await waitFor(() => {
      expect(mockProps.onSuccess).toHaveBeenCalled();
    });
  });

  it('handles promotion errors gracefully', async () => {
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_promotion_eligibility':
          return Promise.resolve(true);
        case 'get_golden_version':
          return Promise.resolve(null);
        case 'promote_to_golden':
          return Promise.reject('Failed to promote version');
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });
    
    render(<PromotionConfirmationModal {...mockProps} />);
    
    // Wait for eligibility check
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });

    // Fill form and submit
    const reasonTextarea = screen.getByPlaceholderText('Explain why this version should be promoted to Golden Image...');
    fireEvent.change(reasonTextarea, { target: { value: 'Test promotion' } });
    
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(screen.getByText('Promotion Failed')).toBeInTheDocument();
      expect(screen.getByText('Failed to promote version')).toBeInTheDocument();
    });
  });

  it('calls onCancel when cancel button is clicked', () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    fireEvent.click(screen.getByText('Cancel'));
    expect(mockProps.onCancel).toHaveBeenCalled();
  });

  it('does not render when version is null', () => {
    render(<PromotionConfirmationModal {...mockProps} version={null} />);
    expect(screen.queryByText('Promote to Golden Image')).not.toBeInTheDocument();
  });

  it('validates reason length limit', async () => {
    render(<PromotionConfirmationModal {...mockProps} />);
    
    // Wait for eligibility check
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });

    // Fill in reason that exceeds limit
    const reasonTextarea = screen.getByPlaceholderText('Explain why this version should be promoted to Golden Image...');
    const longReason = 'A'.repeat(501); // Exceeds 500 character limit
    fireEvent.change(reasonTextarea, { target: { value: longReason } });
    
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(screen.getByText('Reason cannot exceed 500 characters')).toBeInTheDocument();
    });
  });

  it('shows loading state during promotion', async () => {
    // Mock a slow promotion
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_promotion_eligibility':
          return Promise.resolve(true);
        case 'get_golden_version':
          return Promise.resolve(null);
        case 'promote_to_golden':
          return new Promise(resolve => setTimeout(resolve, 100));
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });
    
    render(<PromotionConfirmationModal {...mockProps} />);
    
    // Wait for eligibility check
    await waitFor(() => {
      expect(screen.getByText('Ready for Promotion')).toBeInTheDocument();
    });

    // Fill form and submit
    const reasonTextarea = screen.getByPlaceholderText('Explain why this version should be promoted to Golden Image...');
    fireEvent.change(reasonTextarea, { target: { value: 'Test promotion' } });
    
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    // Button should show loading state
    const promoteButton = screen.getByText('Promote to Golden').closest('button');
    expect(promoteButton).toHaveClass('ant-btn-loading');
  });
});