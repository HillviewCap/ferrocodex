import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import FilterPanel from './FilterPanel';
import useSearchStore from '../../store/search';
import { FilterableField, FilterPreset, MetadataFilter } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const fields: FilterableField[] = [
  { field_name: 'location', field_type: 'text', schema_name: 'PLC Schema', usage_count: 12, sample_values: [] },
  { field_name: 'port', field_type: 'number', schema_name: 'PLC Schema', usage_count: 8, sample_values: [] },
];

const presets: FilterPreset[] = [
  {
    id: 1,
    name: 'Plant A devices',
    description: 'Devices in Plant A',
    filters: [{ field_name: 'location', field_type: 'text', operator: 'Contains', value: 'Plant A', logic_operator: 'And' }],
    created_by: 1,
    usage_count: 3,
    created_at: '2024-05-01T10:00:00Z',
  },
];

const textFilter: MetadataFilter = {
  field_name: 'location',
  field_type: 'text',
  operator: 'Contains',
  value: 'Plant',
  logic_operator: 'And',
};

const seedStore = (filters: MetadataFilter[]) =>
  useSearchStore.setState({
    query: { text_query: undefined, filters, hierarchy_scope: undefined, sort_by: 'Relevance', limit: 50, offset: 0 },
    availableFields: [],
    presets: [],
  });

describe('FilterPanel', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    seedStore([textFilter]);
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_filterable_metadata_fields') return fields;
      if (cmd === 'get_filter_presets') return presets;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads fields and presets on mount and renders the active filter row', async () => {
    render(<FilterPanel />);

    expect(screen.getByText('Advanced Filters')).toBeInTheDocument();
    expect(screen.getByText('1 active')).toBeInTheDocument();
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_filterable_metadata_fields'));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_filter_presets', {}));

    expect(await screen.findByText('location')).toBeInTheDocument();
    expect(screen.getByText('contains')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Plant')).toBeInTheDocument();
    expect(screen.getByText('Load preset')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Clear All/ })).toBeEnabled();
    expect(screen.getByRole('button', { name: /Save/ })).toBeEnabled();
  });

  it('adds a second filter with a logic operator and shows the query description', async () => {
    render(<FilterPanel />);
    await screen.findByText('location');

    fireEvent.click(screen.getByRole('button', { name: /Add Filter/ }));

    expect(useSearchStore.getState().query.filters).toHaveLength(2);
    expect(await screen.findByText('2 active')).toBeInTheDocument();
    expect(screen.getByText('AND')).toBeInTheDocument();

    // Logic builder switch reveals the query description
    fireEvent.click(screen.getByRole('switch'));
    expect(screen.getByText('Query:')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Clear All/ }));
    expect(useSearchStore.getState().query.filters).toHaveLength(0);
  });

  it('renders a compact range input for InRange number filters', async () => {
    seedStore([{ field_name: 'port', field_type: 'number', operator: 'InRange', value: [1, 10], logic_operator: 'And' }]);
    render(<FilterPanel />);

    expect(await screen.findByPlaceholderText('Min')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Max')).toBeInTheDocument();
    expect(screen.getByText('in range')).toBeInTheDocument();
  });

  it('opens the save-preset popover', async () => {
    render(<FilterPanel />);
    await screen.findByText('location');

    fireEvent.click(screen.getByRole('button', { name: /Save/ }));

    expect(await screen.findByText('Save Filter Preset')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Preset name')).toBeInTheDocument();
  });

  it('renders the compact variant', () => {
    render(<FilterPanel compact />);

    expect(screen.getByText('1 filter')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Add Filter/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Clear/ })).toBeInTheDocument();
    expect(screen.queryByText('Advanced Filters')).not.toBeInTheDocument();
  });
});
