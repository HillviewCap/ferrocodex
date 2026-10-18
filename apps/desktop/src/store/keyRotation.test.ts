import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import useKeyRotationStore from './keyRotation';
import useAuthStore from './auth';
import type { KeyRotationProgress } from '../types/keyRotation';

const counts = (total: number) => ({
  configuration_versions: total,
  vault_secrets: 0,
  standalone_credentials: 0,
  firmware_versions: 0,
  total,
});

const progress = (over: Partial<KeyRotationProgress>): KeyRotationProgress => ({
  from_version: 1,
  to_version: 2,
  processed: 0,
  failed: 0,
  remaining: counts(0),
  failure_count: 0,
  done: false,
  finalized: false,
  ...over,
});

const idleStatus = { in_progress: false, active_version: 2, retiring_version: null, details: null };

const continueCalls = () =>
  vi.mocked(invoke).mock.calls.filter(([c]) => c === 'continue_data_key_rotation');

describe('keyRotation store', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useKeyRotationStore.getState().reset();
    useAuthStore.setState({ keyRotationInProgress: false });
  });

  it('runUntilDone loops continue until done and accumulates processed', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return idleStatus;
      const n = continueCalls().length;
      if (n < 3) return progress({ processed: 5, remaining: counts(20 - n * 5) });
      return progress({ processed: 2, done: true, finalized: true });
    });

    const last = await useKeyRotationStore.getState().runUntilDone('tok');

    expect(continueCalls()).toHaveLength(3);
    expect(continueCalls()[0][1]).toEqual({ token: 'tok' });
    expect(last?.finalized).toBe(true);
    expect(useKeyRotationStore.getState().processedTotal).toBe(12);
    expect(useKeyRotationStore.getState().isRunning).toBe(false);
  });

  it('stops on done without finalizing (failures present)', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return { ...idleStatus, in_progress: true };
      return progress({ failed: 1, failure_count: 1, done: true });
    });
    const last = await useKeyRotationStore.getState().runUntilDone('tok');
    expect(continueCalls()).toHaveLength(1);
    expect(last?.done).toBe(true);
    expect(last?.finalized).toBe(false);
  });

  it('stops on error and records it', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return idleStatus;
      if (continueCalls().length === 1) return progress({ processed: 1, remaining: counts(5) });
      throw 'No key rotation is in progress';
    });

    await useKeyRotationStore.getState().runUntilDone('tok');

    expect(continueCalls()).toHaveLength(2);
    expect(useKeyRotationStore.getState().error).toContain('No key rotation is in progress');
    expect(useKeyRotationStore.getState().isRunning).toBe(false);
  });

  it('stops after the current batch when stop() is called', async () => {
    let release: () => void = () => {};
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return idleStatus;
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      return progress({ processed: 1, remaining: counts(9) });
    });

    const run = useKeyRotationStore.getState().runUntilDone('tok');
    await vi.waitFor(() => expect(useKeyRotationStore.getState().isRunning).toBe(true));
    useKeyRotationStore.getState().stop();
    release();
    await run;

    expect(continueCalls()).toHaveLength(1);
    expect(useKeyRotationStore.getState().isRunning).toBe(false);
  });

  it('begin passes arguments and flags the auth state', async () => {
    vi.mocked(invoke).mockResolvedValue({
      recovery_code: 'CODE',
      from_version: 1,
      to_version: 2,
      users_without_access: [],
    });
    const response = await useKeyRotationStore.getState().begin('tok', 'pw', true);
    expect(invoke).toHaveBeenCalledWith('begin_data_key_rotation', {
      token: 'tok',
      password: 'pw',
      acknowledgeUsersWithoutKeys: true,
    });
    expect(response.recovery_code).toBe('CODE');
    expect(useAuthStore.getState().keyRotationInProgress).toBe(true);
  });

  it('finalize sends discardUnreadable and clears the auth flag', async () => {
    useAuthStore.setState({ keyRotationInProgress: true });
    vi.mocked(invoke).mockImplementation(async (cmd: string) =>
      cmd === 'get_key_rotation_status'
        ? idleStatus
        : { retired_version: 1, active_version: 2, discarded: 1, orphaned_files_removed: 0 }
    );
    await useKeyRotationStore.getState().finalize('tok', true);
    expect(invoke).toHaveBeenCalledWith('finalize_data_key_rotation', { token: 'tok', discardUnreadable: true });
    expect(useAuthStore.getState().keyRotationInProgress).toBe(false);
    expect(useKeyRotationStore.getState().finalizeResult?.discarded).toBe(1);
  });

  it('fetchStatus keeps the status and tolerates errors', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ...idleStatus, in_progress: true });
    await useKeyRotationStore.getState().fetchStatus('tok');
    expect(useKeyRotationStore.getState().status?.in_progress).toBe(true);
    expect(useAuthStore.getState().keyRotationInProgress).toBe(true);

    vi.mocked(invoke).mockRejectedValueOnce('boom');
    await expect(useKeyRotationStore.getState().fetchStatus('tok')).resolves.toBeNull();
  });
});
