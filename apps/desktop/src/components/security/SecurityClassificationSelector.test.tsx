import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { expect, test, vi, describe } from 'vitest';
import userEvent from '@testing-library/user-event';
import SecurityClassificationSelector from './SecurityClassificationSelector';
import { SecurityClassificationLevel } from '../../types/security';

describe('SecurityClassificationSelector', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders with all classification levels', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
      />
    );

    const selector = screen.getByRole('combobox');
    expect(selector).toBeInTheDocument();
    expect(selector).toHaveAttribute('placeholder', 'Select security classification');
  });

  test('displays classification options when opened', async () => {
    const user = userEvent.setup();
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
      />
    );

    const selector = screen.getByRole('combobox');
    await user.click(selector);

    // Check that all classification levels are present
    expect(screen.getByText('Public')).toBeInTheDocument();
    expect(screen.getByText('Internal')).toBeInTheDocument();
    expect(screen.getByText('Confidential')).toBeInTheDocument();
    expect(screen.getByText('Restricted')).toBeInTheDocument();
    expect(screen.getByText('Secret')).toBeInTheDocument();
  });

  test('calls onChange when selection is made', async () => {
    const user = userEvent.setup();
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
      />
    );

    const selector = screen.getByRole('combobox');
    await user.click(selector);
    
    const confidentialOption = screen.getByText('Confidential');
    await user.click(confidentialOption);

    expect(mockOnChange).toHaveBeenCalledWith(SecurityClassificationLevel.CONFIDENTIAL);
  });

  test('disables high-level classifications for Engineer role', async () => {
    const user = userEvent.setup();
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        currentUserRole="Engineer"
        allowElevation={false}
      />
    );

    const selector = screen.getByRole('combobox');
    await user.click(selector);

    // Restricted and Secret should be disabled for Engineers without elevation
    const restrictedOption = screen.getByText('Restricted').closest('.ant-select-item');
    const secretOption = screen.getByText('Secret').closest('.ant-select-item');
    
    expect(restrictedOption).toHaveClass('ant-select-item-option-disabled');
    expect(secretOption).toHaveClass('ant-select-item-option-disabled');
  });

  test('shows inheritance warning when parent classification exists', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        inheritedClassification={SecurityClassificationLevel.CONFIDENTIAL}
      />
    );

    expect(screen.getByText(/Inherited Classification: Confidential/)).toBeInTheDocument();
    expect(screen.getByText(/You can only select equal or higher classifications/)).toBeInTheDocument();
  });

  test('shows administrator approval warning for elevated classifications', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        value={SecurityClassificationLevel.RESTRICTED}
        currentUserRole="Engineer"
      />
    );

    expect(screen.getByText('Administrator Approval Required')).toBeInTheDocument();
    expect(screen.getByText(/This classification level requires administrator approval/)).toBeInTheDocument();
  });

  test('displays classification description when enabled', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        value={SecurityClassificationLevel.CONFIDENTIAL}
        showDescription={true}
      />
    );

    expect(screen.getByText('Confidential')).toBeInTheDocument();
    expect(screen.getByText(/Sensitive information requiring controlled access/)).toBeInTheDocument();
  });

  test('shows access requirements when enabled', async () => {
    const user = userEvent.setup();
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        value={SecurityClassificationLevel.CONFIDENTIAL}
        showAccessRequirements={true}
      />
    );

    const selector = screen.getByRole('combobox');
    await user.click(selector);

    expect(screen.getByText('Access Requirements:')).toBeInTheDocument();
    expect(screen.getByText('Manager approval')).toBeInTheDocument();
    expect(screen.getByText('Confidentiality agreement')).toBeInTheDocument();
  });

  test('handles disabled state correctly', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        disabled={true}
      />
    );

    const selector = screen.getByRole('combobox');
    expect(selector).toBeDisabled();
  });

  test('prevents selection of lower classification than inherited', async () => {
    const user = userEvent.setup();
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        inheritedClassification={SecurityClassificationLevel.CONFIDENTIAL}
      />
    );

    const selector = screen.getByRole('combobox');
    await user.click(selector);

    // Public and Internal should be disabled
    const publicOption = screen.getByText('Public').closest('.ant-select-item');
    const internalOption = screen.getByText('Internal').closest('.ant-select-item');
    
    expect(publicOption).toHaveClass('ant-select-item-option-disabled');
    expect(internalOption).toHaveClass('ant-select-item-option-disabled');
  });

  test('renders with different sizes', () => {
    const { rerender } = render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        size="small"
      />
    );

    let selector = screen.getByRole('combobox');
    expect(selector).toHaveClass('ant-select-sm');

    rerender(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        size="large"
      />
    );

    selector = screen.getByRole('combobox');
    expect(selector).toHaveClass('ant-select-lg');
  });

  test('displays selected classification details card', () => {
    render(
      <SecurityClassificationSelector 
        onChange={mockOnChange}
        value={SecurityClassificationLevel.RESTRICTED}
        showDescription={true}
        showAccessRequirements={true}
      />
    );

    expect(screen.getByText('Selected Classification')).toBeInTheDocument();
    expect(screen.getByText('Restricted')).toBeInTheDocument();
    expect(screen.getByText(/Highly sensitive information with limited access/)).toBeInTheDocument();
    
    // Should show access requirements in the details card
    expect(screen.getByText('Access Requirements:')).toBeInTheDocument();
  });
});