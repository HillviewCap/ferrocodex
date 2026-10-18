import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import Dashboard from './Dashboard';
import useKeyRotationStore from '../store/keyRotation';

const authState = vi.hoisted(() => ({
  role: 'Administrator' as 'Administrator' | 'Engineer',
  keyRotationInProgress: true,
}));

vi.mock('../store/auth', () => ({
  default: () => ({
    user: {
      id: 1,
      username: 'someone',
      role: authState.role,
      created_at: '2023-01-01',
      is_active: true,
    },
    token: 'test-token',
    logout: vi.fn(),
    keyRotationInProgress: authState.keyRotationInProgress,
    showRecoveryCode: vi.fn(),
  }),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({ open: vi.fn() }));

const rotationStatus = (inProgress: boolean) => ({
  in_progress: inProgress,
  active_version: 2,
  retiring_version: inProgress ? 1 : null,
  details: null,
});

describe('Dashboard key rotation banner', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useKeyRotationStore.getState().reset();
    authState.role = 'Administrator';
    authState.keyRotationInProgress = true;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return rotationStatus(true);
      return { total_assets: 0, total_versions: 0, encryption_type: 'AES-256' };
    });
  });

  it('shows administrators a Resume action that opens the rotation modal', async () => {
    render(<Dashboard />);
    expect(await screen.findByText('Encryption key rotation in progress')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    expect(await screen.findByText('Rotate Encryption Key')).toBeInTheDocument();
  });

  it('tells engineers data is unavailable and offers no Resume action', async () => {
    authState.role = 'Engineer';
    render(<Dashboard />);
    expect(
      await screen.findByText(
        'Encrypted data is unavailable until an administrator finishes rotating the encryption key.'
      )
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Resume' })).not.toBeInTheDocument();
  });

  it('hides the banner when no rotation is in progress', async () => {
    authState.keyRotationInProgress = false;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_key_rotation_status') return rotationStatus(false);
      return { total_assets: 0, total_versions: 0, encryption_type: 'AES-256' };
    });
    render(<Dashboard />);
    await screen.findByText('Welcome, someone');
    expect(screen.queryByText('Encryption key rotation in progress')).not.toBeInTheDocument();
  });
});
