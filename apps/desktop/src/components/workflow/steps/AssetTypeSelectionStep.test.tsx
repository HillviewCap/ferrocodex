import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { AssetTypeSelectionStep } from './AssetTypeSelectionStep';
import { useWorkflowStore } from '../../../store/workflow';
import { WorkflowState } from '../../../types/workflow';
import { useConsoleGuard } from '../../../test-utils/consoleGuard';

const workflow: WorkflowState = {
  id: 'wf-1',
  workflow_type: 'asset_creation',
  current_step: 'asset_type_selection',
  user_id: 1,
  status: 'active',
  data: {},
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

const stepProps = () => ({
  workflowId: 'wf-1',
  onDataChange: vi.fn(),
  onValidation: vi.fn(),
  onNext: vi.fn(),
  onPrevious: vi.fn(),
});

describe('AssetTypeSelectionStep', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useWorkflowStore.getState().reset();
    useWorkflowStore.setState({ currentWorkflow: workflow });
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'update_workflow_step') {
        return {
          state: { ...workflow, data: { ...workflow.data, ...(args?.stepData as object) } },
          validation: { is_valid: true, errors: [], warnings: [] },
        };
      }
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders both asset type cards and the name/description fields', () => {
    const props = stepProps();
    render(<AssetTypeSelectionStep {...props} data={{}} />);

    expect(screen.getByText('Select Asset Type')).toBeInTheDocument();
    expect(screen.getByText('Folder')).toBeInTheDocument();
    expect(screen.getByText('Device')).toBeInTheDocument();
    expect(screen.getByText(/Contains other assets/)).toBeInTheDocument();
    expect(screen.getByText(/Stores configuration files/)).toBeInTheDocument();
    expect(screen.getByLabelText('Asset Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Description (Optional)')).toBeInTheDocument();

    expect(props.onValidation).toHaveBeenCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'asset_type' })]),
    );
  });

  it('selects a type on card click and persists it to the workflow', async () => {
    const props = stepProps();
    render(<AssetTypeSelectionStep {...props} data={{ asset_name: 'PLC-001' }} />);

    fireEvent.click(screen.getByText('Device'));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_workflow_step', {
        workflowId: 'wf-1',
        stepName: 'asset_type_selection',
        stepData: expect.objectContaining({ asset_type: 'Device', asset_name: 'PLC-001' }),
      });
    });
    expect(await screen.findByText('Selected:')).toBeInTheDocument();
    expect(screen.getByText(/Device - A physical device/)).toBeInTheDocument();
    await waitFor(() => expect(props.onDataChange).toHaveBeenCalledWith(expect.objectContaining({ asset_type: 'Device' })));
    await waitFor(() => expect(props.onValidation).toHaveBeenLastCalledWith(true, []));
  });
});
