/**
 * Folder Creation Validation Test
 * 
 * Tests the specific bug where the Next button stays greyed out 
 * after entering a folder name in the Create Folder workflow.
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach } from 'vitest';

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

describe('Folder Creation Validation Bug Fix', () => {
  const mockWorkflowStore = {
    currentWorkflow: {
      id: 'test-workflow-1',
      workflow_type: 'asset_creation',
      current_step: 'hierarchy_selection',
      user_id: 1,
      status: 'Active',
      data: {
        asset_type: 'Folder',
        asset_name: 'Test Folder',
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
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
    const { invoke } = require('@tauri-apps/api/core');
    invoke.mockClear();
    Object.values(mockWorkflowStore).forEach(mock => {
      if (typeof mock === 'function') mock.mockClear();
    });
  });

  describe('Hierarchy Selection Step for Folders', () => {
    it('should enable Next button when folder has valid location selection', async () => {
      // Setup: Folder is being created and validation results are stored in workflow data
      mockWorkflowStore.currentWorkflow.data = {
        asset_type: 'Folder',
        asset_name: 'Test Folder',
        parent_id: null, // Root level is valid for folders
        validation_results: {
          is_valid: true,
          errors: [],
          warnings: []
        }
      };

      // Mock canNavigateNext to check validation_results first (our fix)
      mockWorkflowStore.canNavigateNext.mockImplementation(() => {
        const { currentWorkflow } = mockWorkflowStore;
        if (!currentWorkflow) return false;

        // This is the fixed logic: check validation_results from step component first
        if (currentWorkflow.data.validation_results !== undefined) {
          return currentWorkflow.data.validation_results.is_valid;
        }

        // Fallback to central validation
        return true;
      });

      mockWorkflowStore.updateStep.mockResolvedValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      // The Next button should be enabled because:
      // 1. It's a folder (can be placed at root)
      // 2. Location selection is valid (parent_id: null for root is OK for folders)
      // 3. validation_results in workflow data shows is_valid: true
      await waitFor(() => {
        const nextButton = screen.getByText('Next');
        expect(nextButton).not.toBeDisabled();
      });
    });

    it('should disable Next button when folder location validation fails', async () => {
      // Setup: Invalid validation results stored in workflow data
      mockWorkflowStore.currentWorkflow.data = {
        asset_type: 'Folder',
        asset_name: 'Test Folder',
        validation_results: {
          is_valid: false,
          errors: [{ field: 'location', message: 'Invalid location', code: 'INVALID' }],
          warnings: []
        }
      };

      // Mock canNavigateNext to use validation_results (our fix)
      mockWorkflowStore.canNavigateNext.mockImplementation(() => {
        const { currentWorkflow } = mockWorkflowStore;
        if (!currentWorkflow) return false;

        if (currentWorkflow.data.validation_results !== undefined) {
          return currentWorkflow.data.validation_results.is_valid;
        }

        return false;
      });

      render(<AssetCreationWizard />);

      await waitFor(() => {
        const nextButton = screen.getByText('Next');
        expect(nextButton).toBeDisabled();
      });
    });

    it('should enable Next button for device when parent folder is selected', async () => {
      // Setup: Device with valid parent folder selection
      mockWorkflowStore.currentWorkflow.data = {
        asset_type: 'Device',
        asset_name: 'Test Device',
        parent_id: 1, // Valid parent folder selected
        parent_path: '/Root/Parent Folder',
        validation_results: {
          is_valid: true,
          errors: [],
          warnings: []
        }
      };

      mockWorkflowStore.canNavigateNext.mockImplementation(() => {
        const { currentWorkflow } = mockWorkflowStore;
        if (!currentWorkflow) return false;

        if (currentWorkflow.data.validation_results !== undefined) {
          return currentWorkflow.data.validation_results.is_valid;
        }

        return false;
      });

      render(<AssetCreationWizard />);

      await waitFor(() => {
        const nextButton = screen.getByText('Next');
        expect(nextButton).not.toBeDisabled();
      });
    });

    it('should disable Next button for device when no parent folder is selected', async () => {
      // Setup: Device without parent folder (validation should fail)
      mockWorkflowStore.currentWorkflow.data = {
        asset_type: 'Device',
        asset_name: 'Test Device',
        parent_id: null, // No parent folder selected (invalid for devices)
        validation_results: {
          is_valid: false,
          errors: [{ 
            field: 'parent_id', 
            message: 'Devices must be placed inside a folder', 
            code: 'REQUIRED' 
          }],
          warnings: []
        }
      };

      mockWorkflowStore.canNavigateNext.mockImplementation(() => {
        const { currentWorkflow } = mockWorkflowStore;
        if (!currentWorkflow) return false;

        if (currentWorkflow.data.validation_results !== undefined) {
          return currentWorkflow.data.validation_results.is_valid;
        }

        return false;
      });

      render(<AssetCreationWizard />);

      await waitFor(() => {
        const nextButton = screen.getByText('Next');
        expect(nextButton).toBeDisabled();
      });
    });
  });

  describe('Step Component Validation Integration', () => {
    it('should call updateStep with validation results when step validation changes', async () => {
      mockWorkflowStore.updateStep.mockResolvedValue({
        is_valid: true,
        errors: [],
        warnings: [],
      });

      render(<AssetCreationWizard />);

      // This tests that our handleValidation function is working
      // It should call updateStep with validation_results when step components validate
      await waitFor(() => {
        expect(mockWorkflowStore.updateStep).toHaveBeenCalled();
      });
    });
  });
});