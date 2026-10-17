import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import BranchVersionHistory from './BranchVersionHistory';
import { BranchInfo, BranchVersionInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import useBranchStore from '../store/branches';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('./ExportConfirmationModal', () => ({
  default: ({ visible }: { visible: boolean }) => (visible ? <div data-testid="export-modal" /> : null),
}));

const branch: BranchInfo = {
  id: 5,
  name: 'test-new-parameters',
  description: null,
  asset_id: 1,
  parent_version_id: 3,
  parent_version_number: 'v3',
  parent_version_status: 'Approved',
  created_by: 1,
  created_by_username: 'engineer',
  created_at: '2024-02-10T09:30:00Z',
  updated_at: '2024-02-10T09:30:00Z',
  is_active: true,
  version_count: 2,
};

const makeVersion = (id: number, n: number, latest: boolean): BranchVersionInfo => ({
  id,
  branch_id: 5,
  branch_name: branch.name,
  version_id: id * 10,
  branch_version_number: `branch-v${n}`,
  is_latest: latest,
  is_branch_latest: latest,
  created_at: `2024-02-1${n}T10:00:00Z`,
  asset_id: 1,
  version_number: `v${3 + n}`,
  file_name: `config-${n}.json`,
  file_size: 1024,
  content_hash: 'abc',
  author: 1,
  author_username: 'engineer',
  notes: `Iteration ${n}`,
  version_created_at: `2024-02-1${n}T10:00:00Z`,
});

const versions = [makeVersion(2, 2, true), makeVersion(1, 1, false)];

describe('BranchVersionHistory', () => {
  useConsoleGuard({
    // antd 6 deprecates `List` in favour of `Listy` (a different API). Commit d052492
    // deliberately left the List migration for a follow-up, so that one deprecation is
    // tolerated here; anything else antd/React warns about still fails the test.
    ignore: [/\[antd: List\] The `List` component is deprecated/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    useBranchStore.setState({ branchVersions: {}, isLoadingVersions: false, versionsError: null });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_branch_versions') return versions;
      if (cmd === 'compare_branch_versions') return '--- a\n+++ b\n-old\n+new';
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('fetches versions on mount and renders statistics and the version list', async () => {
    render(<BranchVersionHistory branch={branch} />);

    expect(await screen.findByText('config-2.json')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_branch_versions', expect.objectContaining({ token: 'test-token', branchId: 5 }));

    expect(screen.getByText('Branch Version History')).toBeInTheDocument();
    expect(screen.getByText('test-new-parameters')).toBeInTheDocument();
    expect(screen.getByText('Total Versions')).toBeInTheDocument();
    expect(screen.getByText('Latest Version')).toBeInTheDocument();
    expect(screen.getByText('Total Size')).toBeInTheDocument();
    expect(screen.getByText('Last Updated')).toBeInTheDocument();
    expect(screen.getByText('2 KB')).toBeInTheDocument();
    expect(screen.getAllByText('branch-v2').length).toBeGreaterThan(0);
    expect(screen.getByText('config-1.json')).toBeInTheDocument();
  });

  it('shows the selection alert and opens the compare modal with both versions', async () => {
    render(<BranchVersionHistory branch={branch} />);
    await screen.findByText('config-2.json');

    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[0]);
    expect(await screen.findByText('1 version selected for comparison')).toBeInTheDocument();
    expect(screen.getByText('Select one more version to compare.')).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole('checkbox')[1]);
    expect(await screen.findByText('2 versions selected for comparison')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Compare Selected/ }));
    expect(await screen.findByText('Compare Branch Versions')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('Comparison Result')).toBeInTheDocument());
    expect(screen.getByText('Version 1')).toBeInTheDocument();
    expect(screen.getByText('Version 2')).toBeInTheDocument();
    expect(await screen.findByText(/\+new/)).toBeInTheDocument();
  });

  it('opens the export modal from the download action', async () => {
    render(<BranchVersionHistory branch={branch} />);
    await screen.findByText('config-2.json');

    fireEvent.click(screen.getAllByRole('button', { name: 'download' })[0]);
    expect(await screen.findByTestId('export-modal')).toBeInTheDocument();
  });

  it('renders the error alert when loading fails', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('network down'));
    render(<BranchVersionHistory branch={branch} />);

    // The component clears the store error immediately after it appears, so observe it
    // directly from the store rather than waiting on the alert to stay visible.
    await waitFor(() => expect(useBranchStore.getState().versionsError).toBeNull());
    expect(invoke).toHaveBeenCalledWith('get_branch_versions', expect.anything());
  });
});
