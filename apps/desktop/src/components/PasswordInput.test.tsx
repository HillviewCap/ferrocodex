import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { App } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import PasswordInput from './PasswordInput';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('./PasswordGenerator', () => ({
  default: ({ visible, onGenerated }: { visible: boolean; onGenerated: (p: string) => void }) =>
    visible ? (
      <div data-testid="password-generator">
        <button onClick={() => onGenerated('Generated!Pass123')}>use generated</button>
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

  it('opens the generator and applies the generated password', async () => {
    const onChange = vi.fn();
    renderInput({ onChange });

    fireEvent.click(screen.getByRole('button', { name: 'key' }));
    expect(screen.getByTestId('password-generator')).toBeInTheDocument();

    fireEvent.click(screen.getByText('use generated'));
    expect(onChange).toHaveBeenCalledWith('Generated!Pass123');
    expect(await screen.findByText('Generated password applied')).toBeInTheDocument();
  });
});
