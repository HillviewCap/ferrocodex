import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import BulkProgressModal from './BulkProgressModal';
import useBulkOperationsStore from '../../store/bulkOperations';
import { BulkOperation, BulkOperationProgress } from '../../types/bulkOperations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const operation = {
  id: 'op-1',
  operation_type: 'move',
  asset_ids: [1, 2, 3],
  status: 'processing',
  created_at: '2024-01-01T00:00:00Z',
} as unknown as BulkOperation;

const progress = (overrides: Partial<BulkOperationProgress> = {}): BulkOperationProgress => ({
  operation_id: 'op-1',
  status: 'processing',
  total_items: 3,
  processed_items: 1,
  failed_items: 0,
  current_item: 'PLC-002',
  estimated_completion: '00:00:10',
  processing_rate: 2.5,
  errors: [],
  ...overrides,
});

// The modal polls progress on open; the mock returns whatever was last seeded so the
// seeded status is not overwritten by the first poll.
let seeded: BulkOperationProgress = progress();

const seed = (p: BulkOperationProgress) => {
  seeded = p;
  useBulkOperationsStore.setState(state => ({
    ui: { ...state.ui, currentOperation: operation, showProgressModal: true },
    operationProgress: new Map([[operation.id, p]]),
    error: null,
  }));
};

// The error list inside "Operation Errors" uses antd `List` (deprecated in antd 6.6,
// explicitly deferred by the migration commit), so these cases use progress without
// per-item errors.
describe('BulkProgressModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useBulkOperationsStore.setState(state => ({
      ui: { ...state.ui, currentOperation: null, showProgressModal: false },
      operationProgress: new Map(),
      error: null,
      isLoading: false,
    }));
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_bulk_operation_progress') return seeded;
      if (cmd === 'cancel_bulk_operation') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders nothing without a current operation', () => {
    const { container } = render(<BulkProgressModal />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByText('Bulk Operation Progress')).not.toBeInTheDocument();
  });

  it('renders operation details, progress and statistics for an active operation', async () => {
    seed(progress());
    render(<BulkProgressModal />);

    expect(await screen.findByText('Bulk Operation Progress')).toBeInTheDocument();
    expect(screen.getByText('Processing')).toBeInTheDocument();
    expect(screen.getByText('Operation Type:')).toBeInTheDocument();
    expect(screen.getByText('move')).toBeInTheDocument();
    expect(screen.getByText('op-1')).toBeInTheDocument();
    expect(screen.getByText('3 selected')).toBeInTheDocument();
    expect(screen.getByText('33% (1/3)')).toBeInTheDocument();
    expect(screen.getByText('Processing: PLC-002')).toBeInTheDocument();
    expect(screen.getByText('2.5 items/sec')).toBeInTheDocument();
    expect(screen.getByText('ETA:')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Cancel Operation/ })).toBeEnabled();
    expect(screen.getByRole('button', { name: /Close/ })).toBeInTheDocument();
  });

  it('shows the completion alert and a Done button when finished', async () => {
    seed(progress({ status: 'completed', processed_items: 3 }));
    render(<BulkProgressModal />);

    expect(await screen.findByText('Operation Completed Successfully')).toBeInTheDocument();
    expect(screen.getByText('Processed 3 items with 0 failures.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Cancel Operation/ })).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: /Done/ }));
    await waitFor(() => expect(useBulkOperationsStore.getState().ui.showProgressModal).toBe(false));
  });

  it('shows failure and cancellation alerts', async () => {
    seed(progress({ status: 'failed' }));
    const { unmount } = render(<BulkProgressModal />);
    expect(await screen.findByText('Operation Failed')).toBeInTheDocument();
    unmount();

    seed(progress({ status: 'cancelled' }));
    render(<BulkProgressModal />);
    expect(await screen.findByText('Operation Cancelled')).toBeInTheDocument();
  });
});
