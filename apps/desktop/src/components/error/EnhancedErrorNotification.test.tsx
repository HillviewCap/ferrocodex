import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import { notification } from 'antd';
import { showEnhancedErrorNotification } from './EnhancedErrorNotification';
import { ProcessedErrorInfo } from '../../utils/errorHandling';
import { RetryProgress } from './RetryProgressIndicator';
import { CircuitBreakerMetrics } from './CircuitBreakerStatus';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const makeErrorInfo = (correlationId: string): ProcessedErrorInfo => ({
  error: 'Authentication failed: session token rejected',
  severity: 'High',
  domain: 'Auth',
  isRecoverable: true,
  userMessage: 'Your session is no longer valid.',
  debugMessage: 'debug',
  context: {
    user: { id: 7, role: 'Admin' },
    operation: { name: 'login', component: 'LoginScreen' },
  },
  timestamp: '2024-05-01T10:15:00Z',
  correlationId,
});

const retryProgress: RetryProgress = {
  execution_id: 'exec-1',
  operation_name: 'Load assets',
  current_attempt: 2,
  max_attempts: 5,
  attempts: [],
  status: 'running',
  total_duration: 1500,
  retry_limit_exceeded: false,
};

const circuitBreakerMetrics: CircuitBreakerMetrics = {
  state: 'Open',
  total_calls: 10,
  successful_calls: 5,
  failed_calls: 5,
  rejected_calls: 0,
  failure_rate: 0.5,
  state_transitions: 1,
};

// showEnhancedErrorNotification uses React hooks, so it has to run inside a component.
const Harness: React.FC<Parameters<typeof showEnhancedErrorNotification>[0]> = props =>
  showEnhancedErrorNotification(props);

describe('showEnhancedErrorNotification', () => {
  useConsoleGuard();

  afterEach(() => {
    act(() => {
      notification.destroy();
    });
  });

  it('opens a notification with title, description and a View Details action', async () => {
    render(<Harness errorInfo={makeErrorInfo('corr-a')} />);

    expect(await screen.findByText('High Error')).toBeInTheDocument();
    expect(screen.getByText('Auth')).toBeInTheDocument();
    expect(screen.getByText('Your session is no longer valid.')).toBeInTheDocument();
    expect(screen.getByText('Operation: login')).toBeInTheDocument();
    expect(screen.getByText('LoginScreen')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /View Details/ })).toBeInTheDocument();
  });

  it('opens the detailed error modal from the notification', async () => {
    render(<Harness errorInfo={makeErrorInfo('corr-b')} />);

    fireEvent.click(await screen.findByRole('button', { name: /View Details/ }));

    expect(await screen.findByText('High Error Detected')).toBeInTheDocument();
    expect(screen.getByText('Suggested Recovery Actions')).toBeInTheDocument();
  });

  it('shows retry and circuit breaker controls when provided', async () => {
    const onCancelRetry = vi.fn();
    render(
      <Harness
        errorInfo={makeErrorInfo('corr-c')}
        retryProgress={retryProgress}
        circuitBreakerMetrics={circuitBreakerMetrics}
        onCancelRetry={onCancelRetry}
      />,
    );

    expect(await screen.findByText('Attempting retry 2 of 5')).toBeInTheDocument();
    expect(screen.getByText('LoginScreen: Open')).toBeInTheDocument();
    expect(screen.getByText('50% failure rate')).toBeInTheDocument();
    // Accessible name includes the thunderbolt icon label
    expect(screen.getByRole('button', { name: /Open$/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Cancel Retry/ }));
    expect(onCancelRetry).toHaveBeenCalled();
  });
});
