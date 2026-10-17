import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import AssetSearchIntegration from './AssetSearchIntegration';
import { SearchContextProvider } from '../../contexts/SearchContext';
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

const renderIntegration = (props: React.ComponentProps<typeof AssetSearchIntegration> = {}) =>
  render(
    <MemoryRouter>
      <SearchContextProvider>
        <AssetSearchIntegration {...props} />
      </SearchContextProvider>
    </MemoryRouter>,
  );

describe('AssetSearchIntegration', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
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

  it('renders the search card and the idle prompt', () => {
    const onAssetCreate = vi.fn();
    renderIntegration({ onAssetCreate });

    expect(screen.getByTestId('metadata-search-bar')).toBeInTheDocument();
    expect(screen.getByText('Search Your Assets')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Create Asset/ }));
    expect(onAssetCreate).toHaveBeenCalled();
  });

  it('shows search status and results after a search', async () => {
    renderIntegration({ showCreateButton: false });

    fireEvent.click(screen.getByText('run search'));

    expect(await screen.findByText('Search Results')).toBeInTheDocument();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'search_assets_by_metadata',
        expect.objectContaining({ token: 'test-token', query: expect.objectContaining({ text_query: 'plc' }) }),
      ),
    );
    expect(await screen.findByText('PLC-001')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Create Asset/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Clear Search' }));
    expect(screen.getByText('Search Your Assets')).toBeInTheDocument();
  });

  it('opens the filter drawer with the filter builder', async () => {
    renderIntegration();

    fireEvent.click(screen.getByText('open filters'));

    expect(await screen.findByText('Advanced Search Filters')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Apply Filters/ })).toBeInTheDocument();
    expect(screen.getByTestId('metadata-filter-builder')).toBeInTheDocument();
  });
});
