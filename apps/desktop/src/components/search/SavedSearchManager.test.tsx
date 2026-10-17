import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { notification } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import SavedSearchManager from './SavedSearchManager';
import useSearchStore from '../../store/search';
import { FilterPreset, MetadataFilter } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const textFilter: MetadataFilter = {
  field_name: 'location',
  field_type: 'text',
  operator: 'Contains',
  value: 'Plant',
  logic_operator: 'And',
};

const presets: FilterPreset[] = [
  {
    id: 1,
    name: 'Plant A devices',
    description: 'Devices in Plant A',
    filters: [textFilter],
    created_by: 1,
    usage_count: 3,
    created_at: '2024-05-01T10:00:00Z',
  },
  {
    id: 2,
    name: 'Unused preset',
    description: '',
    filters: [textFilter, { ...textFilter, field_name: 'vendor', value: 'Siemens' }],
    created_by: 1,
    usage_count: 0,
    created_at: '2024-04-01T10:00:00Z',
  },
];

const seedStore = (filters: MetadataFilter[]) =>
  useSearchStore.setState({
    query: { text_query: undefined, filters, hierarchy_scope: undefined, sort_by: 'Relevance', limit: 50, offset: 0 },
    presets: [],
  });

describe('SavedSearchManager', () => {
  useConsoleGuard({
    // List migration was explicitly deferred by d052492; Listy is not a drop-in.
    ignore: [/\[antd: List\]/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    seedStore([textFilter]);
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_filter_presets') return presets;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  afterEach(() => {
    act(() => {
      notification.destroy();
    });
  });

  it('loads presets and renders them with the current query', async () => {
    render(<SavedSearchManager />);

    expect(screen.getByText('Saved Searches')).toBeInTheDocument();
    expect(await screen.findByText('2 saved presets')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_filter_presets', {});

    expect(screen.getByText('Plant A devices')).toBeInTheDocument();
    expect(screen.getByText('Devices in Plant A')).toBeInTheDocument();
    expect(screen.getByText('3 uses')).toBeInTheDocument();
    expect(screen.getByText('Unused preset')).toBeInTheDocument();
    expect(screen.getAllByText(/^Created /)).toHaveLength(2);

    expect(screen.getByText('Current Query')).toBeInTheDocument();
    expect(screen.getByText('Active filters')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save Current/ })).toBeEnabled();
  });

  it('applies a preset and notifies the caller', async () => {
    const onSearchSelect = vi.fn();
    render(<SavedSearchManager onSearchSelect={onSearchSelect} />);
    await screen.findByText('Plant A devices');

    fireEvent.click(screen.getAllByRole('button', { name: /Apply/ })[0]);

    expect(onSearchSelect).toHaveBeenCalledWith(expect.objectContaining({ id: 1 }));
    expect(useSearchStore.getState().query.filters).toEqual(presets[0].filters);
    expect(await screen.findByText('Preset Applied')).toBeInTheDocument();
  });

  it('opens the create and edit modals', async () => {
    render(<SavedSearchManager />);
    await screen.findByText('Plant A devices');

    fireEvent.click(screen.getByRole('button', { name: /Save Current/ }));
    expect(await screen.findByText('Save Search Preset')).toBeInTheDocument();
    expect(screen.getByText('Current Filters (1):')).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole('button', { name: 'edit' })[0]);
    expect(await screen.findByText('Edit Search Preset')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Plant A devices')).toBeInTheDocument();
    expect(screen.getByText('Preset Filters (1):')).toBeInTheDocument();
  });

  it('renders the empty state when nothing is saved', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    seedStore([]);
    render(<SavedSearchManager />);

    expect(await screen.findByText('No saved searches yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save Current/ })).toBeDisabled();
    expect(screen.queryByText('Current Query')).not.toBeInTheDocument();
  });
});
