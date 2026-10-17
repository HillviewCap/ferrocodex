import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import WorkflowIntegrationPanel from './WorkflowIntegrationPanel';
import useBulkImportStore from '../../store/bulk';
import { useWorkflowStore } from '../../store/workflow';
import { ProgressStatus } from '../../types/bulk';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const progress: ProgressStatus = {
  session_id: 7,
  total_items: 20,
  processed_items: 10,
  failed_items: 1,
  current_item: 'PLC-011',
  estimated_completion: '45',
  processing_rate: 0.5,
  status: 'Processing',
};

describe('WorkflowIntegrationPanel', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useBulkImportStore.getState().reset();
    useWorkflowStore.setState({ isLoading: false, error: null });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_bulk_import_progress') return { ...progress, status: 'Paused' };
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders health, the integration timeline and controls without a session', () => {
    render(<WorkflowIntegrationPanel />);

    expect(screen.getByText('Workflow Integration')).toBeInTheDocument();
    expect(screen.getByText('Auto-sync ON')).toBeInTheDocument();

    expect(screen.getByText('Workflow Health')).toBeInTheDocument();
    for (const name of ['Bulk Import', 'Asset Creation', 'Database', 'Validation']) {
      expect(screen.getByText(name)).toBeInTheDocument();
    }

    // Timeline items
    expect(screen.getByText('Integration Status')).toBeInTheDocument();
    expect(screen.getByText('Bulk Import Session')).toBeInTheDocument();
    expect(screen.getByText('No active session')).toBeInTheDocument();
    expect(screen.getByText('Asset Creation Workflows')).toBeInTheDocument();
    expect(screen.getByText('Workflow Coordination')).toBeInTheDocument();
    expect(screen.getByText('Not synchronized')).toBeInTheDocument();
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(3);

    expect(screen.getByText('Integration Controls')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Sync Now/ })).toBeInTheDocument();
    expect(screen.queryByText('Progress Summary')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('renders the progress summary for a processing session and syncs on demand', async () => {
    useBulkImportStore.setState({ currentProgress: progress });
    render(<WorkflowIntegrationPanel sessionId={7} />);

    // Auto-sync fires immediately for a session id
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_bulk_import_progress', { sessionId: 7 }));
    // Shown in the card header and in the Workflow Coordination timeline item
    await waitFor(() => expect(screen.getAllByText(/Last sync: /)).toHaveLength(2));

    expect(screen.getByText('Progress Summary')).toBeInTheDocument();
    expect(screen.getByText('Total')).toBeInTheDocument();
    expect(screen.getByText('20')).toBeInTheDocument();
    expect(screen.getByText('Processed')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
    // The sync returned a Paused status, so the bulk import health degrades to warning
    expect(screen.getByText('Paused - 10/20')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Open Dashboard/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Sync Now/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });

  it('renders the compact variant with badges and progress', () => {
    useBulkImportStore.setState({ currentProgress: progress });
    render(<WorkflowIntegrationPanel compact />);

    expect(screen.getByText('Workflow Integration')).toBeInTheDocument();
    for (const label of ['Import', 'Assets', 'DB', 'Valid']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText('10/20 items')).toBeInTheDocument();
    expect(screen.getByText('Auto-sync ON')).toBeInTheDocument();
    expect(screen.queryByText('Integration Status')).not.toBeInTheDocument();
  });

  it('surfaces store errors in a closable alert with retry', () => {
    useBulkImportStore.setState({ error: 'bulk import backend offline' });
    render(<WorkflowIntegrationPanel showControls={false} />);

    expect(screen.getByText('Integration Issues Detected')).toBeInTheDocument();
    expect(screen.getByText('bulk import backend offline')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
    expect(screen.queryByText('Integration Controls')).not.toBeInTheDocument();
  });
});
