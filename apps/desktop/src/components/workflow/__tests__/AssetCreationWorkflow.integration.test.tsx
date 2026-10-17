/**
 * Asset Creation Workflow Integration Tests
 * 
 * Tests the complete asset creation workflow from start to finish,
 * including integration with hierarchy, metadata, and security systems.
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { message, Modal } from 'antd';
import { invoke } from '@tauri-apps/api/core';

import { AssetCreationWizard } from '../AssetCreationWizard';
import { useWorkflowStore } from '../../../store/workflow';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock Antd message
vi.mock('antd', async () => {
  const actual = await import('antd');
  return {
    ...actual,
    message: {
      error: vi.fn(),
      success: vi.fn(),
      warning: vi.fn(),
    },
  };
});

// Mock workflow store
vi.mock('../../../store/workflow');

describe('AssetCreationWorkflow Integration Tests', () => {
  const mockWorkflowStore = {
    currentWorkflow: null,
    session: null,
    isLoading: false,
    error: null,
    autoSave: {
      enabled: true,
      interval: 30,
      save_in_progress: false,
    },
    lastSaveTime: null,
    startWorkflow: vi.fn(),
    updateStep: vi.fn(),
    nextStep: vi.fn(),
    previousStep: vi.fn(),
    completeWorkflow: vi.fn(),
    cancelWorkflow: vi.fn(),
    validateCurrentStep: vi.fn(),
    canNavigateNext: vi.fn(),
    canNavigatePrevious: vi.fn(),
    saveWorkflowDraft: vi.fn(),
    clearError: vi.fn(),
  };

  beforeEach(() => {
    (useWorkflowStore as any).mockReturnValue(mockWorkflowStore);
    vi.mocked(invoke).mockReset();
    Object.values(mockWorkflowStore).forEach(mock => {
      if (typeof mock === 'function') mock.mockReset();
    });
    mockWorkflowStore.currentWorkflow = null;
    mockWorkflowStore.error = null;
    mockWorkflowStore.lastSaveTime = null;
  });

  afterEach(() => {
    vi.clearAllMocks();
    // Modal.confirm renders outside the React tree, so RTL cleanup doesn't remove it
    Modal.destroyAll();
  });

  describe('Workflow Initialization', () => {
    it('should initialize workflow on component mount', async () => {
      mockWorkflowStore.startWorkflow.mockResolvedValue(undefined);
      
      render(<AssetCreationWizard />);

      await waitFor(() => {
        expect(mockWorkflowStore.startWorkflow).toHaveBeenCalledWith({
          workflow_type: 'asset_creation',
          initial_data: undefined,
        });
      });
    });

    it('should handle workflow initialization errors', async () => {
      const error = new Error('Failed to initialize workflow');
      mockWorkflowStore.startWorkflow.mockRejectedValue(error);
      
      const onCancel = vi.fn();
      render(<AssetCreationWizard onCancel={onCancel} />);

      await waitFor(() => {
        expect(onCancel).toHaveBeenCalled();
      });
    });

    it('should initialize with provided initial data', async () => {
      const initialData = { asset_type: 'Device' };
      mockWorkflowStore.startWorkflow.mockResolvedValue(undefined);
      
      render(<AssetCreationWizard initialData={initialData} />);

      await waitFor(() => {
        expect(mockWorkflowStore.startWorkflow).toHaveBeenCalledWith({
          workflow_type: 'asset_creation',
          initial_data: initialData,
        });
      });
    });
  });

  describe('Step Navigation', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'asset_type_selection',
        user_id: 1,
        status: 'Active',
        data: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should navigate to next step when validation passes', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });
      mockWorkflowStore.nextStep.mockResolvedValue(undefined);
      mockWorkflowStore.canNavigateNext.mockReturnValue(true);

      render(<AssetCreationWizard />);

      const nextButton = screen.getByRole('button', { name: 'Next' });
      await userEvent.click(nextButton);

      expect(mockWorkflowStore.nextStep).toHaveBeenCalled();
    });

    it('should prevent navigation when validation fails', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: false,
        errors: [{ field: 'asset_name', message: 'Required', code: 'REQUIRED' }],
        warnings: [],
      });
      mockWorkflowStore.canNavigateNext.mockReturnValue(true);

      render(<AssetCreationWizard />);

      const nextButton = screen.getByRole('button', { name: 'Next' });
      await userEvent.click(nextButton);

      expect(mockWorkflowStore.nextStep).not.toHaveBeenCalled();
      expect(message.error).toHaveBeenCalledWith(
        'Please complete all required fields before continuing'
      );
    });

    it('should navigate to previous step', async () => {
      mockWorkflowStore.currentWorkflow.current_step = 'hierarchy_selection';
      mockWorkflowStore.previousStep.mockResolvedValue(undefined);
      mockWorkflowStore.canNavigatePrevious.mockReturnValue(true);

      render(<AssetCreationWizard />);

      const previousButton = screen.getByRole('button', { name: 'Previous' });
      await userEvent.click(previousButton);

      expect(mockWorkflowStore.previousStep).toHaveBeenCalled();
    });
  });

  describe('Asset Type Selection Integration', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'asset_type_selection',
        user_id: 1,
        status: 'Active',
        data: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockWorkflowStore.updateStep.mockResolvedValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });
    });

    it('should update workflow data when asset type is selected', async () => {
      render(<AssetCreationWizard />);

      // Mock the asset type selection step component
      const folderOption = screen.getByText('Folder');
      await userEvent.click(folderOption);

      // Verify workflow update was called
      await waitFor(() => {
        expect(mockWorkflowStore.updateStep).toHaveBeenCalledWith(
          'asset_type_selection',
          expect.objectContaining({
            asset_type: 'Folder',
          })
        );
      });
    });

    it('should validate asset name format', async () => {
      render(<AssetCreationWizard />);

      const nameInput = screen.getByPlaceholderText('Enter a name for this asset');
      await userEvent.type(nameInput, 'Test Asset Name');

      await waitFor(() => {
        expect(mockWorkflowStore.updateStep).toHaveBeenCalledWith(
          'asset_type_selection',
          expect.objectContaining({
            asset_name: 'Test Asset Name',
          })
        );
      });
    });
  });

  describe('Hierarchy Selection Integration', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'hierarchy_selection',
        user_id: 1,
        status: 'Active',
        data: {
          asset_type: 'Device',
          asset_name: 'Test Device',
        },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should require parent folder for device assets', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: false,
        errors: [{ 
          field: 'parent_id', 
          message: 'Devices must be placed inside a folder', 
          code: 'REQUIRED' 
        }],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      expect(screen.getByText('Device Placement Required')).toBeInTheDocument();
    });

    it('should allow root level placement for folder assets', async () => {
      mockWorkflowStore.currentWorkflow.data.asset_type = 'Folder';
      
      render(<AssetCreationWizard />);

      // Should not show the device placement requirement
      expect(screen.queryByText('Device Placement Required')).not.toBeInTheDocument();
    });
  });

  describe('Metadata Configuration Integration', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'metadata_configuration',
        user_id: 1,
        status: 'Active',
        data: {
          asset_type: 'Device',
          asset_name: 'Test Device',
          parent_id: 1,
        },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should require metadata schema selection', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: false,
        errors: [{ 
          field: 'metadata_schema_id', 
          message: 'Metadata schema is required', 
          code: 'REQUIRED' 
        }],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      await waitFor(() => {
        expect(mockWorkflowStore.updateStep).toHaveBeenCalledWith(
          'metadata_configuration',
          expect.objectContaining({
            validation_results: expect.objectContaining({
              is_valid: false,
              errors: expect.arrayContaining([
                expect.objectContaining({ field: 'metadata_schema_id', code: 'REQUIRED' }),
              ]),
            }),
          })
        );
      });
    });

    it('should validate metadata values against schema', async () => {
      mockWorkflowStore.currentWorkflow.data.metadata_schema_id = 1;
      mockWorkflowStore.updateStep.mockResolvedValue({
        is_valid: false,
        errors: [{ 
          field: 'metadata_values', 
          message: 'Required field missing', 
          code: 'REQUIRED' 
        }],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      // Test would interact with metadata form components
      // This is a placeholder for more detailed metadata validation tests
    });
  });

  describe('Security Validation Integration', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'security_validation',
        user_id: 1,
        status: 'Active',
        data: {
          asset_type: 'Device',
          asset_name: 'Test Device',
          parent_id: 1,
          metadata_schema_id: 1,
        },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should require security classification', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: false,
        errors: [{ 
          field: 'security_classification', 
          message: 'Security classification is required', 
          code: 'REQUIRED' 
        }],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      expect(screen.getByText('Security Classification Required')).toBeInTheDocument();
    });

    it('should validate naming compliance', async () => {
      mockWorkflowStore.currentWorkflow.data.security_classification = 'Internal';
      mockWorkflowStore.currentWorkflow.data.asset_name = 'Test<Device>';

      render(<AssetCreationWizard />);

      expect(screen.getByText('Naming Compliance Issue')).toBeInTheDocument();
      expect(screen.getByText('Remove characters: < > : " / \\ | ? *')).toBeInTheDocument();
    });

    it('should show validation passed when all requirements are met', async () => {
      mockWorkflowStore.currentWorkflow.data.security_classification = 'Internal';
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      await waitFor(() => {
        expect(screen.getByText('Security Validation Passed')).toBeInTheDocument();
      });
    });
  });

  describe('Workflow Completion', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'review_confirmation',
        user_id: 1,
        status: 'Active',
        data: {
          asset_type: 'Device',
          asset_name: 'Test Device',
          asset_description: 'Test Description',
          parent_id: 1,
          metadata_schema_id: 1,
          metadata_values: { field1: 'value1' },
          security_classification: 'Internal',
        },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockWorkflowStore.canNavigateNext.mockReturnValue(true);
    });

    it('should complete workflow and create asset', async () => {
      const assetId = 123;
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });
      mockWorkflowStore.completeWorkflow.mockResolvedValue(assetId);
      
      const onComplete = vi.fn();
      render(<AssetCreationWizard onComplete={onComplete} />);

      const createButtons = screen.getAllByRole('button', { name: 'Create Asset' });
      const createButton = createButtons[createButtons.length - 1];
      await userEvent.click(createButton);

      await waitFor(() => {
        expect(mockWorkflowStore.completeWorkflow).toHaveBeenCalled();
        expect(onComplete).toHaveBeenCalledWith(assetId);
        expect(message.success).toHaveBeenCalledWith('Asset created successfully!');
      });
    });

    it('should prevent completion when validation fails', async () => {
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: false,
        errors: [{ field: 'test', message: 'Invalid', code: 'INVALID' }],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      const createButtons = screen.getAllByRole('button', { name: 'Create Asset' });
      const createButton = createButtons[createButtons.length - 1];
      await userEvent.click(createButton);

      expect(mockWorkflowStore.completeWorkflow).not.toHaveBeenCalled();
      expect(message.error).toHaveBeenCalledWith('Please complete all required fields');
    });
  });

  describe('Draft Management', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'asset_type_selection',
        user_id: 1,
        status: 'Active',
        data: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should save draft manually', async () => {
      mockWorkflowStore.saveWorkflowDraft.mockResolvedValue(undefined);

      render(<AssetCreationWizard />);

      const saveDraftButton = screen.getByText('Save Draft');
      await userEvent.click(saveDraftButton);

      expect(mockWorkflowStore.saveWorkflowDraft).toHaveBeenCalled();
      await waitFor(() => {
        expect(message.success).toHaveBeenCalledWith('Progress saved');
      });
    });

    it('should handle draft save errors', async () => {
      const error = new Error('Failed to save draft');
      mockWorkflowStore.saveWorkflowDraft.mockRejectedValue(error);

      render(<AssetCreationWizard />);

      const saveDraftButton = screen.getByText('Save Draft');
      await userEvent.click(saveDraftButton);

      await waitFor(() => {
        expect(message.error).toHaveBeenCalledWith('Failed to save progress');
      });
    });

    it('should show auto-save indicator', async () => {
      mockWorkflowStore.autoSave.enabled = true;
      mockWorkflowStore.lastSaveTime = new Date();

      render(<AssetCreationWizard />);

      expect(screen.getByText('Saved')).toBeInTheDocument();
    });
  });

  describe('Workflow Cancellation', () => {
    beforeEach(() => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'asset_type_selection',
        user_id: 1,
        status: 'Active',
        data: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
    });

    it('should show confirmation dialog when cancelling', async () => {
      render(<AssetCreationWizard />);

      const cancelButton = screen.getByRole('button', { name: 'Cancel' });
      await userEvent.click(cancelButton);

      expect(await screen.findByRole('dialog', { name: 'Cancel Asset Creation' })).toBeInTheDocument();
      expect(screen.getByText('Are you sure you want to cancel? All progress will be lost.')).toBeInTheDocument();
    });

    it('should cancel workflow when confirmed', async () => {
      mockWorkflowStore.cancelWorkflow.mockResolvedValue(undefined);
      const onCancel = vi.fn();

      render(<AssetCreationWizard onCancel={onCancel} />);

      const cancelButton = screen.getByRole('button', { name: 'Cancel' });
      await userEvent.click(cancelButton);

      const confirmButton = await screen.findByRole('button', { name: 'Yes, Cancel' });
      await userEvent.click(confirmButton);

      await waitFor(() => {
        expect(mockWorkflowStore.cancelWorkflow).toHaveBeenCalled();
        expect(onCancel).toHaveBeenCalled();
      });
    });
  });

  describe('Error Handling', () => {
    it('should display and clear errors', async () => {
      mockWorkflowStore.error = 'Test error message';
      mockWorkflowStore.clearError = vi.fn();

      render(<AssetCreationWizard />);

      expect(message.error).toHaveBeenCalledWith('Test error message');
      expect(mockWorkflowStore.clearError).toHaveBeenCalled();
    });

    it('should handle step navigation errors', async () => {
      mockWorkflowStore.currentWorkflow = {
        id: 'test-workflow-1',
        workflow_type: 'asset_creation',
        current_step: 'asset_type_selection',
        user_id: 1,
        status: 'Active',
        data: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockWorkflowStore.nextStep.mockRejectedValue(new Error('Navigation failed'));
      mockWorkflowStore.validateCurrentStep.mockReturnValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });
      mockWorkflowStore.canNavigateNext.mockReturnValue(true);

      render(<AssetCreationWizard />);

      const nextButton = screen.getByRole('button', { name: 'Next' });
      await userEvent.click(nextButton);

      await waitFor(() => {
        expect(message.error).toHaveBeenCalledWith('Failed to advance to next step');
      });
    });
  });
});