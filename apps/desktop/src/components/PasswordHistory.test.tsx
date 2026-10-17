import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PasswordHistoryComponent from './PasswordHistory';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const history = [
  { id: 2, secret_id: 9, password_hash: '$2b$12$currenthashvalue0123456789abcdef', created_at: '2024-04-01T10:00:00Z' },
  {
    id: 1,
    secret_id: 9,
    password_hash: '$2b$12$retiredhashvalue0123456789abcdef',
    created_at: '2024-01-01T10:00:00Z',
    retired_at: '2024-04-01T10:00:00Z',
  },
];

describe('PasswordHistory', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_password_history') return history;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads history when opened and renders the timeline from items', async () => {
    render(<PasswordHistoryComponent visible onCancel={vi.fn()} secretId={9} secretLabel="PLC admin" />);

    // Space renders the title fragments as separate items
    expect(screen.getByText('Password History -')).toBeInTheDocument();
    expect(screen.getByText('PLC admin')).toBeInTheDocument();
    expect(screen.getByText('Password History Security')).toBeInTheDocument();

    expect(await screen.findByText('Password Change History (2 entries)')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_password_history', { token: 'test-token', secretId: 9 });

    expect(screen.getByText('Current Password')).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('Password #1')).toBeInTheDocument();
    expect(screen.getByText(/Retired:/)).toBeInTheDocument();
    expect(screen.getAllByText(/^Hash: /)).toHaveLength(2);
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(2);

    expect(screen.getByText('Security Summary')).toBeInTheDocument();
    expect(screen.getByText('Total Changes')).toBeInTheDocument();
    expect(screen.getByText('Active Password')).toBeInTheDocument();
    expect(screen.getByText('Last Changed')).toBeInTheDocument();
  });

  it('shows the empty state when there is no history', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    render(<PasswordHistoryComponent visible onCancel={vi.fn()} secretId={9} secretLabel="PLC admin" />);

    expect(await screen.findByText('No Password History')).toBeInTheDocument();
    expect(screen.queryByText('Security Summary')).not.toBeInTheDocument();
  });

  it('does not fetch while hidden', () => {
    render(<PasswordHistoryComponent visible={false} onCancel={vi.fn()} secretId={9} secretLabel="PLC admin" />);
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.queryByText('Password History Security')).not.toBeInTheDocument();
  });
});
