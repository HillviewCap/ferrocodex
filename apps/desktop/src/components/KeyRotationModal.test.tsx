import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import KeyRotationModal from './KeyRotationModal';
import useAuthStore from '../store/auth';
import useKeyRotationStore from '../store/keyRotation';

const counts = (total: number) => ({
  configuration_versions: total,
  vault_secrets: 0,
  standalone_credentials: 0,
  firmware_versions: 0,
  total,
});

const details = (over: Record<string, unknown> = {}) => ({
  remaining: counts(0),
  failures: [],
  users_without_keypair: [],
  users_without_access: [],
  active_analysis_jobs: 0,
  ...over,
});

const status = (inProgress: boolean, detailsOver: Record<string, unknown> = {}) => ({
  in_progress: inProgress,
  active_version: inProgress ? 2 : 1,
  retiring_version: inProgress ? 1 : null,
  details: details(detailsOver),
});

const showRecoveryCode = vi.fn();

const calls = (cmd: string) => vi.mocked(invoke).mock.calls.filter(([c]) => c === cmd);

const renderModal = () => render(<KeyRotationModal open onClose={vi.fn()} />);

describe('KeyRotationModal', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useKeyRotationStore.getState().reset();
    showRecoveryCode.mockReset();
    useAuthStore.setState({
      token: 'tok',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '', is_active: true },
      isAuthenticated: true,
      showRecoveryCode,
    });
  });

  it('renders the preflight and lists users without keypairs', async () => {
    vi.mocked(invoke).mockResolvedValue(
      status(false, { users_without_keypair: [{ id: 5, username: 'legacy-eng', role: 'Engineer' }] })
    );
    renderModal();
    expect(await screen.findByText('legacy-eng (Engineer)')).toBeInTheDocument();
    expect(screen.getByText(/Older backups stay readable with the old key/)).toBeInTheDocument();
    expect(screen.getByText(/Reset password/)).toBeInTheDocument();
  });

  it('disables begin until password and acknowledgement are provided', async () => {
    vi.mocked(invoke).mockResolvedValue(
      status(false, { users_without_keypair: [{ id: 5, username: 'legacy-eng', role: 'Engineer' }] })
    );
    renderModal();
    await screen.findByText('legacy-eng (Engineer)');
    const begin = screen.getByRole('button', { name: 'Begin rotation' });
    expect(begin).toBeDisabled();

    fireEvent.change(screen.getByLabelText('Administrator password'), { target: { value: 'pw' } });
    expect(begin).toBeDisabled();

    fireEvent.click(screen.getByRole('checkbox'));
    await waitFor(() => expect(begin).toBeEnabled());
  });

  it('does not require an acknowledgement when nobody loses access', async () => {
    vi.mocked(invoke).mockResolvedValue(status(false));
    renderModal();
    const begin = await screen.findByRole('button', { name: 'Begin rotation' });
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Administrator password'), { target: { value: 'pw' } });
    await waitFor(() => expect(begin).toBeEnabled());
  });

  it('shows the recovery code and loops continue until finalized', async () => {
    let continues = 0;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_key_rotation_status':
          return status(false);
        case 'begin_data_key_rotation':
          return { recovery_code: 'NEW-CODE', from_version: 1, to_version: 2, users_without_access: [] };
        case 'continue_data_key_rotation': {
          continues += 1;
          const last = continues === 3;
          return {
            from_version: 1,
            to_version: 2,
            processed: 4,
            failed: 0,
            remaining: counts(last ? 0 : 8 - continues * 4),
            failure_count: 0,
            done: last,
            finalized: last,
          };
        }
        default:
          throw new Error(`unexpected ${cmd}`);
      }
    });
    renderModal();
    const begin = await screen.findByRole('button', { name: 'Begin rotation' });
    fireEvent.change(screen.getByLabelText('Administrator password'), { target: { value: 'pw' } });
    await waitFor(() => expect(begin).toBeEnabled());
    fireEvent.click(begin);

    await screen.findByText('Encryption key rotated');
    expect(invoke).toHaveBeenCalledWith('begin_data_key_rotation', {
      token: 'tok',
      password: 'pw',
      acknowledgeUsersWithoutKeys: false,
    });
    expect(showRecoveryCode).toHaveBeenCalledWith('NEW-CODE');
    expect(calls('continue_data_key_rotation')).toHaveLength(3);
  });

  it('lists failures and finalizes with discardUnreadable after confirmation', async () => {
    const failure = {
      id: 1,
      table_name: 'vault_secrets',
      row_id: 9,
      from_version: 1,
      error: 'bad tag',
      attempted_at: 'x',
    };
    let finalized = false;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_key_rotation_status':
          return finalized ? status(false) : status(true, { failures: [failure] });
        case 'finalize_data_key_rotation':
          finalized = true;
          return { retired_version: 1, active_version: 2, discarded: 1, orphaned_files_removed: 2 };
        default:
          throw new Error(`unexpected ${cmd}`);
      }
    });
    renderModal();
    expect(await screen.findByText(/vault_secrets row 9: bad tag/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Finalize and discard unreadable items' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Discard and finalize' }));

    await screen.findByText('Encryption key rotated');
    expect(calls('finalize_data_key_rotation')[0][1]).toEqual({ token: 'tok', discardUnreadable: true });
    expect(screen.getByText('Unreadable items discarded: 1')).toBeInTheDocument();
  });

  it('retries failed items and finishes when they succeed', async () => {
    const failure = {
      id: 1,
      table_name: 'firmware_versions',
      row_id: 4,
      from_version: 1,
      error: 'Firmware file is missing',
      attempted_at: 'x',
    };
    let retried = false;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_key_rotation_status':
          return retried ? status(false) : status(true, { failures: [failure] });
        case 'retry_key_rotation_failures':
          retried = true;
          return 1;
        case 'continue_data_key_rotation':
          return {
            from_version: 1,
            to_version: 2,
            processed: 1,
            failed: 0,
            remaining: counts(0),
            failure_count: 0,
            done: true,
            finalized: true,
          };
        default:
          throw new Error(`unexpected ${cmd}`);
      }
    });
    renderModal();
    expect(await screen.findByText(/firmware_versions row 4: Firmware file is missing/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Retry failed items' }));

    await screen.findByText('Encryption key rotated');
    expect(calls('retry_key_rotation_failures')[0][1]).toEqual({ token: 'tok' });
    expect(calls('continue_data_key_rotation')).toHaveLength(1);
    expect(calls('finalize_data_key_rotation')).toHaveLength(0);
  });

  it('shows warnings returned by finalize', async () => {
    const failure = { id: 1, table_name: 'vault_secrets', row_id: 9, from_version: 1, error: 'bad tag', attempted_at: 'x' };
    let finalized = false;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_key_rotation_status':
          return finalized ? status(false) : status(true, { failures: [failure] });
        case 'finalize_data_key_rotation':
          finalized = true;
          return {
            retired_version: 1,
            active_version: 2,
            discarded: 1,
            orphaned_files_removed: 0,
            already_finalized: false,
            warnings: ['Old firmware files were kept because the database checkpoint was busy'],
          };
        default:
          throw new Error(`unexpected ${cmd}`);
      }
    });
    renderModal();
    fireEvent.click(await screen.findByRole('button', { name: 'Finalize and discard unreadable items' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Discard and finalize' }));

    expect(await screen.findByText(/checkpoint was busy/)).toBeInTheDocument();
  });

  it('shows failures after the loop ends done but not finalized', async () => {
    const failure = {
      id: 1,
      table_name: 'firmware_versions',
      row_id: 3,
      from_version: 1,
      error: 'file missing',
      attempted_at: 'x',
    };
    let batchRun = false;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_key_rotation_status':
          return batchRun
            ? status(true, { remaining: counts(0), failures: [failure] })
            : status(true, { remaining: counts(1) });
        case 'continue_data_key_rotation':
          batchRun = true;
          return {
            from_version: 1,
            to_version: 2,
            processed: 0,
            failed: 1,
            remaining: counts(0),
            failure_count: 1,
            done: true,
            finalized: false,
          };
        default:
          throw new Error(`unexpected ${cmd}`);
      }
    });
    renderModal();
    fireEvent.click(await screen.findByRole('button', { name: 'Resume' }));
    expect(await screen.findByText(/firmware_versions row 3: file missing/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Finalize and discard unreadable items' })).toBeInTheDocument();
    expect(calls('continue_data_key_rotation')).toHaveLength(1);
  });

  it('resumes straight into the progress view when a rotation is in progress', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return status(true, { remaining: counts(7) });
      return {
        from_version: 1,
        to_version: 2,
        processed: 7,
        failed: 0,
        remaining: counts(0),
        failure_count: 0,
        done: true,
        finalized: true,
      };
    });
    renderModal();
    expect(await screen.findByText(/Configuration versions: 7 remaining/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Begin rotation' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    await screen.findByText('Encryption key rotated');
    expect(calls('continue_data_key_rotation')).toHaveLength(1);
  });

  it('blocks beginning while analysis jobs are active', async () => {
    vi.mocked(invoke).mockResolvedValue(status(false, { active_analysis_jobs: 2 }));
    renderModal();
    expect(await screen.findByText(/2 firmware analysis job\(s\)/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Administrator password'), { target: { value: 'pw' } });
    expect(screen.getByRole('button', { name: 'Begin rotation' })).toBeDisabled();
  });

  it('renders nothing for non-administrators', () => {
    useAuthStore.setState({
      user: { id: 2, username: 'eng', role: 'Engineer', created_at: '', is_active: true },
    });
    vi.mocked(invoke).mockResolvedValue(status(false));
    renderModal();
    expect(screen.queryByText('Rotate Encryption Key')).not.toBeInTheDocument();
  });
});
