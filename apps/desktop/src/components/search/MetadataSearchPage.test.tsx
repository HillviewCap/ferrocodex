import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import MetadataSearchPage from './MetadataSearchPage';
import useAuthStore from '../../store/auth';
import { AssetSearchResult } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('./MetadataSearchBar', () => ({
  default: ({ onSearch, onAdvancedClick }: { onSearch: (q: string) => void; onAdvancedClick: () => void }) => (
    <div data-testid="metadata-search-bar">
      <button onClick={() => onSearch('plc')}>run search</button>
      <button onClick={onAdvancedClick}>open filters</button>
    </div>
  ),
}));
vi.mock('./MetadataSearchResults', () => ({
  default: ({ results }: { results: AssetSearchResult[] }) => (
    <div data-testid="metadata-search-results">{results.map(r => r.asset_name).join(',')}</div>
  ),
}));
vi.mock('./MetadataFilterBuilder', () => ({ default: () => <div data-testid="metadata-filter-builder" /> }));

const result: AssetSearchResult = {
  asset_id: 1,
  asset_name: 'PLC-001',
  asset_type: 'device',
  hierarchy_path: [],
  metadata_matches: [],
  relevance_score: 9,
  last_updated: '2024-05-01T10:00:00Z',
};

const renderPage = (props: React.ComponentProps<typeof MetadataSearchPage> = {}) =>
  render(
    <MemoryRouter>
      <MetadataSearchPage {...props} />
    </MemoryRouter>,
  );

describe('MetadataSearchPage', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_filterable_metadata_fields':
        case 'get_metadata_filter_presets':
          return [];
        case 'search_assets_by_metadata':
          return [result];
        default:
          throw new Error(`Unexpected command: ${cmd}`);
      }
    });
  });

  it('renders the search bar and results inside its own search context', async () => {
    renderPage();

    expect(screen.getByTestId('metadata-search-bar')).toBeInTheDocument();
    expect(screen.getByTestId('metadata-search-results')).toBeInTheDocument();
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_metadata_filter_presets', { token: 'test-token' }));

    fireEvent.click(screen.getByText('run search'));
    expect(await screen.findByText('PLC-001')).toBeInTheDocument();
  });

  it('opens the filter drawer with the Apply Filters action', async () => {
    renderPage();

    fireEvent.click(screen.getByText('open filters'));

    expect(await screen.findByText('Advanced Filters')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Apply Filters/ })).toBeInTheDocument();
    expect(screen.getByTestId('metadata-filter-builder')).toBeInTheDocument();
  });

  it('runs the initial query and shows the analytics button when enabled', async () => {
    renderPage({ initialQuery: 'hmi', showAnalytics: true });

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'search_assets_by_metadata',
        expect.objectContaining({ query: expect.objectContaining({ text_query: 'hmi' }) }),
      ),
    );
    expect(screen.getByRole('button', { name: /Analytics/ })).toBeInTheDocument();
  });
});
