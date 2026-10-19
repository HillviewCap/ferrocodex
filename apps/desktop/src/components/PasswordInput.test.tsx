import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { App } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import PasswordInput from './PasswordInput';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

// Like the real generator, it never closes itself: the parent must hide it
vi.mock('./PasswordGenerator', () => ({
  default: ({ visible, onGenerated, onCancel }: { visible: boolean; onGenerated: (p: string) => void; onCancel: () => void }) =>
    visible ? (
      <div data-testid="password-generator">
        <button onClick={() => onGenerated('Generated!Pass123')}>use generated</button>
        <button onClick={onCancel}>cancel generator</button>
      </div>
    ) : null,
}));

const weak = {
  score: 25,
  entropy: 20,
  has_uppercase: false,
  has_lowercase: true,
  has_numbers: false,
  has_special: false,
  length: 6,
  feedback: ['Too short'],
};

const renderInput = (props: Partial<React.ComponentProps<typeof PasswordInput>> = {}) =>
  render(
    <App>
      <PasswordInput value="" onChange={vi.fn()} {...props} />
    </App>,
  );

describe('PasswordInput', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'validate_password_strength') return weak;
      if (cmd === 'check_password_reuse') return true;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders the password field and forwards typed values', () => {
    const onChange = vi.fn();
    renderInput({ onChange, placeholder: 'Secret' });

    const input = screen.getByPlaceholderText('Secret');
    fireEvent.change(input, { target: { value: 'abc' } });
    expect(onChange).toHaveBeenCalledWith('abc');
    expect(screen.queryByText('Weak Password')).not.toBeInTheDocument();
  });

  it('analyses the value and shows strength, weak-password and reuse alerts', async () => {
    const onStrengthChange = vi.fn();
    const onReuseCheck = vi.fn();
    renderInput({ value: 'abcdef', secretId: 3, onStrengthChange, onReuseCheck });

    expect(await screen.findByText('Weak (25/100)')).toBeInTheDocument();
    expect(screen.getByText('Weak Password')).toBeInTheDocument();
    expect(screen.getByText(/Password strength is below recommended level \(25\/40\)/)).toBeInTheDocument();

    expect(await screen.findByText('Password Reuse Detected')).toBeInTheDocument();
    expect(screen.getByText('This password has been used before on this credential')).toBeInTheDocument();

    expect(invoke).toHaveBeenCalledWith('validate_password_strength', { token: 'test-token', password: 'abcdef' });
    expect(invoke).toHaveBeenCalledWith('check_password_reuse', {
      token: 'test-token',
      password: 'abcdef',
      secretId: 3,
    });
    expect(onStrengthChange).toHaveBeenCalledWith(weak);
    expect(onReuseCheck).toHaveBeenCalledWith(true);
  });

  it('skips the reuse check for a new secret that has no id yet', async () => {
    const onReuseCheck = vi.fn();
    renderInput({ value: 'abcdef', onReuseCheck });

    expect(await screen.findByText('Weak (25/100)')).toBeInTheDocument();
    await waitFor(() => expect(onReuseCheck).toHaveBeenCalledWith(false));
    expect(invoke).not.toHaveBeenCalledWith('check_password_reuse', expect.anything());
  });

  it('opens the generator from a labelled primary button, applies the password and closes it', async () => {
    const onChange = vi.fn();
    renderInput({ onChange });

    const generate = screen.getByRole('button', { name: /Generate Password/ });
    expect(generate).toHaveClass('ant-btn-primary');
    fireEvent.click(generate);
    expect(screen.getByTestId('password-generator')).toBeInTheDocument();

    fireEvent.click(screen.getByText('use generated'));
    expect(onChange).toHaveBeenCalledWith('Generated!Pass123');
    expect(await screen.findByText('Generated password applied')).toBeInTheDocument();
    expect(screen.queryByTestId('password-generator')).not.toBeInTheDocument();

    // It can be opened again, and cancelling closes it too
    fireEvent.click(generate);
    fireEvent.click(screen.getByText('cancel generator'));
    expect(screen.queryByTestId('password-generator')).not.toBeInTheDocument();
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it('hides the generator button when showGenerator is false and disables it with the field', () => {
    const { unmount } = renderInput({ showGenerator: false });
    expect(screen.queryByRole('button', { name: /Generate Password/ })).not.toBeInTheDocument();
    unmount();

    renderInput({ disabled: true });
    expect(screen.getByRole('button', { name: /Generate Password/ })).toBeDisabled();
  });
});
