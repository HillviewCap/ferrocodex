import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import SchemaDesigner from '../SchemaDesigner';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock react-beautiful-dnd
vi.mock('react-beautiful-dnd', () => ({
  DragDropContext: ({ children }: any) => children,
  Droppable: ({ children }: any) => children({ innerRef: vi.fn(), droppableProps: {}, placeholder: null }),
  Draggable: ({ children }: any) => children({ innerRef: vi.fn(), draggableProps: {}, dragHandleProps: {} }, {})
}));

const mockInvoke = vi.mocked(invoke);

describe('SchemaDesigner', () => {
  const mockOnSave = vi.fn();
  const mockOnCancel = vi.fn();

  const mockExistingSchema = {
    id: 1,
    name: 'Existing Schema',
    description: 'An existing schema for testing',
    schema_json: JSON.stringify({
      type: 'object',
      title: 'Existing Schema',
      description: 'An existing schema for testing',
      properties: {
        name: {
          type: 'string',
          title: 'Name',
          description: 'Device name',
          minLength: 1,
          maxLength: 50
        },
        type: {
          type: 'string',
          title: 'Type',
          enum: ['PLC', 'HMI', 'Switch']
        }
      },
      required: ['name'],
      fieldOrder: ['name', 'type']
    }),
    asset_type_filter: 'device',
    version: 1
  };

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
    mockOnSave.mockReset();
    mockOnCancel.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should render schema designer interface', () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    expect(screen.getByText('Schema Designer')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Schema Name')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Schema Description')).toBeInTheDocument();
    expect(screen.getByText('Add Field')).toBeInTheDocument();
    expect(screen.getByText('Visual Designer')).toBeInTheDocument();
    expect(screen.getByText('JSON Schema')).toBeInTheDocument();
  });

  it('should load existing schema when schemaId is provided', async () => {
    mockInvoke.mockResolvedValueOnce(mockExistingSchema);

    render(
      <SchemaDesigner
        schemaId={1}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_metadata_schema_by_id', {
        token: 'mock-token',
        schemaId: 1
      });
    });

    await waitFor(() => {
      expect(screen.getByDisplayValue('Existing Schema')).toBeInTheDocument();
      expect(screen.getByText('Name')).toBeInTheDocument();
      expect(screen.getByText('Type')).toBeInTheDocument();
    });
  });

  it('should allow adding new fields', async () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const addFieldButton = screen.getByText('Add Field');
    fireEvent.click(addFieldButton);

    await waitFor(() => {
      expect(screen.getByText('Add Field')).toBeInTheDocument(); // Modal title
      expect(screen.getByPlaceholderText('field_name')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Field Label')).toBeInTheDocument();
    });
  });

  it('should handle field configuration modal', async () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    // Open add field modal
    const addFieldButton = screen.getByText('Add Field');
    fireEvent.click(addFieldButton);

    await waitFor(() => {
      expect(screen.getByText('Add Field')).toBeInTheDocument();
    });

    // Fill out field details
    fireEvent.change(screen.getByPlaceholderText('field_name'), {
      target: { value: 'test_field' }
    });

    fireEvent.change(screen.getByPlaceholderText('Field Label'), {
      target: { value: 'Test Field' }
    });

    // Save field
    const okButton = screen.getByRole('button', { name: /ok/i });
    fireEvent.click(okButton);

    await waitFor(() => {
      expect(screen.getByText('Test Field')).toBeInTheDocument();
    });
  });

  it('should generate JSON schema from fields', async () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    // Set schema name
    fireEvent.change(screen.getByPlaceholderText('Schema Name'), {
      target: { value: 'Test Schema' }
    });

    // Add a field
    const addFieldButton = screen.getByText('Add Field');
    fireEvent.click(addFieldButton);

    await waitFor(() => {
      expect(screen.getByPlaceholderText('field_name')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByPlaceholderText('field_name'), {
      target: { value: 'device_name' }
    });

    fireEvent.change(screen.getByPlaceholderText('Field Label'), {
      target: { value: 'Device Name' }
    });

    const okButton = screen.getByRole('button', { name: /ok/i });
    fireEvent.click(okButton);

    // Switch to JSON Schema tab
    const jsonTab = screen.getByText('JSON Schema');
    fireEvent.click(jsonTab);

    await waitFor(() => {
      const textarea = screen.getByRole('textbox');
      expect(textarea.value).toContain('"title": "Test Schema"');
      expect(textarea.value).toContain('"device_name"');
      expect(textarea.value).toContain('"Device Name"');
    });
  });

  it('should save new schema', async () => {
    const mockCreatedSchema = { id: 123 };
    mockInvoke.mockResolvedValueOnce(mockCreatedSchema);

    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    // Set schema details
    fireEvent.change(screen.getByPlaceholderText('Schema Name'), {
      target: { value: 'New Schema' }
    });

    fireEvent.change(screen.getByPlaceholderText('Schema Description'), {
      target: { value: 'A new schema' }
    });

    // Add a field first
    const addFieldButton = screen.getByText('Add Field');
    fireEvent.click(addFieldButton);

    await waitFor(() => {
      expect(screen.getByPlaceholderText('field_name')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByPlaceholderText('field_name'), {
      target: { value: 'test_field' }
    });

    fireEvent.change(screen.getByPlaceholderText('Field Label'), {
      target: { value: 'Test Field' }
    });

    const okButton = screen.getByRole('button', { name: /ok/i });
    fireEvent.click(okButton);

    // Save schema
    const saveButton = screen.getByText('Save Schema');
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_metadata_schema', {
        token: 'mock-token',
        name: 'New Schema',
        description: 'A new schema',
        schemaJson: expect.stringContaining('"test_field"'),
        assetTypeFilter: null
      });
    });

    expect(mockOnSave).toHaveBeenCalledWith(123);
  });

  it('should update existing schema', async () => {
    mockInvoke
      .mockResolvedValueOnce(mockExistingSchema) // Load existing schema
      .mockResolvedValueOnce({}); // Update schema

    render(
      <SchemaDesigner
        schemaId={1}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Existing Schema')).toBeInTheDocument();
    });

    // Modify schema name
    const nameInput = screen.getByDisplayValue('Existing Schema');
    fireEvent.change(nameInput, {
      target: { value: 'Modified Schema' }
    });

    // Save schema
    const saveButton = screen.getByText('Save Schema');
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('update_metadata_schema', {
        token: 'mock-token',
        schemaId: 1,
        name: 'Modified Schema',
        description: 'An existing schema for testing',
        schemaJson: expect.any(String),
        assetTypeFilter: 'device'
      });
    });
  });

  it('should handle validation errors when saving', async () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    // Try to save without schema name
    const saveButton = screen.getByText('Save Schema');
    fireEvent.click(saveButton);

    // Should show error message (mocked via Ant Design message)
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('should handle field deletion', async () => {
    mockInvoke.mockResolvedValueOnce(mockExistingSchema);

    render(
      <SchemaDesigner
        schemaId={1}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Name')).toBeInTheDocument();
    });

    // Find and click delete button for a field
    const deleteButtons = screen.getAllByRole('button', { name: /delete/i });
    fireEvent.click(deleteButtons[0]);

    // Confirm deletion in popconfirm
    const confirmButton = screen.getByText('Yes');
    fireEvent.click(confirmButton);

    await waitFor(() => {
      // Field should be removed from the list
      expect(screen.queryByText('Name')).not.toBeInTheDocument();
    });
  });

  it('should export schema', () => {
    // Mock URL.createObjectURL and document.createElement
    const mockCreateObjectURL = vi.fn(() => 'mock-url');
    const mockRevokeObjectURL = vi.fn();
    const mockClick = vi.fn();
    
    global.URL.createObjectURL = mockCreateObjectURL;
    global.URL.revokeObjectURL = mockRevokeObjectURL;
    
    const mockAnchor = {
      href: '',
      download: '',
      click: mockClick
    };
    
    vi.spyOn(document, 'createElement').mockReturnValue(mockAnchor as any);

    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    // Set schema name
    fireEvent.change(screen.getByPlaceholderText('Schema Name'), {
      target: { value: 'Export Test' }
    });

    // Click export button
    const exportButton = screen.getByText('Export');
    fireEvent.click(exportButton);

    expect(mockCreateObjectURL).toHaveBeenCalled();
    expect(mockClick).toHaveBeenCalled();
    expect(mockRevokeObjectURL).toHaveBeenCalled();
  });

  it('should show field type palette', () => {
    render(
      <SchemaDesigner
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    expect(screen.getByText('Field Types')).toBeInTheDocument();
    expect(screen.getByText('Text Input')).toBeInTheDocument();
    expect(screen.getByText('Text Area')).toBeInTheDocument();
    expect(screen.getByText('Number')).toBeInTheDocument();
    expect(screen.getByText('Date')).toBeInTheDocument();
    expect(screen.getByText('Dropdown')).toBeInTheDocument();
    expect(screen.getByText('Checkbox')).toBeInTheDocument();
  });
});