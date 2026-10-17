import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import EnhancedErrorDisplay from './EnhancedErrorDisplay';
import { ProcessedErrorInfo } from '../../utils/errorHandling';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

// "authentication" classifies as High severity, "auth" as the Auth domain, which maps to
// the registered Auth:High recovery actions (Retry Login / Reset Session).
const errorInfo: ProcessedErrorInfo = {
  error: 'Authentication failed: session token rejected',
  severity: 'High',
  domain: 'Auth',
  isRecoverable: true,
  userMessage: 'Your session is no longer valid. Please sign in again.',
  debugMessage: 'token=abc123 rejected by validate_session',
  context: {
    user: { id: 7, role: 'Admin' },
    operation: { name: 'login', component: 'LoginScreen' },
    session: { sessionId: 'sess-42' },
  },
  timestamp: '2024-05-01T10:15:00Z',
  correlationId: 'corr-123',
};

describe('EnhancedErrorDisplay', () => {
  useConsoleGuard();

  it('renders the alert, context Descriptions and recovery actions', () => {
    render(<EnhancedErrorDisplay errorInfo={errorInfo} visible onClose={vi.fn()} />);

    expect(screen.getByText('High Error Detected')).toBeInTheDocument();
    expect(screen.getByText('Your session is no longer valid. Please sign in again.')).toBeInTheDocument();

    expect(screen.getByText('Context Information')).toBeInTheDocument();
    expect(screen.getByText('Operation')).toBeInTheDocument();
    expect(screen.getByText('login')).toBeInTheDocument();
    expect(screen.getByText('Component')).toBeInTheDocument();
    expect(screen.getByText('LoginScreen')).toBeInTheDocument();
    expect(screen.getByText('User Role')).toBeInTheDocument();

    expect(screen.getByText('Suggested Recovery Actions')).toBeInTheDocument();
    expect(screen.getByText('Retry Login')).toBeInTheDocument();
    expect(screen.getByText('Reset Session')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /Start Recovery/ }).length).toBe(2);
  });

  it('exposes technical details to administrators in a collapse with Descriptions', () => {
    render(<EnhancedErrorDisplay errorInfo={errorInfo} visible onClose={vi.fn()} />);

    expect(screen.getByRole('button', { name: /Show Technical Details/ })).toBeInTheDocument();
    fireEvent.click(screen.getByText('Technical Details'));

    for (const label of ['Error ID', 'Timestamp', 'Severity', 'Domain', 'Recoverable', 'User', 'Session']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText('corr-123')).toBeInTheDocument();
    expect(screen.getByText('sess-42')).toBeInTheDocument();
    expect(screen.getByText('7 (Admin)')).toBeInTheDocument();
    expect(screen.getByText('login in LoginScreen')).toBeInTheDocument();
    expect(screen.getByText('Debug Information')).toBeInTheDocument();
    expect(screen.getByText('token=abc123 rejected by validate_session')).toBeInTheDocument();
  });

  it('hides technical details from non-administrators and closes', () => {
    const onClose = vi.fn();
    render(
      <EnhancedErrorDisplay
        errorInfo={{ ...errorInfo, context: { ...errorInfo.context, user: { id: 2, role: 'Engineer' } } }}
        visible
        onClose={onClose}
      />,
    );

    expect(screen.queryByText('Technical Details')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Technical Details/ })).not.toBeInTheDocument();

    // The modal's X icon is also named "Close"; the footer button is the last one
    const closeButtons = screen.getAllByRole('button', { name: 'Close' });
    fireEvent.click(closeButtons[closeButtons.length - 1]);
    expect(onClose).toHaveBeenCalled();
  });
});
