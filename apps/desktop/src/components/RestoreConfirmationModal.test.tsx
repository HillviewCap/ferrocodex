import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import RestoreConfirmationModal from './RestoreConfirmationModal';
import { ConfigurationVersionInfo } from '../types/assets';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const version: ConfigurationVersionInfo = {
  id: 12,
  asset_id: 1,
  version_number: 'v3',
  file_name: 'plc-config.json',
  file_size: 2048,
  content_hash: 'abc',
  author: 1,
  author_username: 'engineer',
  notes: 'Tuned PID loop',
  status: 'Archived',
  created_at: '2024-02-10T09:30:00Z',
};

describe('RestoreConfirmationModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it('renders the info alert title, version details and reason field', () => {
    render(<RestoreConfirmationModal visible onCancel={vi.fn()} onSuccess={vi.fn()} version={version} token="t" />);

    expect(screen.getByText('Restore Configuration Version')).toBeInTheDocument();
    expect(screen.getByText('Restore Archived Version')).toBeInTheDocument(); // Alert title
    expect(screen.getByText(/You are about to restore this archived configuration version/)).toBeInTheDocument();
    expect(screen.getByText('Version Details')).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
    expect(screen.getByText('plc-config.json')).toBeInTheDocument();
    expect(screen.getByText('"Tuned PID loop"')).toBeInTheDocument();
    expect(screen.getByLabelText('Restore Reason (Optional)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Restore Version/ })).toBeInTheDocument();
  });

  it('submits the restore request with the reason', async () => {
    const onSuccess = vi.fn();
    render(<RestoreConfirmationModal visible onCancel={vi.fn()} onSuccess={onSuccess} version={version} token="t" />);

    fireEvent.change(screen.getByLabelText('Restore Reason (Optional)'), { target: { value: 'Needed again' } });
    fireEvent.click(screen.getByRole('button', { name: /Restore Version/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('restore_version', { token: 't', versionId: 12, restoreReason: 'Needed again' });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalled());
  });

  it('calls onCancel from the cancel button', () => {
    const onCancel = vi.fn();
    render(<RestoreConfirmationModal visible onCancel={onCancel} onSuccess={vi.fn()} version={version} token="t" />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalled();
  });
});
