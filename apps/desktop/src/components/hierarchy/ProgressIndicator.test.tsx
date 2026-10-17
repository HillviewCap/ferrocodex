import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ProgressIndicator, ProgressIndicatorSkeleton } from './ProgressIndicator';
import { BulkOperationProgress } from '../../types/bulkOperations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const progress: BulkOperationProgress = {
  operation_id: 'op-1',
  status: 'processing',
  total_items: 10,
  processed_items: 5,
  failed_items: 1,
  current_item: 'PLC-001',
  estimated_completion: null,
  processing_rate: 3.2,
  errors: [
    { asset_id: 9, asset_name: 'HMI-009', error_type: 'validation', error_message: 'bad', recoverable: false } as never,
  ],
};

describe('ProgressIndicator', () => {
  useConsoleGuard({
    // Pre-existing React warning, not antd: the component renders `<style jsx>` but
    // styled-jsx is not installed, so React sees an unknown boolean attribute. Untouched
    // by the antd migration.
    ignore: [/non-boolean attribute[\s\S]*\bjsx\b/],
  });

  it('renders status, percentage, progress bar and details while processing', () => {
    render(<ProgressIndicator progress={progress} status="processing" />);

    expect(screen.getByText('Processing')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('Processed: 5 / 10')).toBeInTheDocument();
    expect(screen.getByText('Failed: 1')).toBeInTheDocument();
    expect(screen.getByText('Current: PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Rate: 3 items/sec')).toBeInTheDocument();
    // Errors are summarised only once processing is over
    expect(screen.queryByText(/error.* occurred/)).not.toBeInTheDocument();
  });

  it('summarises errors once the operation has finished', () => {
    render(<ProgressIndicator progress={{ ...progress, processed_items: 10 }} status="failed" />);

    expect(screen.getByText('Failed')).toBeInTheDocument();
    expect(screen.getByText('1 error occurred')).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('renders the compact variant and the skeleton', () => {
    const { rerender } = render(<ProgressIndicator progress={progress} status="completed" compact />);
    expect(screen.getByText('Completed')).toBeInTheDocument();
    expect(screen.getByText('5/10')).toBeInTheDocument();

    rerender(<ProgressIndicatorSkeleton />);
    expect(document.querySelector('.ant-spin')).not.toBeNull();
  });
});
