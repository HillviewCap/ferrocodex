import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import CreateBranchModal from './CreateBranchModal';
import { ConfigurationVersionInfo } from '../types/assets';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('./ExportConfirmationModal', () => ({
  default: ({ visible }: { visible: boolean }) => (visible ? <div data-testid="export-modal" /> : null),
}));

const parentVersion: ConfigurationVersionInfo = {
  id: 3,
  asset_id: 1,
  version_number: 'v3',
  file_name: 'plc-config.json',
  file_size: 2048,
  content_hash: 'abc',
  author: 1,
  author_username: 'engineer',
  notes: '',
  status: 'Approved',
  created_at: '2024-02-10T09:30:00Z',
};

const createdBranch = {
  id: 9,
  name: 'feature-improvements',
  description: null,
  asset_id: 1,
  parent_version_id: 3,
  parent_version_number: 'v3',
  parent_version_status: 'Approved',
  created_by: 1,
  created_by_username: 'engineer',
  created_at: '2024-02-12T00:00:00Z',
  updated_at: '2024-02-12T00:00:00Z',
  is_active: true,
  version_count: 1,
};

describe('CreateBranchModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockResolvedValue(createdBranch);
  });

  it('renders both info alerts, the parent version card and the form', () => {
    render(<CreateBranchModal visible onCancel={vi.fn()} onSuccess={vi.fn()} parentVersion={parentVersion} assetId={1} />);

    expect(screen.getByText('Create a Branch')).toBeInTheDocument();
    expect(screen.getByText('Guidelines')).toBeInTheDocument();
    expect(screen.getByText(/Use descriptive kebab-case names/)).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
    expect(screen.getByText('plc-config.json')).toBeInTheDocument();
    expect(screen.getByText(/engineer ·/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/test-new-parameters/)).toBeInTheDocument();
    expect(screen.getByText('Download configuration after creating branch')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Create Branch/ })).toBeInTheDocument();
  });

  it('creates the branch and reports success', async () => {
    const onSuccess = vi.fn();
    render(<CreateBranchModal visible onCancel={vi.fn()} onSuccess={onSuccess} parentVersion={parentVersion} assetId={1} />);

    fireEvent.change(screen.getByPlaceholderText(/test-new-parameters/), { target: { value: 'feature-improvements' } });
    fireEvent.click(screen.getByRole('button', { name: /Create Branch/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_branch', {
        token: 'test-token',
        name: 'feature-improvements',
        description: null,
        assetId: 1,
        parentVersionId: 3,
      });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalledWith(createdBranch));
  });

  it('opens the export modal after creation when download is requested', async () => {
    render(<CreateBranchModal visible onCancel={vi.fn()} onSuccess={vi.fn()} parentVersion={parentVersion} assetId={1} />);

    fireEvent.change(screen.getByPlaceholderText(/test-new-parameters/), { target: { value: 'feature-improvements' } });
    fireEvent.click(screen.getByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: /Create Branch/ }));

    expect(await screen.findByTestId('export-modal')).toBeInTheDocument();
  });
});
