import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
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
          onClick={() => {
            onSelectTemplate({
              template_name: 'Test Template',
              template_type: 'assets',
            });
            onClose();
          }}
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

type User = ReturnType<typeof userEvent.setup>;

// Fill in step 1 and advance to the file upload step
const completeSessionStep = async (user: User, sessionName = 'Test Session') => {
  await user.type(screen.getByPlaceholderText(/Enter a descriptive name/), sessionName);
  await user.click(screen.getByRole('button', { name: 'Next' }));
  await screen.findByText('Upload CSV File');
};

const selectCsvFile = async (user: User) => {
  const file = new File(['name,asset_type\nAsset1,Equipment'], 'test.csv', { type: 'text/csv' });
  await user.upload(document.querySelector('input[type="file"]') as HTMLInputElement, file);
  return file;
};

// Select a file on the upload step and advance to the validation step
const completeUploadStep = async (user: User) => {
  await selectCsvFile(user);
  const uploadButton = screen.getByRole('button', { name: /upload & continue/i });
  await waitFor(() => expect(uploadButton).toBeEnabled());
  await user.click(uploadButton);
  await screen.findByText('Data Validation');
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
    await user.type(sessionNameInput, 'x');
    await user.clear(sessionNameInput);
    
    expect(await screen.findByText('Please enter a session name')).toBeInTheDocument();

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

    // Step 1 -> Step 2: File Upload
    await completeSessionStep(user, 'Test Import Session');
    expect(mockCreateSession).toHaveBeenCalledWith({
      session_name: 'Test Import Session',
      import_type: 'assets',
      template_path: undefined,
    });
    expect(screen.getByText(/Click or drag CSV file/)).toBeInTheDocument();

    // Go back to previous step
    await user.click(screen.getByRole('button', { name: 'Back' }));

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

    await user.click(screen.getByText('Browse Templates'));

    await waitFor(() => {
      expect(screen.getByTestId('template-manager')).toBeInTheDocument();
    });

    // Select a template
    await user.click(screen.getByText('Select Template'));

    // Template manager should close and the template should fill the form
    await waitFor(() => {
      expect(screen.queryByTestId('template-manager')).not.toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText('Select or enter template name')).toHaveValue('Test Template');

    // The selected template is sent with the session request
    await completeSessionStep(user);
    expect(mockCreateSession).toHaveBeenCalledWith({
      session_name: 'Test Session',
      import_type: 'assets',
      template_path: 'Test Template',
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

    await completeSessionStep(user);

    // Upload is disabled until a file is selected
    const uploadButton = screen.getByRole('button', { name: /upload & continue/i });
    expect(uploadButton).toBeDisabled();

    await selectCsvFile(user);

    await waitFor(() => {
      expect(uploadButton).toBeEnabled();
    });
    expect(screen.getByText('File Details')).toBeInTheDocument();

    await user.click(uploadButton);

    await waitFor(() => {
      expect(mockUploadFile).toHaveBeenCalledWith(1, 'test.csv');
      expect(screen.getByText('Data Validation')).toBeInTheDocument();
    });
  });

  test('should reject files without a .csv extension', async () => {
    const user = userEvent.setup({ applyAccept: false });
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    await completeSessionStep(user);

    const file = new File(['not csv'], 'data.txt', { type: 'text/plain' });
    await user.upload(document.querySelector('input[type="file"]') as HTMLInputElement, file);

    expect(await screen.findByText('Invalid File')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /upload & continue/i })).toBeDisabled();
  });

  test('should validate data and show results', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    await completeSessionStep(user);
    await completeUploadStep(user);

    await user.click(screen.getByRole('button', { name: /validate data/i }));

    // Should show validation results
    await waitFor(() => {
      expect(screen.getByText('Validation Results')).toBeInTheDocument();
    });
    expect(mockValidateData).toHaveBeenCalledWith(1);
    expect(screen.getByText('Validation Successful')).toBeInTheDocument();
    expect(screen.getByText('Data Summary').closest('.ant-card')).toHaveTextContent(
      'Total Items: 2Errors: 0Warnings: 1'
    );
    expect(screen.getByText('Description is empty')).toBeInTheDocument();
    expect(screen.getByText('Asset 1')).toBeInTheDocument();
  });

  test('should configure processing options', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    await completeSessionStep(user);
    await completeUploadStep(user);
    await user.click(screen.getByRole('button', { name: /validate data/i }));
    await screen.findByText('Validation Results');
    await user.click(screen.getByRole('button', { name: /continue to configuration/i }));

    // Should be on configuration step
    await waitFor(() => {
      expect(screen.getByText('Processing Configuration')).toBeInTheDocument();
      expect(screen.getByText('Configure Import Options')).toBeInTheDocument();
    });

    // Toggle options and confirm they reach startProcessing
    await user.click(screen.getByText(/Skip existing assets/));
    await user.click(screen.getByText(/Update existing assets/));
    await user.click(screen.getByText(/Create missing parent folders/));
    await user.click(screen.getByRole('button', { name: /start import/i }));

    await waitFor(() => {
      expect(mockStartProcessing).toHaveBeenCalledWith(1, {
        skip_existing: true,
        update_existing: true,
        create_missing_parents: false,
        validation_mode: 'strict',
      });
    });
  });

  test('should start import processing', async () => {
    const user = userEvent.setup();
    render(
      <BulkImportWizard
        visible={true}
        onClose={mockOnClose}
        onComplete={mockOnComplete}
      />
    );

    await completeSessionStep(user);
    await completeUploadStep(user);
    await user.click(screen.getByRole('button', { name: /validate data/i }));
    await screen.findByText('Validation Results');
    await user.click(screen.getByRole('button', { name: /continue to configuration/i }));
    await screen.findByText('Processing Configuration');

    await user.click(screen.getByRole('button', { name: /start import/i }));

    await waitFor(() => {
      expect(mockStartProcessing).toHaveBeenCalledWith(1, {
        skip_existing: false,
        update_existing: false,
        create_missing_parents: true,
        validation_mode: 'strict',
      });
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
    await completeSessionStep(user);
    await completeUploadStep(user);
    
    await user.click(screen.getByRole('button', { name: /validate data/i }));

    // Should show validation errors
    await waitFor(() => {
      expect(screen.getByText('Validation Issues Found')).toBeInTheDocument();
      expect(screen.getByText('Name is required')).toBeInTheDocument();
      expect(screen.getByText('Invalid asset type')).toBeInTheDocument();
    });

    // Continue button should be disabled
    const continueButton = screen.getByRole('button', { name: /continue to configuration/i });
    expect(continueButton).toBeDisabled();
  });
});