import React, { useState } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi, describe, beforeEach } from 'vitest';
import userEvent from '@testing-library/user-event';
import SecurityValidationInput from './SecurityValidationInput';
import { SecurityValidationResult } from '../../types/security';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock auth store
vi.mock('../../store/auth', () => ({
  default: vi.fn(() => ({
    token: 'test-token',
  })),
}));

const { invoke } = await import('@tauri-apps/api/core');
const mockInvoke = vi.mocked(invoke);

// SecurityValidationInput is controlled; hold the value in state so typing accumulates
const ControlledInput: React.FC<Omit<React.ComponentProps<typeof SecurityValidationInput>, 'value'>> = ({
  onChange,
  ...props
}) => {
  const [value, setValue] = useState('');
  return (
    <SecurityValidationInput
      {...props}
      value={value}
      onChange={(next) => {
        setValue(next);
        onChange(next);
      }}
    />
  );
};

// antd renders each suggestion twice (visible item + a11y listbox); return the visible item
const findSuggestionOption = async (name: string) => {
  const matches = await screen.findAllByText(name);
  const option = matches.map(el => el.closest('.ant-select-item-option')).find(Boolean);
  if (!option) throw new Error(`No dropdown option for ${name}`);
  return option as HTMLElement;
};

describe('SecurityValidationInput', () => {
  const mockOnChange = vi.fn();
  const mockOnValidationChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockReset();
  });

  test('renders input field correctly', () => {
    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
      />
    );

    const input = screen.getByRole('combobox');
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute('placeholder', 'Enter asset name');
  });

  test('calls onChange when user types', async () => {
    const user = userEvent.setup();
    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'test-asset');

    expect(mockOnChange).toHaveBeenLastCalledWith('test-asset');
  });

  test('shows loading state during validation', async () => {
    const user = userEvent.setup();
    
    // Mock a delayed validation response
    mockInvoke.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 1000)));

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'test');

    // Wait for debounce delay
    await waitFor(() => {
      expect(screen.getByText('Validating name...')).toBeInTheDocument();
    }, { timeout: 200 });
  });

  test('displays success state for valid names', async () => {
    const user = userEvent.setup();
    
    const mockValidationResult: SecurityValidationResult = {
      isValid: true,
      suggestedCorrections: [],
      securityFlags: []
    };

    mockInvoke.mockResolvedValue(mockValidationResult);

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'valid-asset-name');

    await waitFor(() => {
      expect(screen.getByText('Valid asset name')).toBeInTheDocument();
    });

    expect(mockOnValidationChange).toHaveBeenCalledWith(
      expect.objectContaining({
        isValid: true,
        validationTimestamp: expect.any(String)
      })
    );
  });

  test('displays error state for invalid names', async () => {
    const user = userEvent.setup();
    
    const mockValidationResult: SecurityValidationResult = {
      isValid: false,
      errorCode: 'INVALID_PATTERN',
      errorMessage: 'Asset name contains invalid characters',
      suggestedCorrections: ['valid-asset-name'],
      securityFlags: []
    };

    mockInvoke.mockResolvedValue(mockValidationResult);

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'invalid@asset#name');

    await waitFor(() => {
      expect(screen.getByText('Asset name needs adjustment')).toBeInTheDocument();
      expect(screen.getByText('Asset name contains invalid characters')).toBeInTheDocument();
    });
  });

  test('shows suggestions when available', async () => {
    const user = userEvent.setup();
    
    const mockValidationResult: SecurityValidationResult = {
      isValid: false,
      errorCode: 'INVALID_PATTERN',
      errorMessage: 'Invalid characters detected',
      suggestedCorrections: ['suggestion-1', 'suggestion-2'],
      securityFlags: []
    };

    mockInvoke.mockResolvedValue(mockValidationResult);

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        showSuggestions={true}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'invalid');

    // Wait for validation and then check for suggestions in dropdown
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('validate_asset_name', {
        token: 'test-token',
        name: 'invalid'
      });
    });

    // Click input to open suggestions
    await user.click(input);
    
    expect(await findSuggestionOption('suggestion-1')).toBeInTheDocument();
    expect(await findSuggestionOption('suggestion-2')).toBeInTheDocument();
  });

  test('handles suggestion selection', async () => {
    const user = userEvent.setup();
    
    const mockValidationResult: SecurityValidationResult = {
      isValid: false,
      suggestedCorrections: ['corrected-name'],
      securityFlags: []
    };

    const mockValidationResultForSuggestion: SecurityValidationResult = {
      isValid: true,
      suggestedCorrections: [],
      securityFlags: []
    };

    mockInvoke
      .mockResolvedValueOnce(mockValidationResult)
      .mockResolvedValueOnce(mockValidationResultForSuggestion);

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        showSuggestions={true}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'invalid');

    await user.click(await findSuggestionOption('corrected-name'));

    expect(mockOnChange).toHaveBeenCalledWith('corrected-name');
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenLastCalledWith('validate_asset_name', {
        token: 'test-token',
        name: 'corrected-name',
      });
    });
  });

  test('reports security flags to the parent', async () => {
    const user = userEvent.setup();
    
    const mockValidationResult: SecurityValidationResult = {
      isValid: true,
      suggestedCorrections: [],
      securityFlags: ['Contains sensitive keywords', 'Requires administrator review']
    };

    mockInvoke.mockResolvedValue(mockValidationResult);

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'sensitive-asset');

    await waitFor(() => {
      expect(mockOnValidationChange).toHaveBeenCalledWith(
        expect.objectContaining({
          isValid: true,
          securityFlags: ['Contains sensitive keywords', 'Requires administrator review'],
        })
      );
    });
  });

  test('shows character count', () => {
    render(
      <SecurityValidationInput 
        value="test"
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        maxLength={50}
      />
    );

    expect(screen.getByText('4/50')).toBeInTheDocument();
  });

  test('shows warning when approaching character limit', () => {
    render(
      <SecurityValidationInput 
        value={"a".repeat(95)}
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        maxLength={100}
      />
    );

    const characterCount = screen.getByText('95/100');
    expect(characterCount).toHaveStyle({ color: 'rgb(250, 173, 20)' }); // Warning color
  });

  test('shows error when over character limit', () => {
    render(
      <SecurityValidationInput 
        value={"a".repeat(105)}
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        maxLength={100}
      />
    );

    const characterCount = screen.getByText('105/100');
    expect(characterCount).toHaveStyle({ color: 'rgb(255, 77, 79)' }); // Error color
  });

  test('handles validation API errors gracefully', async () => {
    const user = userEvent.setup();
    
    mockInvoke.mockRejectedValue(new Error('Network error'));

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={50}
      />
    );

    const input = screen.getByRole('combobox');
    await user.type(input, 'test');

    await waitFor(() => {
      expect(mockOnValidationChange).toHaveBeenCalledWith(
        expect.objectContaining({
          isValid: false,
          errorCode: 'VALIDATION_ERROR',
          errorMessage: 'Network error'
        })
      );
    });
  });

  test('respects disabled state', () => {
    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        disabled={true}
      />
    );

    const input = screen.getByRole('combobox');
    expect(input).toBeDisabled();
  });

  test('debounces validation calls', async () => {
    const user = userEvent.setup();
    
    mockInvoke.mockResolvedValue({ isValid: true, suggestedCorrections: [], securityFlags: [] });

    render(
      <ControlledInput
        onChange={mockOnChange}
        onValidationChange={mockOnValidationChange}
        validationDelay={200}
      />
    );

    const input = screen.getByRole('combobox');
    
    // Type multiple characters quickly
    await user.type(input, 'test', { delay: 10 });

    // Should only call validation once after the debounce delay
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith('validate_asset_name', {
        token: 'test-token',
        name: 'test'
      });
    }, { timeout: 500 });
  });
});