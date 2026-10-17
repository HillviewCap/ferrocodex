import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import RecoveryTimeline, { RecoveryOperation } from './RecoveryTimeline';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const operation: RecoveryOperation = {
  operation_id: 'op-1',
  operation_name: 'Reconnect database',
  started_at: '2024-05-01T10:00:00Z',
  status: 'running',
  success_rate: 0.8,
  total_duration: 2500,
  steps: [
    {
      id: 'step-1',
      type: 'retry',
      status: 'completed',
      title: 'Retry connection',
      description: 'Second attempt succeeded',
      timestamp: '2024-05-01T10:00:01Z',
      duration: 450,
      details: { attempt_number: 2, error_message: 'timeout' },
      metadata: { host: 'db-01' },
    },
    {
      id: 'step-2',
      type: 'circuit_breaker',
      status: 'failed',
      title: 'Circuit breaker tripped',
      description: 'Too many failures',
      timestamp: '2024-05-01T10:00:02Z',
      duration: 1200,
    },
    {
      id: 'step-3',
      type: 'manual_intervention',
      status: 'pending',
      title: 'Operator review',
      description: 'Waiting for operator',
      timestamp: '2024-05-01T10:00:03Z',
    },
  ],
};

describe('RecoveryTimeline', () => {
  useConsoleGuard();

  it('renders the operation header and one Timeline item per step', () => {
    render(<RecoveryTimeline operation={operation} />);

    expect(screen.getByText('Reconnect database')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText('80% success')).toBeInTheDocument();
    expect(screen.getByText('Duration: 2.50s')).toBeInTheDocument();

    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(3);
    for (const title of ['Retry connection', 'Circuit breaker tripped', 'Operator review']) {
      expect(screen.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Second attempt succeeded', 'Too many failures', 'Waiting for operator']) {
      expect(screen.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByText('450ms')).toBeInTheDocument();
    expect(screen.getByText('1.20s')).toBeInTheDocument();
  });

  it('expands step details and the metadata collapse', () => {
    render(<RecoveryTimeline operation={operation} showMetadata />);

    fireEvent.click(screen.getByRole('button', { name: 'Show Details' }));
    expect(screen.getByText('Attempt: 2')).toBeInTheDocument();
    expect(screen.getByText('Error: timeout')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Hide Details' })).toBeInTheDocument();

    fireEvent.click(screen.getByText('Metadata'));
    expect(screen.getByText(/"host": "db-01"/)).toBeInTheDocument();
  });

  it('offers retry/skip actions on failed steps and omits details in compact mode', () => {
    const onStepAction = vi.fn();
    const { rerender } = render(<RecoveryTimeline operation={operation} onStepAction={onStepAction} />);

    fireEvent.click(screen.getByRole('button', { name: /Retry/ }));
    expect(onStepAction).toHaveBeenCalledWith('step-2', 'retry');
    fireEvent.click(screen.getByRole('button', { name: 'Skip' }));
    expect(onStepAction).toHaveBeenCalledWith('step-2', 'skip');

    rerender(<RecoveryTimeline operation={operation} compact />);
    expect(screen.queryByRole('button', { name: 'Show Details' })).not.toBeInTheDocument();
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(3);
  });
});
