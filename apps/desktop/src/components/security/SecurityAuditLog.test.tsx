import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import SecurityAuditLog from './SecurityAuditLog';
import useAuthStore from '../../store/auth';
import useSecurityStore from '../../store/security';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const adminUser = { id: 1, username: 'admin', role: 'Administrator' as const, created_at: '2024-01-01', is_active: true };
const engineerUser = { ...adminUser, id: 2, username: 'eng', role: 'Engineer' as const };

const events = [
  {
    id: 101,
    timestamp: '2024-05-01T10:15:00Z',
    eventType: 'asset_creation',
    userId: 1,
    details: 'Created asset PLC-001',
    result: 'success',
  },
  {
    id: 102,
    timestamp: '2024-05-01T11:00:00Z',
    eventType: 'login',
    userId: 2,
    details: 'Failed login attempt for eng',
    result: 'failure',
  },
];

describe('SecurityAuditLog', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', user: adminUser, isAuthenticated: true });
    useSecurityStore.setState({ auditEvents: [], auditFilters: {} });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_audit_events') return events;
      if (cmd === 'export_audit_log') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('fetches events on mount and renders them in the table', async () => {
    render(<SecurityAuditLog autoRefresh={false} />);

    expect(screen.getByText('Security Audit Log')).toBeInTheDocument();
    expect(await screen.findByText('Created asset PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Failed login attempt for eng')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_audit_events', { token: 'test-token', filter: {} });

    expect(screen.getByText('Asset Creation')).toBeInTheDocument();
    expect(screen.getByText('Login')).toBeInTheDocument();
    expect(screen.getByText('SUCCESS')).toBeInTheDocument();
    expect(screen.getByText('FAILURE')).toBeInTheDocument();
    expect(screen.getByText('1-2 of 2 events')).toBeInTheDocument();
  });

  it('opens the details drawer with Descriptions items for the selected event', async () => {
    render(<SecurityAuditLog autoRefresh={false} />);
    await screen.findByText('Created asset PLC-001');

    const viewButtons = screen.getAllByRole('button', { name: 'eye' });
    fireEvent.click(viewButtons[0]);

    expect(await screen.findByText('Audit Event Details')).toBeInTheDocument();
    for (const label of ['Event ID', 'Timestamp', 'Event Type', 'User', 'Result', 'Details']) {
      // Table headers reuse some of these labels; the drawer adds a second copy.
      expect(screen.getAllByText(label).length).toBeGreaterThan(0);
    }
    // Events are sorted newest-first, so the first row is the login failure (id 102).
    // "User 2" appears in both the table row and the drawer.
    expect(screen.getByText('102')).toBeInTheDocument();
    expect(screen.getAllByText('User 2')).toHaveLength(2);
    expect(screen.getAllByText('Failed login attempt for eng')).toHaveLength(2);
  });

  it('opens the export modal and triggers an export', async () => {
    render(<SecurityAuditLog autoRefresh={false} />);
    await screen.findByText('Created asset PLC-001');

    fireEvent.click(screen.getByRole('button', { name: /Export/ }));

    expect(await screen.findByText('Export Audit Log')).toBeInTheDocument();
    expect(screen.getByText('Select export format:')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Export as CSV' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        'export_audit_log',
        expect.objectContaining({ token: 'test-token', format: 'csv' }),
      );
    });
  });

  it('hides the log from non-administrators', () => {
    useAuthStore.setState({ user: engineerUser });
    render(<SecurityAuditLog autoRefresh={false} />);

    expect(screen.getByText('Administrator Access Required')).toBeInTheDocument();
    expect(screen.queryByText('Security Audit Log')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });
});
