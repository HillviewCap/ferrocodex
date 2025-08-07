import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import BulkImportWizard from '../BulkImportWizard';

// Mock the store
vi.mock('../../../store/bulk', () => ({
  default: vi.fn(() => ({
    isLoading: false,
    error: null,
    createSession: vi.fn(),
    uploadFile: vi.fn(),
    validateData: vi.fn(),
    startProcessing: vi.fn(),
    clearError: vi.fn(),
  })),
}));

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock child components
vi.mock('../ImportTemplateManager', () => ({
  default: () => <div data-testid="template-manager">Template Manager</div>,
}));

describe('BulkImportWizard', () => {
  const mockProps = {
    visible: true,
    onClose: vi.fn(),
    onComplete: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders wizard with initial step', () => {
    render(<BulkImportWizard {...mockProps} />);
    
    expect(screen.getByText('Bulk Import Wizard')).toBeInTheDocument();
    expect(screen.getByText('Import Session Configuration')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter a descriptive name for this import session')).toBeInTheDocument();
  });

  test('validates session name input', async () => {
    const user = userEvent.setup();
    render(<BulkImportWizard {...mockProps} />);
    
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    const nextButton = screen.getByRole('button', { name: /next/i });
    
    // Try to proceed without entering session name
    await user.click(nextButton);
    
    expect(screen.getByText('Please enter a session name')).toBeInTheDocument();
  });

  test('validates session name length', async () => {
    const user = userEvent.setup();
    render(<BulkImportWizard {...mockProps} />);
    
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    
    // Enter a short name
    await user.type(sessionNameInput, 'AB');
    await user.tab(); // Trigger validation
    
    await waitFor(() => {
      expect(screen.getByText('Session name must be at least 3 characters')).toBeInTheDocument();
    });
  });

  test('accepts valid session configuration', async () => {
    const user = userEvent.setup();
    const mockCreateSession = vi.fn().mockResolvedValue({ id: 1 });
    
    vi.mocked(require('../../../store/bulk').default).mockReturnValue({
      isLoading: false,
      error: null,
      createSession: mockCreateSession,
      uploadFile: vi.fn(),
      validateData: vi.fn(),
      startProcessing: vi.fn(),
      clearError: vi.fn(),
    });

    render(<BulkImportWizard {...mockProps} />);
    
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    const importTypeSelect = screen.getByRole('combobox');
    const nextButton = screen.getByRole('button', { name: /next/i });
    
    await user.type(sessionNameInput, 'Test Import Session');
    await user.click(importTypeSelect);
    await user.click(screen.getByText('Assets'));
    await user.click(nextButton);
    
    await waitFor(() => {
      expect(mockCreateSession).toHaveBeenCalledWith({
        session_name: 'Test Import Session',
        import_type: 'assets',
        template_path: undefined,
      });
    });
  });

  test('displays file upload step after session creation', async () => {
    const user = userEvent.setup();
    const mockCreateSession = vi.fn().mockResolvedValue({ id: 1 });
    
    vi.mocked(require('../../../store/bulk').default).mockReturnValue({
      isLoading: false,
      error: null,
      createSession: mockCreateSession,
      uploadFile: vi.fn(),
      validateData: vi.fn(),
      startProcessing: vi.fn(),
      clearError: vi.fn(),
    });

    render(<BulkImportWizard {...mockProps} />);
    
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    const nextButton = screen.getByRole('button', { name: /next/i });
    
    await user.type(sessionNameInput, 'Test Import Session');
    await user.click(nextButton);
    
    await waitFor(() => {
      expect(screen.getByText('Upload CSV File')).toBeInTheDocument();
      expect(screen.getByText('Click or drag CSV file to this area to upload')).toBeInTheDocument();
    });
  });

  test('opens template manager when browse button is clicked', async () => {
    const user = userEvent.setup();
    render(<BulkImportWizard {...mockProps} />);
    
    const browseButton = screen.getByRole('button', { name: /browse templates/i });
    await user.click(browseButton);
    
    expect(screen.getByTestId('template-manager')).toBeInTheDocument();
  });

  test('handles wizard cancellation', async () => {
    const user = userEvent.setup();
    render(<BulkImportWizard {...mockProps} />);
    
    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    await user.click(cancelButton);
    
    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('navigates between steps correctly', async () => {
    const user = userEvent.setup();
    const mockCreateSession = vi.fn().mockResolvedValue({ id: 1 });
    
    vi.mocked(require('../../../store/bulk').default).mockReturnValue({
      isLoading: false,
      error: null,
      createSession: mockCreateSession,
      uploadFile: vi.fn(),
      validateData: vi.fn(),
      startProcessing: vi.fn(),
      clearError: vi.fn(),
    });

    render(<BulkImportWizard {...mockProps} />);
    
    // Complete first step
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    await user.type(sessionNameInput, 'Test Import Session');
    await user.click(screen.getByRole('button', { name: /next/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Upload CSV File')).toBeInTheDocument();
    });
    
    // Go back to first step
    const backButton = screen.getByRole('button', { name: /back/i });
    await user.click(backButton);
    
    expect(screen.getByText('Import Session Configuration')).toBeInTheDocument();
  });

  test('displays processing options step', async () => {
    const user = userEvent.setup();
    const mockCreateSession = vi.fn().mockResolvedValue({ id: 1 });
    const mockUploadFile = vi.fn().mockResolvedValue({ total_items: 10, valid_items: 10, invalid_items: 0, errors: [] });
    const mockValidateData = vi.fn().mockResolvedValue({ is_valid: true, errors: [], warnings: [], preview_items: [] });
    
    vi.mocked(require('../../../store/bulk').default).mockReturnValue({
      isLoading: false,
      error: null,
      createSession: mockCreateSession,
      uploadFile: mockUploadFile,
      validateData: mockValidateData,
      startProcessing: vi.fn(),
      clearError: vi.fn(),
    });

    render(<BulkImportWizard {...mockProps} />);
    
    // Navigate through all steps to processing options
    const sessionNameInput = screen.getByPlaceholderText('Enter a descriptive name for this import session');
    await user.type(sessionNameInput, 'Test Import Session');
    await user.click(screen.getByRole('button', { name: /next/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Upload CSV File')).toBeInTheDocument();
    });
    
    // Mock file upload (simplified for testing)
    await user.click(screen.getByRole('button', { name: /upload & continue/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Data Validation')).toBeInTheDocument();
    });
    
    await user.click(screen.getByRole('button', { name: /validate data/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Validation Results')).toBeInTheDocument();
    });
    
    await user.click(screen.getByRole('button', { name: /continue to configuration/i }));
    
    await waitFor(() => {
      expect(screen.getByText('Processing Configuration')).toBeInTheDocument();
      expect(screen.getByText('Configure Import Options')).toBeInTheDocument();
    });
  });
});