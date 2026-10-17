import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import BulkProgressTracker from './BulkProgressTracker';
import useBulkImportStore from '../../store/bulk';
import { BulkImportSessionDetails, ProgressStatus } from '../../types/bulk';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const progress = (overrides: Partial<ProgressStatus> = {}): ProgressStatus => ({
  session_id: 7,
  total_items: 20,
  processed_items: 10,
  failed_items: 2,
  current_item: 'PLC-011',
  estimated_completion: '90',
  processing_rate: 4,
  status: 'Processing',
  ...overrides,
});

const session: BulkImportSessionDetails = {
  session: {
    id: 7,
    session_name: 'Line 1 import',
    import_type: 'assets',
    total_items: 20,
    processed_items: 20,
    failed_items: 0,
    status: 'Completed',
    template_path: null,
    error_log: null,
    created_by: 1,
    created_at: '2024-01-01T10:00:00Z',
    updated_at: '2024-01-01T10:05:00Z',
    completed_at: '2024-01-01T10:10:00Z',
  },
  items: [],
  errors: [],
};

describe('BulkProgressTracker', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useBulkImportStore.getState().reset();
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_bulk_import_progress') return progress({ status: 'Paused' });
      if (cmd === 'pause_bulk_import' || cmd === 'resume_bulk_import' || cmd === 'cancel_bulk_import') return undefined;
      if (cmd === 'get_bulk_import_session_details') return session;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('offers to load progress when none is available', async () => {
    render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);

    expect(screen.getByText('No progress data available')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Load Progress' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_bulk_import_progress', { sessionId: 7 }));
    expect(await screen.findByText('Import Progress')).toBeInTheDocument();
  });

  it('renders progress, statistics, current item and controls while processing', () => {
    useBulkImportStore.setState({ currentProgress: progress() });
    render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);

    expect(screen.getByText('Import Progress')).toBeInTheDocument();
    expect(screen.getByText('Processing')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByText('10 of 20 items processed')).toBeInTheDocument();
    for (const title of ['Total Items', 'Processed', 'Failed', 'Success Rate', 'Processing Rate', 'Estimated Completion']) {
      expect(screen.getByText(title)).toBeInTheDocument();
    }
    expect(screen.getByText('4.0 items/sec')).toBeInTheDocument();
    expect(screen.getByText('2m')).toBeInTheDocument();
    expect(screen.getByText('Currently Processing')).toBeInTheDocument();
    expect(screen.getByText('PLC-011')).toBeInTheDocument();
    expect(screen.getByText('2 items failed to process')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Pause/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Cancel/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Resume/ })).not.toBeInTheDocument();
  });

  it('pauses the import from the toolbar', async () => {
    useBulkImportStore.setState({ currentProgress: progress() });
    render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);

    fireEvent.click(screen.getByRole('button', { name: /Pause/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('pause_bulk_import', { sessionId: 7 }));
    expect(await screen.findByText('Import Paused')).toBeInTheDocument();
  });

  it('renders the completion alert and timeline items for a finished session', () => {
    useBulkImportStore.setState({
      currentProgress: progress({ status: 'Completed', processed_items: 20, failed_items: 0, current_item: null }),
      currentSession: session,
    });
    render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);

    expect(screen.getByText('Import Completed Successfully')).toBeInTheDocument();
    expect(screen.getByText('Successfully processed 20 items')).toBeInTheDocument();
    expect(screen.getByText('Import Timeline')).toBeInTheDocument();
    expect(screen.getByText(/Session Created - /)).toBeInTheDocument();
    expect(screen.getByText(/Processing Started - /)).toBeInTheDocument();
    expect(screen.getByText(/^Completed - /)).toBeInTheDocument();
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(3);
    expect(screen.queryByRole('button', { name: /Cancel/ })).not.toBeInTheDocument();
  });

  it('renders the failed and cancelled alerts', () => {
    useBulkImportStore.setState({ currentProgress: progress({ status: 'Failed' }) });
    const { unmount } = render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);
    expect(screen.getByText('Import Failed')).toBeInTheDocument();
    unmount();

    useBulkImportStore.setState({ currentProgress: progress({ status: 'Cancelled' }) });
    render(<BulkProgressTracker sessionId={7} autoRefresh={false} />);
    expect(screen.getByText('Import Cancelled')).toBeInTheDocument();
  });
});
