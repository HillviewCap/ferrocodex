import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import DynamicMetadataForm from '../DynamicMetadataForm';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('DynamicMetadataForm', () => {
  const mockSchema = {
    id: 1,
    name: 'Test Schema',
    description: 'A test schema for validation',
    schema_json: JSON.stringify({
      type: 'object',
      title: 'Test Schema',
      properties: {
        name: {
          type: 'string',
          title: 'Asset Name',
          description: 'Name of the asset',
          minLength: 2,
          maxLength: 50
        },
        ip_address: {
          type: 'string',
          title: 'IP Address',
          description: 'Network IP address',
          pattern: '^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$'
        },
        port: {
          type: 'number',
          title: 'Port Number',
          description: 'Network port',
          minimum: 1,
          maximum: 65535
        },
        device_type: {
          type: 'string',
          title: 'Device Type',
          description: 'Type of device',
          enum: ['PLC', 'HMI', 'Switch', 'Router']
        },
        is_active: {
          type: 'boolean',
          title: 'Active Status',
          description: 'Whether the device is active'
        },
        notes: {
          type: 'string',
          title: 'Notes',
          description: 'Additional notes',
          maxLength: 1000
        }
      },
      required: ['name', 'device_type'],
      fieldOrder: ['name', 'device_type', 'ip_address', 'port', 'is_active', 'notes']
    }),
    asset_type_filter: 'network_device',
    version: 1
  };

  const mockOnSubmit = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    // Mock localStorage
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: vi.fn(() => 'mock-token'),
        setItem: vi.fn(),
        removeItem: vi.fn(),
      },
      writable: true,
    });

    // Reset mocks
    mockInvoke.mockReset();
    mockOnSubmit.mockReset();
    mockOnCancel.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should render loading state initially', () => {
    mockInvoke.mockImplementation(() => new Promise(() => {})); // Never resolves

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    expect(screen.getByText('Loading form schema...')).toBeInTheDocument();
  });

  it('should render form fields after loading schema', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema); // get_metadata_schema_by_id

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Check that form fields are rendered
    expect(screen.getByLabelText(/Asset Name/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Device Type/)).toBeInTheDocument();
    expect(screen.getByLabelText(/IP Address/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Port Number/)).toBeInTheDocument();
    expect(screen.getByText(/Active Status/)).toBeInTheDocument(); // Checkbox
    expect(screen.getByLabelText(/Notes/)).toBeInTheDocument();
  });

  it('should show required field indicators', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema);

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Required fields should have asterisk
    const nameLabel = screen.getByText('Asset Name');
    const deviceTypeLabel = screen.getByText('Device Type');
    
    expect(nameLabel.textContent).toContain('*');
    expect(deviceTypeLabel.textContent).toContain('*');
  });

  it('should handle form submission with valid data', async () => {
    mockInvoke
      .mockResolvedValueOnce(mockSchema) // get_metadata_schema_by_id
      .mockResolvedValue(true); // validate_metadata_values

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Fill out form
    fireEvent.change(screen.getByLabelText(/Asset Name/), {
      target: { value: 'Test Device' }
    });

    fireEvent.change(screen.getByLabelText(/Device Type/), {
      target: { value: 'PLC' }
    });

    fireEvent.change(screen.getByLabelText(/IP Address/), {
      target: { value: '192.168.1.100' }
    });

    // Submit form
    const saveButton = screen.getByText('Save Metadata');
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        name: 'Test Device',
        device_type: 'PLC',
        ip_address: '192.168.1.100',
        port: undefined,
        is_active: undefined,
        notes: undefined
      });
    });
  });

  it('should handle validation errors', async () => {
    const validationError = 'name: Field is required, device_type: Invalid option selected';
    
    mockInvoke
      .mockResolvedValueOnce(mockSchema) // get_metadata_schema_by_id
      .mockRejectedValue(validationError); // validate_metadata_values fails

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Submit form without filling required fields
    const saveButton = screen.getByText('Save Metadata');
    fireEvent.click(saveButton);

    await waitFor(() => {
      // Form should not be submitted
      expect(mockOnSubmit).not.toHaveBeenCalled();
    });
  });

  it('should handle initial values', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema);

    const initialValues = {
      name: 'Existing Device',
      device_type: 'HMI',
      ip_address: '10.0.0.1',
      port: 8080,
      is_active: true,
      notes: 'Pre-filled notes'
    };

    render(
      <DynamicMetadataForm
        schemaId={1}
        initialValues={initialValues}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Check that initial values are populated
    expect(screen.getByDisplayValue('Existing Device')).toBeInTheDocument();
    expect(screen.getByDisplayValue('HMI')).toBeInTheDocument();
    expect(screen.getByDisplayValue('10.0.0.1')).toBeInTheDocument();
    expect(screen.getByDisplayValue('8080')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Pre-filled notes')).toBeInTheDocument();
  });

  it('should handle readonly mode', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema);

    render(
      <DynamicMetadataForm
        schemaId={1}
        readonly={true}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Save button should not be present in readonly mode
    expect(screen.queryByText('Save Metadata')).not.toBeInTheDocument();

    // Fields should be disabled
    const nameInput = screen.getByLabelText(/Asset Name/);
    expect(nameInput).toBeDisabled();
  });

  it('should handle schema not found', async () => {
    mockInvoke.mockResolvedValueOnce(null); // Schema not found

    render(
      <DynamicMetadataForm
        schemaId={999}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Schema not found')).toBeInTheDocument();
    });

    expect(screen.getByText('Retry')).toBeInTheDocument();
  });

  it('should handle network errors gracefully', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Network error'));

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      // Should stop loading and show error state
      expect(screen.queryByText('Loading form schema...')).not.toBeInTheDocument();
    });
  });

  it('should reset form when reset button is clicked', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema);

    const initialValues = {
      name: 'Initial Name',
      device_type: 'PLC'
    };

    render(
      <DynamicMetadataForm
        schemaId={1}
        initialValues={initialValues}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    // Modify a field
    const nameInput = screen.getByLabelText(/Asset Name/);
    fireEvent.change(nameInput, { target: { value: 'Modified Name' } });

    expect(screen.getByDisplayValue('Modified Name')).toBeInTheDocument();

    // Click reset
    const resetButton = screen.getByText('Reset');
    fireEvent.click(resetButton);

    // Should revert to initial values
    await waitFor(() => {
      expect(screen.getByDisplayValue('Initial Name')).toBeInTheDocument();
    });
  });

  it('should call onCancel when cancel button is clicked', async () => {
    mockInvoke.mockResolvedValueOnce(mockSchema);

    render(
      <DynamicMetadataForm
        schemaId={1}
        onSubmit={mockOnSubmit}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Schema')).toBeInTheDocument();
    });

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(mockOnCancel).toHaveBeenCalled();
  });
});