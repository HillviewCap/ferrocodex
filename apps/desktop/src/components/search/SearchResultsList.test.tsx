import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import SearchResultsList from './SearchResultsList';
import useSearchStore from '../../store/search';
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

describe('SearchResultsList', () => {
  useConsoleGuard({
    // List migration was explicitly deferred by d052492; Listy is not a drop-in.
    ignore: [/\[antd: List\]/],
  });

  beforeEach(() => {
    useSearchStore.setState({
      query: { text_query: undefined, filters: [], hierarchy_scope: undefined, sort_by: 'Relevance', limit: 50, offset: 0 },
    });
  });

  it('renders the results card, breadcrumb paths and match tags', () => {
    render(<SearchResultsList results={results} totalResults={2} searchTime={12} />);

    expect(screen.getByText('Search Results')).toBeInTheDocument();
    expect(screen.getByText('Found in 12ms')).toBeInTheDocument();
    expect(screen.getByText('Relevance')).toBeInTheDocument();

    expect(screen.getByRole('button', { name: 'PLC-001' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Control Room' })).toBeInTheDocument();
    expect(screen.getByText('device')).toBeInTheDocument();
    expect(screen.getByText('85%')).toBeInTheDocument();

    // Breadcrumb items from hierarchy_path ("Plant A" also appears in a match tag)
    const breadcrumb = within(document.querySelector('.ant-breadcrumb') as HTMLElement);
    expect(breadcrumb.getByText('Plant A')).toBeInTheDocument();
    expect(breadcrumb.getByText('Line 1')).toBeInTheDocument();

    expect(screen.getByText(/location:/)).toBeInTheDocument();
    expect(screen.getByText(/vendor:/)).toBeInTheDocument();
    expect(screen.getAllByText(/Last updated /)).toHaveLength(2);
  });

  it('expands metadata details with the collapse and Descriptions items', () => {
    const onAssetClick = vi.fn();
    render(<SearchResultsList results={results} totalResults={2} showMetadataDetails onAssetClick={onAssetClick} />);

    fireEvent.click(screen.getAllByRole('button', { name: /More/ })[0]);
    fireEvent.click(screen.getByText('Metadata Details'));

    for (const label of ['Asset ID', 'Asset Type', 'Relevance Score', 'Full Path']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText('8.50 (85%)')).toBeInTheDocument();
    expect(screen.getByText('Plant A > Line 1')).toBeInTheDocument();
    expect(screen.getByText('Matched Fields')).toBeInTheDocument();
    expect(screen.getByText('ExactMatch')).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole('button', { name: /View/ })[0]);
    expect(onAssetClick).toHaveBeenCalledWith(1);
  });

  it('renders loading, error and empty states', () => {
    const { rerender } = render(<SearchResultsList results={[]} loading />);
    expect(screen.getByText('Searching assets...')).toBeInTheDocument();

    rerender(<SearchResultsList results={[]} error="backend unavailable" />);
    expect(screen.getByText('Search Error')).toBeInTheDocument();
    expect(screen.getByText('backend unavailable')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();

    rerender(<SearchResultsList results={[]} />);
    expect(screen.getByText('No assets found')).toBeInTheDocument();
  });
});
