import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { App } from 'antd';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PasswordGenerator from '../PasswordGenerator';
import { GeneratePasswordRequest, PasswordStrength, defaultPasswordRequest } from '../../types/vault';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock auth store; tests can set authState.token to simulate a missing session
const authState = vi.hoisted(() => ({ token: 'mock-token' as string | null }));
vi.mock('../../store/auth', () => ({
  default: () => authState,
}));

// Mock PasswordStrengthIndicator component
vi.mock('../PasswordStrengthIndicator', () => ({
  default: ({ strength, showDetails }: { strength: any; showDetails: boolean }) => (
    <div data-testid="password-strength-indicator">
      <div>Score: {strength.score}</div>
      <div>Entropy: {strength.entropy}</div>
      {showDetails && (
        <div>
          <div>Uppercase: {strength.has_uppercase ? 'Yes' : 'No'}</div>
          <div>Lowercase: {strength.has_lowercase ? 'Yes' : 'No'}</div>
          <div>Numbers: {strength.has_numbers ? 'Yes' : 'No'}</div>
          <div>Special: {strength.has_special ? 'Yes' : 'No'}</div>
        </div>
      )}
    </div>
  ),
}));

const mockInvoke = vi.mocked(invoke);

const defaultProps = {
  visible: true,
  onCancel: vi.fn(),
  onGenerated: vi.fn(),
  title: 'Test Password Generator',
};

const mockPasswordStrength: PasswordStrength = {
  score: 85,
  entropy: 75.2,
  has_uppercase: true,
  has_lowercase: true,
  has_numbers: true,
  has_special: true,
  length: 16,
  feedback: ['Strong password'],
};

// Helper function to generate test passwords with specific characteristics
const generateTestPassword = (request: GeneratePasswordRequest): string => {
  let charset = '';
  if (request.include_uppercase) charset += 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  if (request.include_lowercase) charset += 'abcdefghijklmnopqrstuvwxyz';
  if (request.include_numbers) charset += '0123456789';
  if (request.include_special) charset += '!@#$%^&*()_+-=[]{}|;:,.<>?';
  
  if (request.exclude_ambiguous) {
    charset = charset.replace(/[0Ol1I]/g, '');
  }
  
  let password = '';
  for (let i = 0; i < request.length; i++) {
    password += charset.charAt(Math.floor(Math.random() * charset.length));
  }
  return password;
};

const AppWrapper = ({ children }: { children: React.ReactNode }) => (
  <App>{children}</App>
);

// Each switch shares an ant-space with its visible label text
const getSwitch = (label: string) =>
  screen.getByText(label).closest('.ant-space')!.querySelector<HTMLElement>('[role="switch"]')!;

// The antd slider handle has no value setter; move it with the keyboard instead.
// rc-slider reads keyCode rather than key.
const SLIDER_KEYS = { ArrowRight: 39, End: 35 } as const;
const pressSliderKey = async (key: keyof typeof SLIDER_KEYS, times = 1) => {
  const slider = screen.getByRole('slider');
  for (let i = 0; i < times; i++) {
    await act(async () => {
      fireEvent.keyDown(slider, { key, keyCode: SLIDER_KEYS[key] });
    });
  }
};

