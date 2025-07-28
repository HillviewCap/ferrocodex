import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import { App } from 'antd';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PasswordGenerator from '../PasswordGenerator';
import { GeneratePasswordRequest, PasswordStrength, defaultPasswordRequest } from '../../types/vault';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock auth store
vi.mock('../../store/auth', () => ({
  default: () => ({
    token: 'mock-token',
  }),
}));

// Mock matchMedia for Ant Design responsive observer
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // Deprecated
    removeListener: vi.fn(), // Deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock PasswordStrengthIndicator component
vi.mock('../PasswordStrengthIndicator', () => ({
  default: ({ strength }: { strength: any }) => (
    <div data-testid="password-strength-indicator">
      Score: {strength.score}
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

const AppWrapper = ({ children }: { children: React.ReactNode }) => (
  <App>{children}</App>
);

describe('PasswordGenerator UI Synchronization Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Default mock implementations
    mockInvoke.mockImplementation((command: string, args: any) => {
      if (command === 'generate_secure_password') {
        const request = args.request as GeneratePasswordRequest;
        return Promise.resolve(`GeneratedPassword${request.length}`);
      }
      if (command === 'validate_password_strength') {
        return Promise.resolve(mockPasswordStrength);
      }
      return Promise.resolve();
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Modal Opening State Synchronization', () => {
    it('should reset to default configuration when modal opens', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Wait for initial render and auto-generation
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: defaultPasswordRequest,
        });
      });

      // Verify the exact default configuration is used
      expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
        token: 'mock-token',
        request: {
          length: 16,
          include_uppercase: true,
          include_lowercase: true,
          include_numbers: true,
          include_special: true,
          exclude_ambiguous: true,
        },
      });
    });

    it('should synchronize toggle switches to match default configuration on modal open', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Check that all toggle switches reflect the default configuration
      const uppercaseSwitch = screen.getByText('Uppercase (A-Z)').previousElementSibling?.querySelector('.ant-switch');
      const lowercaseSwitch = screen.getByText('Lowercase (a-z)').previousElementSibling?.querySelector('.ant-switch');
      const numbersSwitch = screen.getByText('Numbers (0-9)').previousElementSibling?.querySelector('.ant-switch');
      const specialSwitch = screen.getByText('Special (!@#$...)').previousElementSibling?.querySelector('.ant-switch');
      const ambiguousSwitch = screen.getByText('Exclude ambiguous characters').previousElementSibling?.querySelector('.ant-switch');

      // All switches should be checked according to defaultPasswordRequest
      expect(uppercaseSwitch).toHaveClass('ant-switch-checked'); // true in default
      expect(lowercaseSwitch).toHaveClass('ant-switch-checked'); // true in default
      expect(numbersSwitch).toHaveClass('ant-switch-checked');   // true in default
      expect(specialSwitch).toHaveClass('ant-switch-checked');   // true in default
      expect(ambiguousSwitch).toHaveClass('ant-switch-checked'); // true in default
    });

    it('should auto-generate password immediately when modal opens', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Should generate password immediately on modal open
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: defaultPasswordRequest,
        });
      });

      // Should also analyze the generated password
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('validate_password_strength', {
          token: 'mock-token',
          password: 'GeneratedPassword16',
        });
      });
    });

    it('should display the generated password in the text area', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        const passwordField = screen.getByDisplayValue('GeneratedPassword16');
        expect(passwordField).toBeInTheDocument();
      });
    });
  });

  describe('State Consistency Between Form and Generation', () => {
    it('should maintain consistent state between UI and generation parameters', async () => {
      const { rerender } = render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={false} />
        </AppWrapper>
      );

      // Initially no generation should happen
      expect(mockInvoke).not.toHaveBeenCalled();

      // Open the modal
      rerender(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={true} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: defaultPasswordRequest,
        });
      });

      // Verify UI switches match the generation request
      const uppercaseSwitch = screen.getByText('Uppercase (A-Z)').previousElementSibling?.querySelector('.ant-switch');
      expect(uppercaseSwitch).toHaveClass('ant-switch-checked');
    });

    it('should reset previous custom settings when modal reopens', async () => {
      const { rerender } = render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      // Wait for initial setup
      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Close the modal
      rerender(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={false} />
        </AppWrapper>
      );

      vi.clearAllMocks();

      // Reopen the modal - should reset to defaults again
      rerender(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={true} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: defaultPasswordRequest,
        });
      });

      // Verify all switches are in default state
      const uppercaseSwitch = screen.getByText('Uppercase (A-Z)').previousElementSibling?.querySelector('.ant-switch');
      const lowercaseSwitch = screen.getByText('Lowercase (a-z)').previousElementSibling?.querySelector('.ant-switch');
      const numbersSwitch = screen.getByText('Numbers (0-9)').previousElementSibling?.querySelector('.ant-switch');
      const specialSwitch = screen.getByText('Special (!@#$...)').previousElementSibling?.querySelector('.ant-switch');
      const ambiguousSwitch = screen.getByText('Exclude ambiguous characters').previousElementSibling?.querySelector('.ant-switch');

      expect(uppercaseSwitch).toHaveClass('ant-switch-checked');
      expect(lowercaseSwitch).toHaveClass('ant-switch-checked');
      expect(numbersSwitch).toHaveClass('ant-switch-checked');
      expect(specialSwitch).toHaveClass('ant-switch-checked');
      expect(ambiguousSwitch).toHaveClass('ant-switch-checked');
    });
  });

  describe('Length Slider Synchronization', () => {
    it('should set length slider to default value on modal open', async () => {
      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      const slider = screen.getByRole('slider');
      expect(slider).toHaveAttribute('aria-valuenow', '16'); // Default length
    });

    it('should generate password with correct length from slider', async () => {
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

  describe('Form Field Reset Behavior', () => {
    it('should properly initialize form fields when modal becomes visible', async () => {
      const { rerender } = render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={false} />
        </AppWrapper>
      );

      // Modal is not visible, no form should be rendered
      expect(screen.queryByText('Password Configuration')).not.toBeInTheDocument();

      // Make modal visible
      rerender(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={true} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Password Configuration')).toBeInTheDocument();
      });

      // Verify form fields are initialized correctly
      const slider = screen.getByRole('slider');
      expect(slider).toHaveAttribute('aria-valuenow', '16');
      
      // Check switches are in correct state
      const switches = screen.getAllByRole('switch');
      expect(switches).toHaveLength(5); // 5 toggle switches total
      
      // All should be checked according to default configuration
      switches.slice(0, 4).forEach(switchElement => { // First 4 are all true in default
        expect(switchElement).toBeChecked();
      });
      expect(switches[4]).toBeChecked(); // exclude_ambiguous is also true
    });
  });

  describe('Edge Cases and Error Handling', () => {
    it('should handle rapid modal open/close cycles gracefully', async () => {
      const { rerender } = render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={false} />
        </AppWrapper>
      );

      // Rapid open/close cycles
      for (let i = 0; i < 3; i++) {
        rerender(
          <AppWrapper>
            <PasswordGenerator {...defaultProps} visible={true} />
          </AppWrapper>
        );
        
        rerender(
          <AppWrapper>
            <PasswordGenerator {...defaultProps} visible={false} />
          </AppWrapper>
        );
      }

      // Final open
      rerender(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} visible={true} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Should still work correctly
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('generate_secure_password', {
          token: 'mock-token',
          request: defaultPasswordRequest,
        });
      });
    });

    it('should handle authentication token issues during modal opening', async () => {
      // Mock no token
      vi.doMock('../../store/auth', () => ({
        default: () => ({
          token: null,
        }),
      }));

      render(
        <AppWrapper>
          <PasswordGenerator {...defaultProps} />
        </AppWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Generate')).toBeInTheDocument();
      });

      // Should not crash, but also should not call generation
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });
});