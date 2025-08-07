import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { expect, test, vi, beforeEach, describe } from 'vitest';
import userEvent from '@testing-library/user-event';
import BulkImportWizard from './BulkImportWizard';
import { ValidationResults, BulkImportSession } from '../../types/bulk';

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock the bulk store
const mockCreateSession = vi.fn();
const mockUploadFile = vi.fn();
const mockValidateData = vi.fn();
const mockStartProcessing = vi.fn();
const mockClearError = vi.fn();

vi.mock('../../store/bulk', () => ({
  default: () => ({
    isLoading: false,
    error: null,
    createSession: mockCreateSession,
    uploadFile: mockUploadFile,
    validateData: mockValidateData,
    startProcessing: mockStartProcessing,
    clearError: mockClearError,
  }),
}));

// Mock ImportTemplateManager
vi.mock('./ImportTemplateManager', () => ({
  default: ({ visible, onClose, onSelectTemplate }: any) =>
    visible ? (
      <div data-testid="template-manager">
        <button onClick={onClose}>Close Templates</button>
        <button 
          onClick={() => onSelectTemplate({
            template_name: 'Test Template',
            template_type: 'assets',
          })}
        >
          Select Template
        </button>
      </div>
    ) : null,
}));

// Mock validation results
const mockValidationResults: ValidationResults = {
  is_valid: true,
  errors: [],
  warnings: [
    {
      row: 2,
      field: 'description',
      value: '',
      message: 'Description is empty',
    },
  ],
  preview_items: [
    {
      name: 'Asset 1',
      asset_type: 'Equipment',
      description: 'Test equipment',
      parent_name: 'Parent Asset',
    },
    {
      name: 'Asset 2',
      asset_type: 'Device',
      description: '',
      parent_name: 'Parent Asset',
    },
  ],
};

const mockSession: BulkImportSession = {
  id: 1,
  session_name: 'Test Session',
  import_type: 'assets',
  total_items: 0,
  processed_items: 0,
  failed_items: 0,
  status: 'Created',
  template_path: null,
  error_log: null,
  created_by: 1,
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
  completed_at: null,
};

