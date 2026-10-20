import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import MetadataDisplay from './MetadataDisplay';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('../forms/DynamicMetadataForm', () => ({
  default: ({ onCancel }: { onCancel: () => void }) => (
    <div data-testid="dynamic-metadata-form">
      <button onClick={onCancel}>cancel edit</button>
    </div>
  ),
}));

const schema = {
  id: 1,
  name: 'PLC Schema',
  description: 'Schema for PLCs',
  schema_json: JSON.stringify({
    properties: {
      ip_address: { type: 'string', title: 'IP Address', category: 'Network' },
      port: { type: 'number', title: 'Port', category: 'Network' },
      is_managed: { type: 'boolean', title: 'Managed', category: 'General' },
      install_date: { type: 'string', format: 'date', title: 'Install Date', category: 'General' },
      tags: { type: 'array', title: 'Tags', category: 'General' },
    },
    fieldOrder: ['ip_address', 'port', 'is_managed', 'install_date', 'tags'],
  }),
};

const fullMetadata = {
  current: {
    asset_id: 42,
    schema_id: 1,
    metadata_values_json: JSON.stringify({
      ip_address: '10.0.0.5',
      port: 502,
      is_managed: true,
      install_date: '2024-03-15',
      tags: ['critical', 'line-1'],
    }),
    schema_version: 3,
    created_at: '2024-03-15T10:00:00Z',
    updated_at: '2024-04-01T12:30:00Z',
    created_by: 7,
  },
  schema,
  history: [],
  validation_status: null,
  related_assets: [],
};

const mockCommands = (overrides: Record<string, unknown> = {}) => {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    const responses: Record<string, unknown> = {
      get_metadata_schemas: [{ ...schema, name: 'Default Equipment Schema', is_system_template: true }],
      get_metadata_schema_by_id: schema,
      get_asset_metadata_full: fullMetadata,
      ...overrides,
    };
    if (cmd in responses) return responses[cmd];
    throw new Error(`Unexpected command: ${cmd}`);
  });
};

describe('MetadataDisplay', () => {
  useConsoleGuard({
    // Pre-existing, not a deprecation: the field Descriptions use a responsive `column`
    // map, and antd's useBreakpoint reports no screens on the very first render, so the
    // 2-span array field overflows the fallback 3-column row and antd logs a usage
    // warning. This happened before the items migration and also in the real app.
    ignore: [/Sum of column `span` in a line not match `column` of Descriptions/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
  });

  it('renders metadata grouped by category using Descriptions items', async () => {
    mockCommands();
    render(<MetadataDisplay assetId={42} schemaId={1} />);

    expect(await screen.findByText('PLC Schema')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_metadata_schema_by_id', { token: 'test-token', schemaId: 1 });
    expect(invoke).toHaveBeenCalledWith('get_asset_metadata_full', {
      token: 'test-token',
      assetId: 42,
      includeHistory: false,
    });

    // Category cards
    expect(screen.getByText('Network')).toBeInTheDocument();
    expect(screen.getByText('General')).toBeInTheDocument();

    // Labels and formatted values
    expect(screen.getByText('IP Address')).toBeInTheDocument();
    expect(screen.getByText('10.0.0.5')).toBeInTheDocument();
    expect(screen.getByText('Port')).toBeInTheDocument();
    expect(screen.getByText('502')).toBeInTheDocument();
    expect(screen.getByText('Managed')).toBeInTheDocument();
    expect(screen.getByText('Yes')).toBeInTheDocument();
    expect(screen.getByText('Install Date')).toBeInTheDocument();
    expect(screen.getByText('2024-03-15')).toBeInTheDocument();
    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.getByText('line-1')).toBeInTheDocument();

    // Additional Information card
    expect(screen.getByText('Additional Information')).toBeInTheDocument();
    expect(screen.getByText('Schema Version')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('Schema ID')).toBeInTheDocument();
    expect(screen.getByText('Last Updated')).toBeInTheDocument();
    expect(screen.getByText('Updated By')).toBeInTheDocument();
    expect(screen.getByText('User #7')).toBeInTheDocument();
  });

  it('shows the empty state when the asset has no metadata and opens the edit form', async () => {
    mockCommands({ get_asset_metadata_full: { ...fullMetadata, current: null } });
    render(<MetadataDisplay assetId={42} location="Plant 1 › Line 3" />);

    expect(await screen.findByText('No additional information recorded for this asset')).toBeInTheDocument();

    expect(screen.getByText('Plant 1 › Line 3')).toBeInTheDocument();
    // No schema was passed, so the system default schema is resolved
    expect(invoke).toHaveBeenCalledWith('get_metadata_schemas', { token: 'test-token', assetTypeFilter: null });

    fireEvent.click(screen.getByRole('button', { name: /Add Information/ }));
    expect(screen.getByTestId('dynamic-metadata-form')).toBeInTheDocument();

    fireEvent.click(screen.getByText('cancel edit'));
    expect(screen.getByText('No additional information recorded for this asset')).toBeInTheDocument();
  });

  it('shows Serial Number in the Device group, ahead of the other groups', async () => {
    const deviceSchema = {
      ...schema,
      name: 'Default Equipment Schema',
      is_system_template: true,
      schema_json: JSON.stringify({
        properties: {
          ip_address: { type: 'string', title: 'IP Address', category: 'Network' },
          serial_number: { type: 'string', title: 'Serial Number', category: 'Device' },
          notes: { type: 'string', title: 'Notes', category: 'Operational' },
        },
        fieldOrder: ['ip_address', 'serial_number', 'notes'],
      }),
    };
    mockCommands({
      get_asset_metadata_full: {
        ...fullMetadata,
        current: { ...fullMetadata.current, metadata_values_json: JSON.stringify({ serial_number: 'SN-1/A' }) },
      },
      get_metadata_schema_by_id: deviceSchema,
    });
    render(<MetadataDisplay assetId={42} location="Root" />);

    expect(await screen.findByText('Serial Number')).toBeInTheDocument();
    expect(screen.getByText('SN-1/A')).toBeInTheDocument();
    const titles = screen.getAllByText(/^(Device|Network|Operational)$/).map(el => el.textContent);
    expect(titles).toEqual(['Device', 'Network', 'Operational']);
    expect(screen.getByTestId('asset-location')).toHaveTextContent('Location:Root');
  });

  it('uses the schema referenced by the asset values over the default', async () => {
    mockCommands({
      get_asset_metadata_full: { ...fullMetadata, current: { ...fullMetadata.current, schema_id: 7 } },
    });
    render(<MetadataDisplay assetId={42} />);

    await screen.findByText('PLC Schema');
    expect(invoke).toHaveBeenCalledWith('get_metadata_schema_by_id', { token: 'test-token', schemaId: 7 });
    expect(invoke).not.toHaveBeenCalledWith('get_metadata_schemas', expect.anything());
  });

  it('shows an error alert when the schema cannot be loaded', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('schema missing'));
    render(<MetadataDisplay assetId={42} />);

    expect(await screen.findByText('Error Loading Additional Information')).toBeInTheDocument();
    expect(screen.getByText('schema missing')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
  });
});
