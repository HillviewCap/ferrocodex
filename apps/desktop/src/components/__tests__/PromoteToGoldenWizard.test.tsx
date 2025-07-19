import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, expect, describe, it, beforeEach } from 'vitest';
import PromoteToGoldenWizard from '../PromoteToGoldenWizard';
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

describe('PromoteToGoldenWizard', () => {
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

  it('renders the wizard with correct initial state', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    expect(screen.getByText('Golden Image Promotion - Information')).toBeInTheDocument();
    expect(screen.getByText('Promote to Golden Image')).toBeInTheDocument();
    expect(screen.getByText('What is a Golden Image?')).toBeInTheDocument();
    expect(screen.getByText('config.json')).toBeInTheDocument();
    expect(screen.getByText('v2')).toBeInTheDocument();
  });

  it('checks promotion eligibility on mount', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_promotion_eligibility', {
        token: 'test-token',
        versionId: 1
      });
    });
  });

  it('checks for existing golden version on mount', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_golden_version', {
        token: 'test-token',
        assetId: 1
      });
    });
  });

  it('navigates through wizard steps correctly', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Start at step 0
    expect(screen.getByText('Information')).toBeInTheDocument();
    
    // Click Next to go to step 1
    fireEvent.click(screen.getByText('Next'));
    expect(screen.getByText('Golden Image Promotion - Impact Assessment')).toBeInTheDocument();
    
    // Wait for eligibility check to complete
    await waitFor(() => {
      expect(screen.getByText('Promotion Eligible')).toBeInTheDocument();
    });
    
    // Click Next to go to step 2
    fireEvent.click(screen.getByText('Next'));
    expect(screen.getByText('Golden Image Promotion - Confirmation')).toBeInTheDocument();
  });

  it('shows previous button and allows going back', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to step 1
    fireEvent.click(screen.getByText('Next'));
    
    // Previous button should be visible
    expect(screen.getByText('Previous')).toBeInTheDocument();
    
    // Click Previous to go back to step 0
    fireEvent.click(screen.getByText('Previous'));
    expect(screen.getByText('Golden Image Promotion - Information')).toBeInTheDocument();
  });

  it('displays existing golden version warning when present', async () => {
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
    
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to impact assessment
    fireEvent.click(screen.getByText('Next'));
    
    await waitFor(() => {
      expect(screen.getByText('Existing Golden Version Will Be Archived')).toBeInTheDocument();
    });
  });

  it('prevents promotion when version is not eligible', async () => {
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
    
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to impact assessment
    fireEvent.click(screen.getByText('Next'));
    
    await waitFor(() => {
      expect(screen.getByText('Not Eligible for Promotion')).toBeInTheDocument();
    });
    
    // Next button should be disabled
    const nextButton = screen.getByText('Next');
    expect(nextButton.closest('button')).toBeDisabled();
  });

  it('requires promotion reason in final step', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to final step
    fireEvent.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByText('Promotion Eligible')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('Next'));
    
    // Try to promote without reason
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(screen.getByText('Please provide a reason for this promotion')).toBeInTheDocument();
    });
  });

  it('successfully promotes version with valid reason', async () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to final step
    fireEvent.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByText('Promotion Eligible')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('Next'));
    
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
    
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    // Navigate to final step and fill form
    fireEvent.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByText('Promotion Eligible')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('Next'));
    
    const reasonTextarea = screen.getByPlaceholderText('Explain why this version should be promoted to Golden Image...');
    fireEvent.change(reasonTextarea, { target: { value: 'Test promotion' } });
    
    // Submit promotion
    fireEvent.click(screen.getByText('Promote to Golden'));
    
    await waitFor(() => {
      expect(screen.getByText('Promotion Failed')).toBeInTheDocument();
      expect(screen.getByText('Failed to promote version')).toBeInTheDocument();
    });
  });

  it('calls onCancel when cancel button is clicked', () => {
    render(<PromoteToGoldenWizard {...mockProps} />);
    
    fireEvent.click(screen.getByText('Cancel'));
    expect(mockProps.onCancel).toHaveBeenCalled();
  });

  it('does not render when version is null', () => {
    render(<PromoteToGoldenWizard {...mockProps} version={null} />);
    expect(screen.queryByText('Golden Image Promotion - Information')).not.toBeInTheDocument();
  });
});