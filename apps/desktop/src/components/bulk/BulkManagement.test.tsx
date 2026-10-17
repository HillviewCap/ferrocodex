import React from 'react';
import { render, screen, waitFor, within } from '@testing-library/react';
import { expect, test, vi, beforeEach, describe } from 'vitest';
import userEvent from '@testing-library/user-event';
import BulkManagement from './BulkManagement';
import { BulkImportSession, BulkImportStatus, BulkImportStats } from '../../types/bulk';

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock the bulk store
const mockLoadSessions = vi.fn();
const mockLoadStats = vi.fn();
const mockDeleteSession = vi.fn();
const mockClearError = vi.fn();

let mockSessions: BulkImportSession[] = [];
let mockStats: BulkImportStats | null = null;

vi.mock('../../store/bulk', () => ({
  default: () => ({
    sessions: mockSessions,
    stats: mockStats,
    isLoading: false,
    error: null,
    loadSessions: mockLoadSessions,
    loadStats: mockLoadStats,
    deleteSession: mockDeleteSession,
    clearError: mockClearError,
  }),
}));

// Mock child components to simplify testing
vi.mock('./BulkImportWizard', () => ({
  default: ({ visible, onClose, onComplete }: any) => 
    visible ? (
      <div data-testid="bulk-import-wizard">
        <button onClick={onClose}>Close Wizard</button>
        <button onClick={() => onComplete(1)}>Complete Import</button>
      </div>
    ) : null,
}));

vi.mock('./ImportTemplateManager', () => ({
  default: ({ visible, onClose }: any) => 
    visible ? (
      <div data-testid="import-template-manager">
        <button onClick={onClose}>Close Template Manager</button>
      </div>
    ) : null,
}));

vi.mock('./WorkflowDashboard', () => ({
  default: () => <div data-testid="workflow-dashboard">Workflow Dashboard</div>,
}));

vi.mock('./BulkProgressTracker', () => ({
  default: ({ sessionId }: any) => 
    <div data-testid="bulk-progress-tracker">Progress Tracker for session {sessionId}</div>,
}));

vi.mock('./WorkflowIntegrationPanel', () => ({
  default: ({ sessionId }: any) => 
    <div data-testid="workflow-integration-panel">Integration Panel for session {sessionId}</div>,
}));

// Mock data
const testSessions: BulkImportSession[] = [
  {
    id: 1,
    session_name: 'Test Import 1',
    import_type: 'assets',
    total_items: 100,
    processed_items: 50,
    failed_items: 5,
    status: 'Processing' as BulkImportStatus,
    template_path: 'template1.csv',
    error_log: null,
    created_by: 1,
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T01:00:00Z',
    completed_at: null,
  },
  {
    id: 2,
    session_name: 'Test Import 2',
    import_type: 'configurations',
    total_items: 200,
    processed_items: 200,
    failed_items: 0,
    status: 'Completed' as BulkImportStatus,
    template_path: 'template2.csv',
    error_log: null,
    created_by: 1,
    created_at: '2025-01-02T00:00:00Z',
    updated_at: '2025-01-02T02:00:00Z',
    completed_at: '2025-01-02T02:00:00Z',
  },
];

const testStats: BulkImportStats = {
  total_sessions: 10,
  active_sessions: 2,
  total_items_processed: 1500,
  success_rate: 95.5,
};

