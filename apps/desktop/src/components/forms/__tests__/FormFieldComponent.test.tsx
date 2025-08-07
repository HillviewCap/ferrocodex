import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { vi, describe, it, expect } from 'vitest';
import { Form } from 'antd';
import FormFieldComponent, { FormField } from '../FormFieldComponent';

describe('FormFieldComponent', () => {
  const mockOnChange = vi.fn();
  const mockOnBlur = vi.fn();

  const baseField: FormField = {
    name: 'test_field',
    type: 'text',
    label: 'Test Field',
    description: 'A test field',
    required: false
  };

  beforeEach(() => {
    mockOnChange.mockClear();
    mockOnBlur.mockClear();
  });

  const renderWithForm = (component: React.ReactElement, initialValues?: any) => {
    return render(
      <Form initialValues={initialValues}>
        {component}
      </Form>
    );
  };

  it('should render text input field', () => {
    render(
      <Form>
        <FormFieldComponent
          field={baseField}
          onChange={mockOnChange}
          onBlur={mockOnBlur}
        />
      </Form>
    );

    expect(screen.getByLabelText(/Test Field/)).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('should render required field indicator', () => {
    const requiredField = { ...baseField, required: true };
    
    render(
      <FormFieldComponent
        field={requiredField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByText('*')).toBeInTheDocument();
  });

  it('should render field with description tooltip', () => {
    render(
      <FormFieldComponent
        field={baseField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByRole('img', { name: /info-circle/i })).toBeInTheDocument();
  });

  it('should render number input field', () => {
    const numberField: FormField = {
      ...baseField,
      type: 'number',
      validation: { min: 0, max: 100 }
    };

    render(
      <FormFieldComponent
        field={numberField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByRole('spinbutton')).toBeInTheDocument();
  });

  it('should render textarea field', () => {
    const textareaField: FormField = {
      ...baseField,
      type: 'textarea'
    };

    render(
      <FormFieldComponent
        field={textareaField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toHaveAttribute('rows', '4');
  });

  it('should render dropdown field with options', () => {
    const dropdownField: FormField = {
      ...baseField,
      type: 'dropdown',
      options: [
        { label: 'Option 1', value: 'opt1' },
        { label: 'Option 2', value: 'opt2' }
      ]
    };

    render(
      <FormFieldComponent
        field={dropdownField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByRole('combobox')).toBeInTheDocument();
  });

  it('should render checkbox field', () => {
    const checkboxField: FormField = {
      ...baseField,
      type: 'checkbox'
    };

    render(
      <FormFieldComponent
        field={checkboxField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    expect(screen.getByRole('checkbox')).toBeInTheDocument();
    expect(screen.getByText('Test Field')).toBeInTheDocument();
  });

  it('should render date picker field', () => {
    const dateField: FormField = {
      ...baseField,
      type: 'date'
    };

    render(
      <FormFieldComponent
        field={dateField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    // Ant Design DatePicker renders as textbox
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('should handle disabled state', () => {
    render(
      <FormFieldComponent
        field={baseField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
        disabled={true}
      />
    );

    expect(screen.getByRole('textbox')).toBeDisabled();
  });

  it('should display error state', () => {
    render(
      <FormFieldComponent
        field={baseField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
        error="This field is required"
      />
    );

    expect(screen.getByText('This field is required')).toBeInTheDocument();
  });

  it('should handle value changes', () => {
    render(
      <FormFieldComponent
        field={baseField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
        value="initial value"
      />
    );

    const input = screen.getByRole('textbox');
    expect(input).toHaveValue('initial value');

    fireEvent.change(input, { target: { value: 'new value' } });
    expect(mockOnChange).toHaveBeenCalledWith('new value');
  });

  it('should handle blur events', () => {
    render(
      <FormFieldComponent
        field={baseField}
        onChange={mockOnChange}
        onBlur={mockOnBlur}
      />
    );

    const input = screen.getByRole('textbox');
    fireEvent.blur(input);
    expect(mockOnBlur).toHaveBeenCalled();
  });
});