describe('BulkImportWizard', () => {
  const mockOnClose = vi.fn();
  const mockOnComplete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockCreateSession.mockResolvedValue(mockSession);
    mockUploadFile.mockResolvedValue(undefined);
    mockValidateData.mockResolvedValue(mockValidationResults);
    mockStartProcessing.mockResolvedValue(undefined);
  });

  test('should render wizard when visible', () => {
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText('Bulk Import Wizard')).toBeInTheDocument();
    expect(screen.getByText('Import Session Configuration')).toBeInTheDocument();
  });

  test('should not render wizard when not visible', () => {
    render(
      <BulkImportWizard
        visible={false}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.queryByText('Bulk Import Wizard')).not.toBeInTheDocument();
  });

  test('should display all wizard steps', () => {
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText('Session Setup')).toBeInTheDocument();
    expect(screen.getByText('File Upload')).toBeInTheDocument();
    expect(screen.getByText('Validation')).toBeInTheDocument();
    expect(screen.getByText('Review')).toBeInTheDocument();
    expect(screen.getByText('Configuration')).toBeInTheDocument();
  });

  test('should validate session name input', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    
    // Test empty name
    await user.clear(sessionNameInput);
    await user.tab(); // Trigger validation
    
    await waitFor(() => {
      expect(screen.getByText('Please enter a session name')).toBeInTheDocument();
    });

    // Test valid name
    await user.type(sessionNameInput, 'Valid Session Name');
    
    // Validation error should be gone
    await waitFor(() => {
      expect(screen.queryByText('Please enter a session name')).not.toBeInTheDocument();
    });
  });

  test('should navigate through wizard steps', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Step 1: Fill session configuration
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Import Session');
    
    const nextButton = screen.getByText('Next');
    await user.click(nextButton);

    // Should move to Step 2: File Upload
    await waitFor(() => {
      expect(screen.getByText('Upload CSV File')).toBeInTheDocument();
      expect(screen.getByText(/Click or drag CSV file/)).toBeInTheDocument();
    });

    // Go back to previous step
    const backButton = screen.getByText('Back');
    await user.click(backButton);

    await waitFor(() => {
      expect(screen.getByText('Import Session Configuration')).toBeInTheDocument();
    });
  });

  test('should open template manager when Browse Templates is clicked', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    const browseButton = screen.getByText('Browse Templates');
    await user.click(browseButton);

    await waitFor(() => {
      expect(screen.getByTestId('template-manager')).toBeInTheDocument();
    });

    // Select a template
    const selectTemplateButton = screen.getByText('Select Template');
    await user.click(selectTemplateButton);

    // Template manager should close and template should be selected
    await waitFor(() => {
      expect(screen.queryByTestId('template-manager')).not.toBeInTheDocument();
    });
  });

  test('should handle file upload', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Move to file upload step
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Session');
    
    const nextButton = screen.getByText('Next');
    await user.click(nextButton);

    await waitFor(() => {
      expect(screen.getByText('Upload CSV File')).toBeInTheDocument();
    });

    // Create a mock CSV file
    const file = new File(['name,type\nAsset1,Equipment'], 'test.csv', {
      type: 'text/csv',
    });

    // Find the file input
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    if (fileInput) {
      // Simulate file selection
      Object.defineProperty(fileInput, 'files', {
        value: [file],
        writable: false,
      });
      
      fireEvent.change(fileInput);
    }

    // The Upload & Continue button should be enabled after file selection
    const uploadButton = screen.getByText('Upload & Continue');
    expect(uploadButton).not.toBeDisabled();
  });

  test('should validate data and show results', async () => {
    const user = userEvent.setup();
    mockCreateSession.mockResolvedValue(mockSession);
    
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Fill session name and move to next step
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Session');
    await user.click(screen.getByText('Next'));

    // Skip file upload for this test (move directly to validation)
    // In real scenario, file would be uploaded first
    
    // Mock that we're on step 2 with a file selected
    const file = new File(['test'], 'test.csv', { type: 'text/csv' });
    const fileInput = document.querySelector('input[type="file"]');
    if (fileInput) {
      Object.defineProperty(fileInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(fileInput);
    }

    // Click upload
    const uploadButton = await screen.findByText('Upload & Continue');
    await user.click(uploadButton);

    // Should move to validation step
    await waitFor(() => {
      expect(screen.getByText('Data Validation')).toBeInTheDocument();
    });

    // Click validate
    const validateButton = screen.getByText('Validate Data');
    await user.click(validateButton);

    // Should show validation results
    await waitFor(() => {
      expect(screen.getByText('Validation Results')).toBeInTheDocument();
      expect(screen.getByText('Validation Successful')).toBeInTheDocument();
      expect(screen.getByText('Total Items:')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument(); // 2 preview items
    });
  });

  test('should configure processing options', async () => {
    const user = userEvent.setup();
    
    // Mock to get to configuration step quickly
    mockCreateSession.mockResolvedValue(mockSession);
    mockValidateData.mockResolvedValue({ ...mockValidationResults, is_valid: true });
    
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Navigate through steps (simplified for testing)
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Session');
    await user.click(screen.getByText('Next'));

    // Simulate file selection and upload
    const file = new File(['test'], 'test.csv', { type: 'text/csv' });
    const fileInput = document.querySelector('input[type="file"]');
    if (fileInput) {
      Object.defineProperty(fileInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(fileInput);
    }
    
    await user.click(await screen.findByText('Upload & Continue'));
    await waitFor(() => screen.getByText('Data Validation'));
    
    await user.click(screen.getByText('Validate Data'));
    await waitFor(() => screen.getByText('Validation Results'));
    
    await user.click(screen.getByText('Continue to Configuration'));

    // Should be on configuration step
    await waitFor(() => {
      expect(screen.getByText('Processing Configuration')).toBeInTheDocument();
      expect(screen.getByText('Configure Import Options')).toBeInTheDocument();
    });

    // Test configuration options
    const skipExistingCheckbox = screen.getByText(/Skip existing assets/);
    const updateExistingCheckbox = screen.getByText(/Update existing assets/);
    const createParentsCheckbox = screen.getByText(/Create missing parent folders/);

    expect(skipExistingCheckbox).toBeInTheDocument();
    expect(updateExistingCheckbox).toBeInTheDocument();
    expect(createParentsCheckbox).toBeInTheDocument();

    // Toggle options
    await user.click(skipExistingCheckbox);
    await user.click(updateExistingCheckbox);
  });

  test('should start import processing', async () => {
    const user = userEvent.setup();
    
    mockCreateSession.mockResolvedValue(mockSession);
    mockValidateData.mockResolvedValue({ ...mockValidationResults, is_valid: true });
    
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Quick navigation to final step
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Session');
    await user.click(screen.getByText('Next'));

    const file = new File(['test'], 'test.csv', { type: 'text/csv' });
    const fileInput = document.querySelector('input[type="file"]');
    if (fileInput) {
      Object.defineProperty(fileInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(fileInput);
    }
    
    await user.click(await screen.findByText('Upload & Continue'));
    await waitFor(() => screen.getByText('Data Validation'));
    
    await user.click(screen.getByText('Validate Data'));
    await waitFor(() => screen.getByText('Validation Results'));
    
    await user.click(screen.getByText('Continue to Configuration'));
    await waitFor(() => screen.getByText('Processing Configuration'));

    // Click Start Import
    const startImportButton = screen.getByText('Start Import');
    await user.click(startImportButton);

    await waitFor(() => {
      expect(mockStartProcessing).toHaveBeenCalledWith(1, expect.objectContaining({
        skip_existing: false,
        update_existing: false,
        create_missing_parents: true,
        validation_mode: 'strict',
      }));
      expect(mockOnComplete).toHaveBeenCalledWith(1);
      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  test('should close wizard when Cancel is clicked', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    const cancelButton = screen.getByText('Cancel');
    await user.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  test('should show validation errors when data is invalid', async () => {
    const invalidResults: ValidationResults = {
      is_valid: false,
      errors: [
        {
          row: 1,
          field: 'name',
          value: '',
          message: 'Name is required',
        },
        {
          row: 3,
          field: 'asset_type',
          value: 'Invalid',
          message: 'Invalid asset type',
        },
      ],
      warnings: [],
      preview_items: [],
    };

    mockValidateData.mockResolvedValue(invalidResults);
    
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    // Navigate to validation
    const sessionNameInput = screen.getByPlaceholderText(/Enter a descriptive name/);
    await user.type(sessionNameInput, 'Test Session');
    await user.click(screen.getByText('Next'));

    const file = new File(['test'], 'test.csv', { type: 'text/csv' });
    const fileInput = document.querySelector('input[type="file"]');
    if (fileInput) {
      Object.defineProperty(fileInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(fileInput);
    }
    
    await user.click(await screen.findByText('Upload & Continue'));
    await waitFor(() => screen.getByText('Data Validation'));
    
    await user.click(screen.getByText('Validate Data'));

    // Should show validation errors
    await waitFor(() => {
      expect(screen.getByText('Validation Issues Found')).toBeInTheDocument();
      expect(screen.getByText('Name is required')).toBeInTheDocument();
      expect(screen.getByText('Invalid asset type')).toBeInTheDocument();
    });

    // Continue button should be disabled
    const continueButton = screen.getByText('Continue to Configuration');
    expect(continueButton).toBeDisabled();
  });
});