import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import ArchiveConfirmationModal from './ArchiveConfirmationModal';
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
  status: 'Approved',
  created_at: '2024-02-10T09:30:00Z',
};

describe('ArchiveConfirmationModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it('renders the warning alert title, version details and reason field', () => {
    render(<ArchiveConfirmationModal visible onCancel={vi.fn()} onSuccess={vi.fn()} version={version} token="t" />);

    expect(screen.getByText('Archive Configuration Version')).toBeInTheDocument();
    expect(screen.getAllByText('Archive Version')).toHaveLength(2); // Alert title + OK button
    expect(screen.getByText(/You are about to archive this configuration version/)).toBeInTheDocument();
    expect(screen.getByText('Version Details')).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
    expect(screen.getByText('plc-config.json')).toBeInTheDocument();
    expect(screen.getByText(/Created by engineer on/)).toBeInTheDocument();
    expect(screen.getByText('"Tuned PID loop"')).toBeInTheDocument();
    expect(screen.getByLabelText('Archive Reason (Optional)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Archive Version/ })).toBeInTheDocument();
  });

  it('submits the archive request with the reason', async () => {
    const onSuccess = vi.fn();
    render(<ArchiveConfirmationModal visible onCancel={vi.fn()} onSuccess={onSuccess} version={version} token="t" />);

    fireEvent.change(screen.getByLabelText('Archive Reason (Optional)'), { target: { value: 'Superseded' } });
    fireEvent.click(screen.getByRole('button', { name: /Archive Version/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('archive_version', { token: 't', versionId: 12, archiveReason: 'Superseded' });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalled());
  });

  it('calls onCancel from the cancel button', () => {
    const onCancel = vi.fn();
    render(<ArchiveConfirmationModal visible onCancel={onCancel} onSuccess={vi.fn()} version={version} token="t" />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalled();
  });
});
