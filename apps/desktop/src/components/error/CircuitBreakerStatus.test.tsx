import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import CircuitBreakerStatus, { CircuitBreakerMetrics } from './CircuitBreakerStatus';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const metrics: CircuitBreakerMetrics = {
  state: 'Open',
  total_calls: 40,
  successful_calls: 30,
  failed_calls: 8,
  rejected_calls: 2,
  failure_rate: 0.25,
  last_opened_at: '2024-05-01T10:00:00Z',
  state_transitions: 3,
};

describe('CircuitBreakerStatus', () => {
  useConsoleGuard();

  it('renders the full card with state, statistics and history', () => {
    render(<CircuitBreakerStatus serviceName="Database" metrics={metrics} showDetails />);

    expect(screen.getByText('Database Circuit Breaker')).toBeInTheDocument();
    expect(screen.getAllByText('Open').length).toBeGreaterThan(0);
    expect(screen.getByText('Service is failing - requests are blocked')).toBeInTheDocument();

    // Statistic titles and values (styles.content conversion)
    expect(screen.getByText('Total Calls')).toBeInTheDocument();
    expect(screen.getByText('40')).toBeInTheDocument();
    expect(screen.getByText('Successful')).toBeInTheDocument();
    expect(screen.getByText('30')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
    expect(screen.getByText('8')).toBeInTheDocument();
    expect(screen.getByText('Rejected')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();

    expect(screen.getByText('Success Rate')).toBeInTheDocument();
    expect(screen.getByText('Last Opened:')).toBeInTheDocument();
    expect(screen.getByText('Last Closed:')).toBeInTheDocument();
    expect(screen.getByText('Never')).toBeInTheDocument();
    expect(screen.getByText('State transitions: 3')).toBeInTheDocument();

    // Failure-rate circle plus the success-rate bar
    expect(screen.getAllByRole('progressbar').length).toBeGreaterThanOrEqual(2);
  });

  it('hides statistics without showDetails and renders a healthy state', () => {
    render(
      <CircuitBreakerStatus
        serviceName="Vault"
        metrics={{ ...metrics, state: 'Closed', failure_rate: 0 }}
      />,
    );

    expect(screen.getByText('Vault Circuit Breaker')).toBeInTheDocument();
    expect(screen.getByText('Service is healthy - requests flow normally')).toBeInTheDocument();
    expect(screen.queryByText('Total Calls')).not.toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('renders the compact badge with the failure rate when open', () => {
    render(<CircuitBreakerStatus serviceName="Database" metrics={metrics} compact />);

    expect(screen.getByText('Database: Open')).toBeInTheDocument();
    expect(screen.getByText('25% failure rate')).toBeInTheDocument();
    expect(screen.queryByText('Database Circuit Breaker')).not.toBeInTheDocument();
  });
});
