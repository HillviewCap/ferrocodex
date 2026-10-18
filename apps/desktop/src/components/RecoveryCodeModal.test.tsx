import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeEach } from 'vitest';
import RecoveryCodeModal from './RecoveryCodeModal';
import useAuthStore from '../store/auth';

describe('RecoveryCodeModal', () => {
  beforeEach(() => {
    useAuthStore.setState({ pendingRecoveryCode: null });
  });

  it('renders nothing without a pending recovery code', () => {
    render(<RecoveryCodeModal />);

    expect(screen.queryByText('Save your recovery key')).not.toBeInTheDocument();
  });

  it('shows the code and stays open until the user confirms storing it', () => {
    useAuthStore.setState({ pendingRecoveryCode: '1A2B-3C4D-5E6F' });
    render(<RecoveryCodeModal />);

    expect(screen.getByText('Save your recovery key')).toBeInTheDocument();
    expect(screen.getByText('1A2B-3C4D-5E6F')).toBeInTheDocument();
    expect(screen.getByText('This key is shown only once')).toBeInTheDocument();

    const done = screen.getByRole('button', { name: 'Done' });
    expect(done).toBeDisabled();

    fireEvent.click(screen.getByRole('checkbox', { name: /I have stored the recovery key/ }));
    expect(done).toBeEnabled();

    fireEvent.click(done);
    expect(useAuthStore.getState().pendingRecoveryCode).toBeNull();
  });
});
