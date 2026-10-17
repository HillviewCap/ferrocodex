import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Modal } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import { AssetCreationWizard } from './AssetCreationWizard';
import { useWorkflowStore } from '../../store/workflow';
import { WorkflowState } from '../../types/workflow';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

// Each step is a full screen with its own store wiring and tests; stub them so this test
// covers the wizard's Steps `items`, Spin `description` and navigation chrome.
vi.mock('./steps/AssetTypeSelectionStep', () => ({ AssetTypeSelectionStep: () => <div data-testid="step-asset-type" /> }));
vi.mock('./steps/HierarchySelectionStep', () => ({ HierarchySelectionStep: () => <div data-testid="step-hierarchy" /> }));
vi.mock('./steps/MetadataConfigurationStep', () => ({ MetadataConfigurationStep: () => <div data-testid="step-metadata" /> }));
vi.mock('./steps/SecurityValidationStep', () => ({ SecurityValidationStep: () => <div data-testid="step-security" /> }));
vi.mock('./steps/ReviewConfirmationStep', () => ({ ReviewConfirmationStep: () => <div data-testid="step-review" /> }));

const workflow = (overrides: Partial<WorkflowState> = {}): WorkflowState => ({
  id: 'wf-1',
  workflow_type: 'asset_creation',
  current_step: 'asset_type_selection',
  user_id: 1,
  status: 'active',
  data: {},
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  ...overrides,
});

describe('AssetCreationWizard', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useWorkflowStore.getState().reset();
  });

  afterEach(() => {
    // Modal.confirm renders outside the React tree, so RTL cleanup doesn't remove it
    Modal.destroyAll();
  });

  it('renders the workflow steps from items with titles and descriptions', () => {
    useWorkflowStore.setState({ currentWorkflow: workflow() });
    render(<AssetCreationWizard />);

    expect(screen.getByText('Create New Asset')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save Draft/ })).toBeInTheDocument();

    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Asset Type', 'Location', 'Metadata', 'Security', 'Review']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const description of [
      'Choose the type of asset to create',
      'Select where to place this asset in the hierarchy',
      'Configure asset metadata and properties',
      'Set security classification and validate compliance',
      'Review and confirm asset creation',
    ]) {
      expect(steps.getByText(description)).toBeInTheDocument();
    }

    expect(screen.getByTestId('step-asset-type')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('enables Next once the step reports valid data and shows Create Asset on the last step', () => {
    useWorkflowStore.setState({
      currentWorkflow: workflow({
        current_step: 'hierarchy_selection',
        data: { asset_type: 'Folder', asset_name: 'Plant', validation_results: { is_valid: true, errors: [], warnings: [] } },
      }),
    });
    const { rerender } = render(<AssetCreationWizard />);

    expect(screen.getByTestId('step-hierarchy')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled();

    useWorkflowStore.setState({ currentWorkflow: workflow({ current_step: 'review_confirmation' }) });
    rerender(<AssetCreationWizard />);
    expect(screen.getByTestId('step-review')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Asset' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Next' })).not.toBeInTheDocument();
  });

  it('asks for confirmation before cancelling', async () => {
    useWorkflowStore.setState({ currentWorkflow: workflow() });
    render(<AssetCreationWizard />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    // Modal.confirm renders into document.body; antd may keep a leaving copy briefly
    expect((await screen.findAllByText('Cancel Asset Creation')).length).toBeGreaterThan(0);
    expect(screen.getAllByText('Are you sure you want to cancel? All progress will be lost.').length).toBeGreaterThan(0);
    expect(screen.getAllByRole('button', { name: 'Continue Working' }).length).toBeGreaterThan(0);
  });

  it('shows the initializing spinner while the workflow starts', () => {
    // Keep the start call pending so the component stays in its loading branch
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
    render(<AssetCreationWizard />);

    expect(screen.getByText('Initializing asset creation workflow...')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('start_asset_creation_workflow', {
      workflowType: 'asset_creation',
      initialData: undefined,
    });
  });
});
