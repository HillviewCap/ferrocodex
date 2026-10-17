import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import WorkflowDashboard from './WorkflowDashboard';
import useBulkImportStore from '../../store/bulk';
import { BulkImportSession, BulkOperationStats } from '../../types/bulk';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('./BulkProgressTracker', () => ({
  default: ({ sessionId }: { sessionId: number }) => <div data-testid="bulk-progress-tracker">session {sessionId}</div>,
}));
vi.mock('./WorkflowIntegrationPanel', () => ({
  default: () => <div data-testid="workflow-integration-panel" />,
}));

const stats: BulkOperationStats = {
  total_sessions: 10,
  active_sessions: 2,
  completed_sessions: 7,
  failed_sessions: 1,
  total_items_processed: 1234,
  average_processing_time: 12.5,
  success_rate: 91.3,
};

const sessions: BulkImportSession[] = [
  {
    id: 7,
    session_name: 'Line 1 import',
    import_type: 'assets',
    total_items: 20,
    processed_items: 10,
    failed_items: 0,
    status: 'Processing',
    template_path: null,
    error_log: null,
    created_by: 1,
    created_at: '2024-01-01T10:00:00Z',
    updated_at: '2024-01-01T10:05:00Z',
    completed_at: null,
  },
  {
    id: 8,
    session_name: 'Archived import',
    import_type: 'assets',
    total_items: 5,
    processed_items: 5,
    failed_items: 0,
    status: 'Completed',
    template_path: null,
    error_log: null,
    created_by: 1,
    created_at: '2024-01-01T08:00:00Z',
    updated_at: '2024-01-01T08:05:00Z',
    completed_at: '2024-01-01T08:10:00Z',
  },
];

describe('WorkflowDashboard', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useBulkImportStore.getState().reset();
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_bulk_import_sessions') return sessions;
      if (cmd === 'get_bulk_operation_stats') return stats;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads data on mount and renders every default widget', async () => {
    render(<WorkflowDashboard />);

    expect(screen.getByText('Workflow Dashboard')).toBeInTheDocument();
    for (const title of ['Overall Statistics', 'Active Sessions', 'Workflow Integration', 'Recent Activity']) {
      expect(screen.getByText(title)).toBeInTheDocument();
    }

    expect(await screen.findByText('Total Sessions')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_bulk_import_sessions');
    expect(invoke).toHaveBeenCalledWith('get_bulk_operation_stats');
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('Items Processed')).toBeInTheDocument();
    expect(screen.getByText('1,234')).toBeInTheDocument();
    expect(screen.getByText('Success Rate')).toBeInTheDocument();
    // Statistic splits the integer and decimal parts into separate spans
    const statistics = Array.from(document.querySelectorAll('.ant-statistic-content-value'));
    expect(statistics.some(el => el.textContent === '91.3')).toBe(true);
    expect(screen.getByText('Avg Processing Time')).toBeInTheDocument();

    // Only active sessions are listed
    expect(screen.getByText('Line 1 import')).toBeInTheDocument();
    expect(screen.queryByText('Archived import')).not.toBeInTheDocument();
    expect(screen.getByText('Processing')).toBeInTheDocument();
    expect(screen.getByTestId('workflow-integration-panel')).toBeInTheDocument();
    expect(screen.getByText('Data available for 2 recent sessions')).toBeInTheDocument();
    expect(screen.getByText(/Last updated: /)).toBeInTheDocument();
  });

  it('opens the session progress drawer from the sessions table', async () => {
    render(<WorkflowDashboard />);
    await screen.findByText('Line 1 import');

    fireEvent.click(screen.getByRole('button', { name: 'View' }));

    expect(await screen.findByText('Session Progress')).toBeInTheDocument();
    expect(screen.getByTestId('bulk-progress-tracker')).toHaveTextContent('session 7');
  });

  it('opens the settings drawer and toggles widget visibility', async () => {
    render(<WorkflowDashboard />);
    await screen.findByText('Total Sessions');

    fireEvent.click(screen.getByRole('button', { name: /Settings/ }));

    expect(await screen.findByText('Dashboard Settings')).toBeInTheDocument();
    expect(screen.getByText('Auto Refresh')).toBeInTheDocument();
    expect(screen.getByText('Enable Auto Refresh')).toBeInTheDocument();
    expect(screen.getByText('Interval (seconds)')).toBeInTheDocument();
    expect(screen.getByText('Widget Visibility')).toBeInTheDocument();
    expect(screen.getByText('Dashboard Layout')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reset to Default Layout' })).toBeInTheDocument();

    // Hide the chart widget: its title disappears from the grid but stays in the drawer
    const switches = screen.getAllByRole('switch');
    fireEvent.click(switches[switches.length - 1]);
    await waitFor(() => expect(screen.getAllByText('Recent Activity')).toHaveLength(1));
    expect(screen.queryByText(/Chart visualization will be implemented/)).not.toBeInTheDocument();
  });

  it('shows the fullscreen toggle and manual refresh feedback', async () => {
    const onToggleFullscreen = vi.fn();
    render(<WorkflowDashboard fullscreen onToggleFullscreen={onToggleFullscreen} />);
    await screen.findByText('Total Sessions');

    fireEvent.click(screen.getByRole('button', { name: /Exit Fullscreen/ }));
    expect(onToggleFullscreen).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: /Refresh/ }));
    expect(await screen.findByText('Dashboard Refreshed')).toBeInTheDocument();
  });
});
