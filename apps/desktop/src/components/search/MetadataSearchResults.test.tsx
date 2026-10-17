import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import MetadataSearchResults from './MetadataSearchResults';
import { SearchContextProvider } from '../../contexts/SearchContext';
import useAuthStore from '../../store/auth';
import { AssetSearchResult } from '../../types/search';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const results: AssetSearchResult[] = [
  {
    asset_id: 1,
    asset_name: 'PLC-001',
    asset_type: 'device',
    hierarchy_path: ['Plant A', 'Line 1'],
    metadata_matches: [
      { field_name: 'location', field_value: 'Plant A', highlighted_value: 'Plant A', schema_name: 'PLC Schema', match_type: 'ExactMatch' },
      { field_name: 'vendor', field_value: 'Siemens', highlighted_value: 'Siemens', schema_name: 'PLC Schema', match_type: 'PartialMatch' },
    ],
    relevance_score: 8.5,
    last_updated: '2024-05-01T10:00:00Z',
  },
  {
    asset_id: 2,
    asset_name: 'Control Room',
    asset_type: 'folder',
    hierarchy_path: [],
    metadata_matches: [],
    relevance_score: 2,
    last_updated: '2024-04-01T10:00:00Z',
  },
];

const renderResults = (props: Partial<React.ComponentProps<typeof MetadataSearchResults>> = {}) =>
  render(
    <SearchContextProvider>
      <MetadataSearchResults results={results} totalResults={2} searchTime={12} {...props} />
    </SearchContextProvider>,
  );

describe('MetadataSearchResults', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it('renders the summary, each result, and hierarchy breadcrumbs', () => {
    renderResults();

    expect(screen.getByText('2 results')).toBeInTheDocument();
    expect(screen.getByText('(12ms)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Copy Results/ })).toBeInTheDocument();

    expect(screen.getByText('PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Control Room')).toBeInTheDocument();
    expect(screen.getByText('ID: 1')).toBeInTheDocument();
    expect(screen.getByText('device')).toBeInTheDocument();

    // Breadcrumb items
    expect(screen.getByText('Plant A')).toBeInTheDocument();
    expect(screen.getByText('Line 1')).toBeInTheDocument();
    expect(screen.getByText('2 metadata matches')).toBeInTheDocument();
  });

  it('expands metadata matches and triggers callbacks', () => {
    const onAssetClick = vi.fn();
    renderResults({ onAssetClick });

    fireEvent.click(screen.getAllByRole('button', { name: /More/ })[0]);
    expect(screen.getByText('Metadata Matches (2)')).toBeInTheDocument();
    expect(screen.getByText('location')).toBeInTheDocument();
    expect(screen.getByText('vendor')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Less/ })).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole('button', { name: /View Details/ })[0]);
    expect(onAssetClick).toHaveBeenCalledWith(1);
  });

  it('renders loading, error and empty states', () => {
    const { rerender } = renderResults({ loading: true });
    expect(screen.getByText('Searching assets...')).toBeInTheDocument();

    rerender(
      <SearchContextProvider>
        <MetadataSearchResults results={[]} error="backend unavailable" />
      </SearchContextProvider>,
    );
    expect(screen.getByText('Search Error')).toBeInTheDocument();
    expect(screen.getByText('backend unavailable')).toBeInTheDocument();

    rerender(
      <SearchContextProvider>
        <MetadataSearchResults results={[]} />
      </SearchContextProvider>,
    );
    expect(screen.getByText('No assets found matching your search criteria')).toBeInTheDocument();
  });
});
