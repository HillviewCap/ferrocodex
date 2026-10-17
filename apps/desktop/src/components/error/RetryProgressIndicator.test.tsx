import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import RetryProgressIndicator, { RetryProgress } from './RetryProgressIndicator';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const progress: RetryProgress = {
  execution_id: 'exec-1',
  operation_name: 'Load assets',
  current_attempt: 2,
  max_attempts: 5,
  status: 'running',
  total_duration: 1500,
  retry_limit_exceeded: false,
  attempts: [
    {
      attempt: 1,
      error: { id: 'e1', message: 'Connection refused', severity: 'High', domain: 'Data', timestamp: '2024-05-01T10:00:00Z' },
      timestamp: '2024-05-01T10:00:00Z',
      delay: 500,
      is_final: false,
    },
    {
      attempt: 2,
      error: { id: 'e2', message: 'Timed out', severity: 'Critical', domain: 'Data', timestamp: '2024-05-01T10:00:01Z' },
      timestamp: '2024-05-01T10:00:01Z',
      delay: 0,
      is_final: true,
    },
  ],
};

describe('RetryProgressIndicator', () => {
  useConsoleGuard();

  it('renders the running card with progress and a cancel action', () => {
    const onCancel = vi.fn();
    render(<RetryProgressIndicator progress={progress} onCancel={onCancel} />);

    expect(screen.getByText('Load assets')).toBeInTheDocument();
    expect(screen.getByText('2/5')).toBeInTheDocument();
    expect(screen.getByText('Attempting retry 2 of 5')).toBeInTheDocument();
    expect(screen.getByText('Total duration: 1.50s')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.queryByText('Retry History')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Cancel/ }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('renders the retry history as Timeline items when showDetails is set', () => {
    render(<RetryProgressIndicator progress={progress} showDetails />);

    expect(screen.getByText('Retry History')).toBeInTheDocument();
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(2);
    expect(screen.getByText(/Attempt 1/)).toBeInTheDocument();
    expect(screen.getByText(/Attempt 2/)).toBeInTheDocument();
    expect(screen.getByText('Connection refused')).toBeInTheDocument();
    expect(screen.getByText('Timed out')).toBeInTheDocument();
    expect(screen.getByText('Final')).toBeInTheDocument();
    expect(screen.getByText(/Delayed 0\.5s/)).toBeInTheDocument();
  });

  it('offers a manual retry after a non-exhausted failure', () => {
    const onRetry = vi.fn();
    render(<RetryProgressIndicator progress={{ ...progress, status: 'failed' }} onRetry={onRetry} />);

    expect(screen.getByText('Operation failed')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Retry Operation/ }));
    expect(onRetry).toHaveBeenCalled();
  });

  it('renders the compact variant', () => {
    render(<RetryProgressIndicator progress={{ ...progress, status: 'success' }} compact />);

    expect(screen.getByText('Operation completed successfully')).toBeInTheDocument();
    expect(screen.queryByText('Load assets')).not.toBeInTheDocument();
  });
});