// Each test mounts a full antd Modal and waits on async generate + analyze round trips,
// which can exceed the 5s default when the suite runs files in parallel
describe('PasswordGenerator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Default mock implementations
    mockInvoke.mockImplementation((command: string, args: any) => {
      if (command === 'generate_secure_password') {
        const request = args.request as GeneratePasswordRequest;
        return Promise.resolve(generateTestPassword(request));
      }
      if (command === 'validate_password_strength') {
        return Promise.resolve({
          ...mockPasswordStrength,
          has_uppercase: args.password.match(/[A-Z]/) !== null,
          has_lowercase: args.password.match(/[a-z]/) !== null,
          has_numbers: args.password.match(/[0-9]/) !== null,
          has_special: args.password.match(/[!@#$%^&*()_+\-=\[\]{}|;:,.<>?]/) !== null,
          length: args.password.length,
        });
      }
      return Promise.resolve();
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    authState.token = 'mock-token';
  });

  describe('Component Rendering', () => {
    it('renders the password generator modal when visible', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      expect(screen.getByText('Test Password Generator')).toBeInTheDocument();
      expect(screen.getByText('Password Configuration')).toBeInTheDocument();
      expect(screen.getByText('Generated Password')).toBeInTheDocument();
    });

    it('does not render when not visible', () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={false} />
        </AppWrapper>
      );

      expect(screen.queryByText('Test Password Generator')).not.toBeInTheDocument();
    });

    it('renders all toggle switches with correct initial state', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      expect(screen.getByText('Uppercase (A-Z)')).toBeInTheDocument();
      expect(screen.getByText('Lowercase (a-z)')).toBeInTheDocument();
      expect(screen.getByText('Numbers (0-9)')).toBeInTheDocument();
      expect(screen.getByText('Special (!@#$...)')).toBeInTheDocument();
      expect(screen.getByText('Exclude ambiguous characters')).toBeInTheDocument();
    });
  });

  describe('Password Generation with Toggle Settings', () => {
    it('generates password respecting uppercase toggle ON', async () => {
      const testPassword = 'TestPassword123!';
      mockInvoke.mockImplementation((command: string, args: any) => {
        if (command === 'generate_secure_password') {
          const request = args.request as GeneratePasswordRequest;
          expect(request.include_uppercase).toBe(true);
          return Promise.resolve(testPassword);
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve({
            ...mockPasswordStrength,
            has_uppercase: true,
          });
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_uppercase: true,
          }),
        });
      });
    });

    it('generates password respecting uppercase toggle OFF', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Wait for initial render and password generation
      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Find and toggle uppercase switch OFF
      const uppercaseSwitch = getSwitch('Uppercase (A-Z)');
      expect(uppercaseSwitch).toBeInTheDocument();
      
      await act(async () => {
        fireEvent.click(uppercaseSwitch);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_uppercase: false,
          }),
        });
      });
    });

    it('generates password respecting lowercase toggle OFF', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      const lowercaseSwitch = getSwitch('Lowercase (a-z)');
      expect(lowercaseSwitch).toBeInTheDocument();
      
      await act(async () => {
        fireEvent.click(lowercaseSwitch);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_lowercase: false,
          }),
        });
      });
    });

    it('generates password respecting numbers toggle OFF', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      const numbersSwitch = getSwitch('Numbers (0-9)');
      expect(numbersSwitch).toBeInTheDocument();
      
      await act(async () => {
        fireEvent.click(numbersSwitch);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_numbers: false,
          }),
        });
      });
    });

    it('generates password respecting special characters toggle OFF', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      const specialSwitch = getSwitch('Special (!@#$...)');
      expect(specialSwitch).toBeInTheDocument();
      
      await act(async () => {
        fireEvent.click(specialSwitch);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_special: false,
          }),
        });
      });
    });

    it('respects exclude ambiguous characters toggle', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      const ambiguousSwitch = getSwitch('Exclude ambiguous characters');
      expect(ambiguousSwitch).toBeInTheDocument();
      
      // Toggle OFF (currently ON by default)
      await act(async () => {
        fireEvent.click(ambiguousSwitch);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            exclude_ambiguous: false,
          }),
        });
      });
    });
  });

  describe('Length Parameter Validation', () => {
    it('respects length parameter changes', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Move the length slider from the default 16 up to 18
      await pressSliderKey('ArrowRight', 2);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            length: 18,
          }),
        });
      });
    });

    it('uses default length of 16 initially', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            length: 16,
          }),
        });
      });
    });
  });

  describe('Auto-regeneration Functionality', () => {
    it('auto-regenerates password when settings change', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Wait for initial generation
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledTimes(2); // generate + analyze
      });

      vi.clearAllMocks();

      // Change a setting (toggle uppercase off)
      const uppercaseSwitch = getSwitch('Uppercase (A-Z)');
      
      await act(async () => {
        fireEvent.click(uppercaseSwitch);
      });

      // Should auto-regenerate
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            include_uppercase: false,
          }),
        });
      });
    });

    it('auto-regenerates when length changes', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledTimes(2);
      });

      vi.clearAllMocks();

      // Jump the slider to its maximum
      await pressSliderKey('End');

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.objectContaining({
            length: 64,
          }),
        });
      });
    });
  });

  describe('Validation and Error Handling', () => {
    it('shows warning when no character types are selected', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Turn off all character types
      const switches = [
        getSwitch('Uppercase (A-Z)'),
        getSwitch('Lowercase (a-z)'),
        getSwitch('Numbers (0-9)'),
        getSwitch('Special (!@#$...)'),
      ];

      for (const switchEl of switches) {
        await act(async () => {
          fireEvent.click(switchEl);
        });
      }

      await waitFor(() => {
        expect(screen.getByText('At least one character type must be selected')).toBeInTheDocument();
      });
    });

    it('disables generate button when no character types selected', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Turn off all character types
      const switches = [
        getSwitch('Uppercase (A-Z)'),
        getSwitch('Lowercase (a-z)'),
        getSwitch('Numbers (0-9)'),
        getSwitch('Special (!@#$...)'),
      ];

      for (const switchEl of switches) {
        await act(async () => {
          fireEvent.click(switchEl);
        });
      }

      const generateButton = screen.getByRole('button', { name: /Generate/ });
      await waitFor(() => {
        expect(generateButton).toBeDisabled();
      });
    });

    it('handles password generation errors gracefully', async () => {
      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return Promise.reject(new Error('Generation failed'));
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // The error should be handled and not crash the component
      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });
    });

    it('handles password strength analysis errors gracefully', async () => {
      mockInvoke.mockImplementation((command: string, args: any) => {
        if (command === 'generate_secure_password') {
          return Promise.resolve('TestPassword123!');
        }
        if (command === 'validate_password_strength') {
          return Promise.reject(new Error('Analysis failed'));
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });
    });
  });

  describe('User Interface Interactions', () => {
    it('shows and hides password when eye icon is clicked', async () => {
      const testPassword = 'TestPassword123!';
      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return Promise.resolve(testPassword);
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve(mockPasswordStrength);
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        const passwordField = screen.getByDisplayValue(testPassword);
        expect(passwordField).toBeInTheDocument();
      });

      // Hide the password
      await act(async () => {
        fireEvent.click(screen.getByLabelText('Hide password'));
      });

      await waitFor(() => {
        expect(screen.getByDisplayValue('••••••••••••••••')).toBeInTheDocument();
      });

      // Show it again
      await act(async () => {
        fireEvent.click(screen.getByLabelText('Show password'));
      });

      expect(screen.getByDisplayValue(testPassword)).toBeInTheDocument();
    });

    it('copies password to clipboard when copy button is clicked', async () => {
      const testPassword = 'TestPassword123!';
      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return Promise.resolve(testPassword);
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve(mockPasswordStrength);
        }
        return Promise.resolve();
      });

      // Mock clipboard API
      Object.assign(navigator, {
        clipboard: {
          writeText: vi.fn().mockResolvedValue(undefined),
        },
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByDisplayValue(testPassword)).toBeInTheDocument();
      });

      const copyButton = screen.getByLabelText('Copy to clipboard');
      await act(async () => {
        fireEvent.click(copyButton);
      });

      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(testPassword);
    });

    it('calls onGenerated callback when "Use This Password" is clicked', async () => {
      const testPassword = 'TestPassword123!';
      const onGenerated = vi.fn();
      const onCancel = vi.fn();

      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return Promise.resolve(testPassword);
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve({ ...mockPasswordStrength, score: 85 });
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} onGenerated={onGenerated} onCancel={onCancel} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByDisplayValue(testPassword)).toBeInTheDocument();
      });

      // Enabled once the strength analysis comes back above the threshold
      const useButton = screen.getByRole('button', { name: 'Use This Password' });
      await waitFor(() => {
        expect(useButton).not.toBeDisabled();
      });

      await act(async () => {
        fireEvent.click(useButton);
      });

      expect(onGenerated).toHaveBeenCalledWith(testPassword);
      // The parent closes the modal; PasswordGenerator doesn't call onCancel itself
      expect(onCancel).not.toHaveBeenCalled();
    });

    it('disables "Use This Password" button for weak passwords', async () => {
      const testPassword = 'weak';
      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return Promise.resolve(testPassword);
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve({ ...mockPasswordStrength, score: 20 });
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Wait for the weak analysis result to land, then check the button
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('validate_password_strength', {
          token: 'mock-token',
          password: testPassword,
        });
      });
      await waitFor(() => {
        expect(screen.getByText('Score: 20')).toBeInTheDocument();
      });
      expect(screen.getByRole('button', { name: 'Use This Password' })).toBeDisabled();
    });
  });

  describe('Manual Regeneration', () => {
    it('regenerates password when generate button is clicked', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledTimes(2); // initial generation + analysis
      });

      vi.clearAllMocks();

      const generateButton = screen.getByText('Generate');
      await act(async () => {
        fireEvent.click(generateButton);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: expect.any(Object),
        });
      });
    });

    it('shows loading state during generation', async () => {
      let resolveGeneration: (value: string) => void;
      const generationPromise = new Promise<string>((resolve) => {
        resolveGeneration = resolve;
      });

      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return generationPromise;
        }
        if (command === 'validate_password_strength') {
          return Promise.resolve(mockPasswordStrength);
        }
        return Promise.resolve();
      });

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      const generateButton = screen.getByText('Generate');
      await act(async () => {
        fireEvent.click(generateButton);
      });

      // Should show loading state
      expect(generateButton.closest('button')).toHaveClass('ant-btn-loading');

      // Resolve the generation
      await act(async () => {
        resolveGeneration!('TestPassword123!');
      });

      await waitFor(() => {
        expect(generateButton.closest('button')).not.toHaveClass('ant-btn-loading');
      });
    });
  });

  describe('Edge Cases', () => {
    it('handles missing auth token gracefully', async () => {
      authState.token = null;

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Should not crash and not call invoke without token
      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('handles component unmounting during async operations', async () => {
      let resolveGeneration: (value: string) => void;
      const generationPromise = new Promise<string>((resolve) => {
        resolveGeneration = resolve;
      });

      mockInvoke.mockImplementation((command: string) => {
        if (command === 'generate_secure_password') {
          return generationPromise;
        }
        return Promise.resolve();
      });

      const { unmount } = render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Start generation and immediately unmount
      unmount();

      // Resolve after unmount - should not cause errors
      await act(async () => {
        resolveGeneration!('TestPassword123!');
      });

      // No assertions needed - just ensuring no errors are thrown
    });

    it('validates minimum length requirements', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // The slider should enforce minimum of 8
      const slider = screen.getByRole('slider');
      expect(slider).toHaveAttribute('aria-valuemin', '8');
    });

    it('validates maximum length requirements', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // The slider should enforce maximum of 64
      const slider = screen.getByRole('slider');
      expect(slider).toHaveAttribute('aria-valuemax', '64');
    });
  });
});