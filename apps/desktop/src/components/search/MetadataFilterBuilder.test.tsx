import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import MetadataFilterBuilder from './MetadataFilterBuilder';
import { SearchContextProvider } from '../../contexts/SearchContext';
import useAuthStore from '../../store/auth';
import { FilterableField, FilterPreset, MetadataFilter } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const fields: FilterableField[] = [
  { field_name: 'location', field_type: 'text', schema_name: 'PLC Schema', usage_count: 12, sample_values: [] },
  { field_name: 'port', field_type: 'number', schema_name: 'PLC Schema', usage_count: 1, sample_values: [] },
];

const filters: MetadataFilter[] = [
  { field_name: 'location', field_type: 'text', operator: 'Contains', value: 'Plant', logic_operator: 'And' },
  { field_name: 'port', field_type: 'number', operator: 'InRange', value: { min: 1, max: 10 }, logic_operator: 'Or' },
];

const presets: FilterPreset[] = [
  {
    id: 1,
    name: 'Prod',
    description: 'Production devices',
    filters: [filters[0]],
    created_by: 1,
    usage_count: 2,
    created_at: '2024-05-01T10:00:00Z',
  },
];

const renderBuilder = (props: Partial<React.ComponentProps<typeof MetadataFilterBuilder>> = {}) =>
  render(
    <SearchContextProvider>
      <MetadataFilterBuilder filters={filters} availableFields={fields} onFiltersChange={vi.fn()} presets={presets} {...props} />
    </SearchContextProvider>,
  );

describe('MetadataFilterBuilder', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_filterable_metadata_fields') return fields;
      if (cmd === 'get_metadata_filter_presets') return presets;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders filters, logic operators, the compact range input and presets', () => {
    renderBuilder();

    expect(screen.getByText('Advanced Filters')).toBeInTheDocument();
    expect(screen.getByText('2 filters')).toBeInTheDocument();
    expect(screen.getByText('Filter Presets')).toBeInTheDocument();
    expect(screen.getByText('Prod (1)')).toBeInTheDocument();

    expect(screen.getAllByText('Field:')).toHaveLength(2);
    expect(screen.getAllByText('Operator:')).toHaveLength(2);
    expect(screen.getAllByText('Value:')).toHaveLength(2);
    expect(screen.getByText('Logic:')).toBeInTheDocument();
    expect(screen.getByText('AND')).toBeInTheDocument();
    expect(screen.getByText('OR')).toBeInTheDocument();
    expect(screen.getByText('NOT')).toBeInTheDocument();

    expect(screen.getByText('contains')).toBeInTheDocument();
    expect(screen.getByText('is in range')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Plant')).toBeInTheDocument();
    // Space.Compact range input (Input.Group replacement)
    expect(screen.getByPlaceholderText('Min')).toHaveValue('1');
    expect(screen.getByPlaceholderText('Max')).toHaveValue('10');

    expect(screen.getByText(/Type: text • Schema: PLC Schema/)).toBeInTheDocument();
    expect(screen.getByText(/Used in 1 asset$/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Add Filter/ })).toBeInTheDocument();
  });

  it('opens the save-preset modal listing the filters', async () => {
    renderBuilder();

    fireEvent.click(screen.getByRole('button', { name: /Save Preset/ }));

    expect(await screen.findByText('Save Filter Preset')).toBeInTheDocument();
    expect(screen.getByLabelText('Preset Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
    expect(screen.getByText('Filters to save (2):')).toBeInTheDocument();
  });

  it('renders the empty state when there are no filters', () => {
    renderBuilder({ filters: [], presets: [] });

    expect(screen.getByText('No filters applied')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save Preset/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /Clear All/ })).toBeDisabled();
    expect(screen.queryByText('Filter Presets')).not.toBeInTheDocument();
  });

  it('disables adding filters when no fields are available', () => {
    renderBuilder({ filters: [], availableFields: [] });

    expect(screen.getByRole('button', { name: /No filterable fields available/ })).toBeDisabled();
  });
});
