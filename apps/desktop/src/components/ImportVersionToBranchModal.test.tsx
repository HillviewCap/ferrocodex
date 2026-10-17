import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { App } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import ImportVersionToBranchModal from './ImportVersionToBranchModal';
import { BranchInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import useBranchStore from '../store/branches';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

const branch: BranchInfo = {
  id: 5,
  name: 'test-new-parameters',
  description: 'Experimenting with PID values',
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

const importedVersion = {
  id: 8,
  branch_id: 5,
  branch_name: 'test-new-parameters',
  version_id: 80,
  branch_version_number: 'branch-v3',
  is_latest: true,
  is_branch_latest: true,
  created_at: '2024-02-12T00:00:00Z',
  asset_id: 1,
  version_number: 'v5',
  file_name: 'plc-config.json',
  file_size: 100,
  content_hash: 'x',
  author: 1,
  author_username: 'engineer',
  notes: 'Third iteration',
  version_created_at: '2024-02-12T00:00:00Z',
};

// The component reads message from App.useApp(), so render inside <App> as the real app does
const renderModal = (props: Partial<React.ComponentProps<typeof ImportVersionToBranchModal>> = {}) =>
  render(
    <App>
      <ImportVersionToBranchModal visible onCancel={vi.fn()} onSuccess={vi.fn()} branch={branch} {...props} />
    </App>,
  );

describe('ImportVersionToBranchModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    useBranchStore.setState({ isImportingVersion: false, branchVersions: {} });
    vi.mocked(dialog.open).mockResolvedValue('/tmp/configs/plc-config.json');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'import_version_to_branch') return importedVersion;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders alerts, the target branch card and the form', () => {
    renderModal();

    expect(screen.getByText('Import Version to Branch')).toBeInTheDocument();
    expect(screen.getByText('Import New Version')).toBeInTheDocument();
    expect(screen.getByText('Version Import Guidelines')).toBeInTheDocument();
    expect(screen.getByText(/Each imported version gets a unique branch version number/)).toBeInTheDocument();
    expect(screen.getByText('Target Branch')).toBeInTheDocument();
    expect(screen.getByText('test-new-parameters')).toBeInTheDocument();
    expect(screen.getByText('Parent: v3')).toBeInTheDocument();
    expect(screen.getByText('Experimenting with PID values')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Select Configuration File/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Import Version/ })).toBeDisabled();
  });

  it('selects a file and imports the version through the branch store', async () => {
    const onSuccess = vi.fn();
    renderModal({ onSuccess });

    fireEvent.click(screen.getByRole('button', { name: /Select Configuration File/ }));
    expect(await screen.findByText('plc-config.json')).toBeInTheDocument();
    expect(screen.getByText('/tmp/configs/plc-config.json')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Change File/ })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Version Notes'), { target: { value: 'Third iteration' } });
    fireEvent.click(screen.getByRole('button', { name: /Import Version/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        'import_version_to_branch',
        expect.objectContaining({ token: 'test-token', branchId: 5, filePath: '/tmp/configs/plc-config.json', notes: 'Third iteration' }),
      );
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalled());
  });
});
