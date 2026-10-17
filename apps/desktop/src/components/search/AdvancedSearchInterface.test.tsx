import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import AdvancedSearchInterface from './AdvancedSearchInterface';
import useSearchStore from '../../store/search';
import { AssetSearchResult } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

// The panels are separate screens with their own tests; stub them here.
vi.mock('./FilterPanel', () => ({ default: () => <div data-testid="filter-panel" /> }));
vi.mock('./SearchResultsList', () => ({
  default: ({ results }: { results: AssetSearchResult[] }) => (
    <div data-testid="search-results-list">{results.map(r => r.asset_name).join(',')}</div>
  ),
}));
vi.mock('./SavedSearchManager', () => ({ default: () => <div data-testid="saved-search-manager" /> }));

const result: AssetSearchResult = {
  asset_id: 1,
  asset_name: 'PLC-001',
  asset_type: 'device',
  hierarchy_path: ['Plant A'],
  metadata_matches: [],
  relevance_score: 9,
  last_updated: '2024-05-01T10:00:00Z',
};

const resetStore = () =>
  useSearchStore.setState({
    query: { text_query: undefined, filters: [], hierarchy_scope: undefined, sort_by: 'Relevance', limit: 50, offset: 0 },
    results: [],
    loading: false,
    error: undefined,
    suggestions: [],
    totalResults: 0,
    searchTime: 0,
    isAdvancedMode: false,
  });

describe('AdvancedSearchInterface', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.removeItem('ferrocodex-search-history');
    resetStore();
  });

  it('renders the search card with controls', () => {
    render(<AdvancedSearchInterface />);

    expect(screen.getByText('Asset Search')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Search assets by name, metadata, or properties...')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Saved/ })).toBeInTheDocument();
    expect(screen.getByRole('switch')).not.toBeChecked();
    expect(screen.getByRole('button', { name: /Clear/ })).toBeDisabled();
    expect(screen.queryByTestId('filter-panel')).not.toBeInTheDocument();
  });

  it('shows the filter panel in advanced mode', () => {
    render(<AdvancedSearchInterface />);

    fireEvent.click(screen.getByRole('switch'));

    expect(screen.getByTestId('filter-panel')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Search with Filters/ })).toBeInTheDocument();
    expect(useSearchStore.getState().isAdvancedMode).toBe(true);
  });

  it('opens the saved searches drawer', async () => {
    render(<AdvancedSearchInterface />);

    fireEvent.click(screen.getByRole('button', { name: /Saved/ }));

    expect(await screen.findByText('Saved Searches')).toBeInTheDocument();
    expect(screen.getByTestId('saved-search-manager')).toBeInTheDocument();
  });

  it('renders the results summary and list inline when results exist', () => {
    useSearchStore.setState({ results: [result], totalResults: 1, searchTime: 12 });
    render(<AdvancedSearchInterface />);

    expect(screen.getByText('Found 1 results in 12ms')).toBeInTheDocument();
    expect(screen.getByTestId('search-results-list')).toHaveTextContent('PLC-001');
  });

  it('renders results in a drawer when requested', async () => {
    useSearchStore.setState({ results: [result], totalResults: 1, searchTime: 12 });
    render(<AdvancedSearchInterface showResultsInDrawer />);

    expect(await screen.findByText('Search Results (1 found)')).toBeInTheDocument();
    expect(screen.getByTestId('search-results-list')).toHaveTextContent('PLC-001');
    expect(screen.queryByText('Found 1 results in 12ms')).not.toBeInTheDocument();
  });
});