describe('BulkManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSessions = testSessions;
    mockStats = testStats;
    mockLoadSessions.mockResolvedValue(testSessions);
    mockLoadStats.mockResolvedValue(testStats);
  });

  test('should render the bulk management interface', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('Bulk Import Management')).toBeInTheDocument();
    });
    
    expect(screen.getByText(/Manage bulk import operations/)).toBeInTheDocument();
  });

  test('should load sessions and stats on mount', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(mockLoadSessions).toHaveBeenCalled();
      expect(mockLoadStats).toHaveBeenCalled();
    });
  });

  test('should display statistics correctly', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('Total Sessions')).toBeInTheDocument();
      expect(screen.getByText('Active Sessions')).toBeInTheDocument();
      expect(screen.getByText('Items Processed')).toBeInTheDocument();
      expect(screen.getByText('Success Rate')).toBeInTheDocument();
      
    });

    // Each Statistic renders its value next to its title
    const statValue = (title: string) =>
      screen.getByText(title).closest('.ant-statistic')!.querySelector('.ant-statistic-content');
    expect(statValue('Total Sessions')).toHaveTextContent('10');
    expect(statValue('Active Sessions')).toHaveTextContent('2');
    expect(statValue('Items Processed')).toHaveTextContent('1,500');
    expect(statValue('Success Rate')).toHaveTextContent('95.5%');
  });

  test('should display sessions in the table', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('Test Import 1')).toBeInTheDocument();
      expect(screen.getByText('Test Import 2')).toBeInTheDocument();
      expect(screen.getByText('Processing')).toBeInTheDocument();
      expect(screen.getByText('Completed')).toBeInTheDocument();
    });
  });

  test('should open bulk import wizard when New Import is clicked', async () => {
    const user = userEvent.setup();
    render(<BulkManagement />);
    
    const newImportButton = await screen.findByText('New Import');
    await user.click(newImportButton);
    
    await waitFor(() => {
      expect(screen.getByTestId('bulk-import-wizard')).toBeInTheDocument();
    });
  });

  test('should open template manager when Templates is clicked', async () => {
    const user = userEvent.setup();
    render(<BulkManagement />);
    
    const templatesButton = await screen.findByText('Templates');
    await user.click(templatesButton);
    
    await waitFor(() => {
      expect(screen.getByTestId('import-template-manager')).toBeInTheDocument();
    });
  });

  test('should switch between tabs correctly', async () => {
    const user = userEvent.setup();
    render(<BulkManagement />);
    
    // Initially on Sessions tab
    expect(screen.getByText('Recent Sessions')).toBeInTheDocument();
    
    // Switch to Progress tab
    const progressTab = screen.getByRole('tab', { name: /progress/i });
    await user.click(progressTab);
    
    await waitFor(() => {
      expect(screen.getByText('No session selected')).toBeInTheDocument();
    });
    
    // Switch to Dashboard tab
    const dashboardTab = screen.getByRole('tab', { name: /dashboard/i });
    await user.click(dashboardTab);
    
    await waitFor(() => {
      expect(screen.getByTestId('workflow-dashboard')).toBeInTheDocument();
    });
  });

  test('should show progress tracker when View is clicked on a session', async () => {
    const user = userEvent.setup();
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('Test Import 1')).toBeInTheDocument();
    });
    
    const viewButtons = screen.getAllByText('View');
    await user.click(viewButtons[0]);
    
    await waitFor(() => {
      expect(screen.getByTestId('bulk-progress-tracker')).toBeInTheDocument();
      expect(screen.getByText('Progress Tracker for session 1')).toBeInTheDocument();
    });
  });

  test('should delete session when confirmed', async () => {
    const user = userEvent.setup();
    mockDeleteSession.mockResolvedValue(undefined);
    
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('Test Import 1')).toBeInTheDocument();
    });
    
    // The delete button is icon-only; use the first session row's
    const firstRow = screen.getByText('Test Import 1').closest('tr')!;
    const deleteButton = firstRow.querySelector('.anticon-delete')!.closest('button')!;
    await user.click(deleteButton);

    // Confirm deletion in the popconfirm
    const popconfirm = await screen.findByRole('tooltip');
    expect(within(popconfirm).getByText('Are you sure you want to delete this session?')).toBeInTheDocument();
    await user.click(within(popconfirm).getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(mockDeleteSession).toHaveBeenCalledWith(1);
    });
  });

  test('should handle wizard completion', async () => {
    const user = userEvent.setup();
    render(<BulkManagement />);
    
    // Open wizard
    const newImportButton = await screen.findByText('New Import');
    await user.click(newImportButton);
    
    // Complete wizard
    const completeButton = await screen.findByText('Complete Import');
    await user.click(completeButton);
    
    await waitFor(() => {
      expect(screen.getByTestId('bulk-progress-tracker')).toBeInTheDocument();
      expect(screen.getByText('Progress Tracker for session 1')).toBeInTheDocument();
    });
  });

  test('should display empty state when no sessions exist', async () => {
    mockSessions = [];
    mockLoadSessions.mockResolvedValue([]);
    
    render(<BulkManagement />);
    
    await waitFor(() => {
      expect(screen.getByText('No import sessions yet')).toBeInTheDocument();
      expect(screen.getByText('Create Your First Import')).toBeInTheDocument();
    });
  });

  test('should calculate and display progress correctly', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      // First session: 50/100 items
      expect(screen.getByText('50 / 100 items')).toBeInTheDocument();
      
      // Second session: 200/200 items
      expect(screen.getByText('200 / 200 items')).toBeInTheDocument();
      
      // Check for success rate text
      const successRates = screen.getAllByText(/Success rate:/i);
      expect(successRates).toHaveLength(2);
    });
  });

  test('should display correct status colors', async () => {
    render(<BulkManagement />);
    
    await waitFor(() => {
      const processingTag = screen.getByText('Processing');
      const completedTag = screen.getByText('Completed');
      
      // Check that tags are rendered (Ant Design adds color classes)
      expect(processingTag.closest('.ant-tag')).toBeInTheDocument();
      expect(completedTag.closest('.ant-tag')).toBeInTheDocument();
    });
  });
});