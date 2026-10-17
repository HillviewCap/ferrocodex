import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { SecurityValidationStep } from './SecurityValidationStep';
import { useWorkflowStore } from '../../../store/workflow';
import { WorkflowState } from '../../../types/workflow';
import { useConsoleGuard } from '../../../test-utils/consoleGuard';

const workflow: WorkflowState = {
  id: 'wf-1',
  workflow_type: 'asset_creation',
  current_step: 'security_validation',
  user_id: 1,
  status: 'active',
  data: { asset_type: 'Device', asset_name: 'PLC-001' },
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

describe('SecurityValidationStep', () => {
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

  it('asks for a classification, then passes once one is selected', async () => {
    const props = stepProps();
    render(<SecurityValidationStep {...props} data={{ asset_type: 'Device', asset_name: 'PLC-001' }} />);

    expect(screen.getByText('Security Validation')).toBeInTheDocument();
    expect(screen.getByText('Security Classification Required')).toBeInTheDocument();
    expect(screen.getByText('Please select a security classification for this asset.')).toBeInTheDocument();
    expect(screen.getByText('Naming Compliance')).toBeInTheDocument();
    expect(screen.getByText('Name: PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Security Guidelines')).toBeInTheDocument();
    expect(props.onValidation).toHaveBeenCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'security_classification' })]),
    );

    fireEvent.mouseDown(screen.getByRole('combobox'));
    fireEvent.click(await screen.findByText('Confidential'));

    expect(await screen.findByText('Security Validation Passed')).toBeInTheDocument();
    expect(screen.getByText('Classification: CONFIDENTIAL')).toBeInTheDocument();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_workflow_step', {
        workflowId: 'wf-1',
        stepName: 'security_validation',
        stepData: expect.objectContaining({ security_classification: 'CONFIDENTIAL' }),
      });
    });
    await waitFor(() => expect(props.onValidation).toHaveBeenLastCalledWith(true, []));
  });

  it('reports naming compliance issues with suggestions', () => {
    const props = stepProps();
    render(
      <SecurityValidationStep
        {...props}
        data={{ asset_type: 'Device', asset_name: 'bad:name', security_classification: 'INTERNAL' }}
      />,
    );

    expect(screen.getByText('Naming Compliance Issue')).toBeInTheDocument();
    expect(screen.getAllByText('Contains prohibited characters').length).toBeGreaterThan(0);
    expect(screen.getByText('Suggestions:')).toBeInTheDocument();
    expect(screen.getByText('Remove characters: < > : " / \\ | ? *')).toBeInTheDocument();
    expect(props.onValidation).toHaveBeenLastCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'naming_compliance' })]),
    );
  });
});
