import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import RecoveryKeyResetModal from './RecoveryKeyResetModal';
import useAuthStore from '../store/auth';

const fillForm = () => {
  fireEvent.change(screen.getByLabelText('Username'), { target: { value: 'admin' } });
  fireEvent.change(screen.getByLabelText('Recovery Key'), { target: { value: '1a2b-3c4d' } });
  fireEvent.change(screen.getByLabelText('New Password'), { target: { value: 'Reset-Pass-456' } });
  fireEvent.change(screen.getByLabelText('Confirm New Password'), { target: { value: 'Reset-Pass-456' } });
};

describe('RecoveryKeyResetModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ pendingRecoveryCode: null });
  });

  it('resets the password and shows the replacement recovery key', async () => {
    vi.mocked(invoke).mockResolvedValue('9F8E-7D6C');
    const onClose = vi.fn();
    render(<RecoveryKeyResetModal open onClose={onClose} />);

    fillForm();
    fireEvent.click(screen.getByRole('button', { name: 'Reset Password' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('reset_password_with_recovery_key', {
        username: 'admin',
        recoveryCode: '1a2b-3c4d',
        newPassword: 'Reset-Pass-456',
      });
    });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(useAuthStore.getState().pendingRecoveryCode).toBe('9F8E-7D6C');
  });

  it('shows the error and stays open when the recovery key is wrong', async () => {
    vi.mocked(invoke).mockRejectedValue('Invalid username or recovery key');
    const onClose = vi.fn();
    render(<RecoveryKeyResetModal open onClose={onClose} />);

    fillForm();
    fireEvent.click(screen.getByRole('button', { name: 'Reset Password' }));

    expect(await screen.findByText('Invalid username or recovery key')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(useAuthStore.getState().pendingRecoveryCode).toBeNull();
  });

  it('requires matching passwords', async () => {
    render(<RecoveryKeyResetModal open onClose={vi.fn()} />);

    fillForm();
    fireEvent.change(screen.getByLabelText('Confirm New Password'), { target: { value: 'Different-Pass-1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reset Password' }));

    expect(await screen.findByText('Passwords do not match')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });
});
