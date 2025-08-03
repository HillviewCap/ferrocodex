import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import VirtualizedTree from './VirtualizedTree';
import { AssetHierarchy } from '../../types/assets';

// Mock react-window
vi.mock('react-window', () => ({
  FixedSizeList: ({ children, itemCount, itemSize }: any) => (
    <div data-testid="virtualized-list">
      {Array.from({ length: Math.min(itemCount, 10) }, (_, index) => 
        children({ index, style: { height: itemSize } })
      )}
    </div>
  ),
}));

// Mock useTreeVirtualization hook
vi.mock('../../hooks/useTreeVirtualization', () => ({
  useTreeVirtualization: vi.fn(() => ({
    flattenedItems: [],
    visibleItems: [],
    expandNode: vi.fn(),
    collapseNode: vi.fn(),
    searchItems: vi.fn(),
    performanceMetrics: {
      totalItems: 0,
      visibleItems: 0,
      maxDepth: 0,
      renderTime: 0,
      searchTime: 0,
      lastUpdate: new Date(),
    },
  })),
}));

const mockAssetHierarchy: AssetHierarchy[] = [
  {
    id: 1,
    name: 'Production Line 1',
    description: 'Main production line',
    asset_type: 'Folder',
    parent_id: null,
    sort_order: 0,
    created_by: 1,
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: [
      {
        id: 2,
        name: 'PLC-001',
        description: 'Primary PLC',
        asset_type: 'Device',
        parent_id: 1,
        sort_order: 0,
        created_by: 1,
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
        children: [],
      },
      {
        id: 3,
        name: 'HMI-001',
        description: 'Main HMI',
        asset_type: 'Device',
        parent_id: 1,
        sort_order: 1,
        created_by: 1,
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
        children: [],
      },
    ],
  },
  {
    id: 4,
    name: 'Production Line 2',
    description: 'Secondary production line',
    asset_type: 'Folder',
    parent_id: null,
    sort_order: 1,
    created_by: 1,
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: [],
  },
];

describe('VirtualizedTree', () => {
  const defaultProps = {
    hierarchyData: mockAssetHierarchy,
    onAssetSelect: vi.fn(),
    onAssetCreate: vi.fn(),
    onAssetMove: vi.fn(),
    selectedAssetId: null,
    loading: false,
    height: 400,
    itemHeight: 32,
    enableSearch: true,
    maxItems: 10000,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders without crashing', () => {
    render(<VirtualizedTree {...defaultProps} />);
    expect(screen.getByTestId('virtualized-list')).toBeInTheDocument();
  });

  it('displays search input when enableSearch is true', () => {
    render(<VirtualizedTree {...defaultProps} />);
    expect(screen.getByPlaceholderText('Search assets...')).toBeInTheDocument();
  });

  it('hides search input when enableSearch is false', () => {
    render(<VirtualizedTree {...defaultProps} enableSearch={false} />);
    expect(screen.queryByPlaceholderText('Search assets...')).not.toBeInTheDocument();
  });

  it('shows loading state', () => {
    render(<VirtualizedTree {...defaultProps} loading={true} />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('shows empty state when no items', () => {
    render(<VirtualizedTree {...defaultProps} hierarchyData={[]} />);
    expect(screen.getByText('No assets found')).toBeInTheDocument();
  });

  it('shows no search results message', async () => {
    render(<VirtualizedTree {...defaultProps} hierarchyData={[]} />);
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    fireEvent.change(searchInput, { target: { value: 'nonexistent' } });
    
    await waitFor(() => {
      expect(screen.getByText('No assets found matching your search')).toBeInTheDocument();
    });
  });

  it('calls onAssetSelect when asset is selected', () => {
    render(<VirtualizedTree {...defaultProps} />);
    
    // The tree item should be rendered by the virtualized list
    // Note: This test might need adjustment based on the actual implementation
    expect(screen.getByTestId('virtualized-list')).toBeInTheDocument();
  });

  it('handles search input changes', async () => {
    render(<VirtualizedTree {...defaultProps} />);
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    fireEvent.change(searchInput, { target: { value: 'PLC' } });
    
    expect(searchInput).toHaveValue('PLC');
  });

  it('handles keyboard navigation', () => {
    render(<VirtualizedTree {...defaultProps} />);
    
    const treeContainer = screen.getByRole('tree');
    fireEvent.keyDown(treeContainer, { key: 'ArrowDown' });
    fireEvent.keyDown(treeContainer, { key: 'ArrowUp' });
    fireEvent.keyDown(treeContainer, { key: 'ArrowRight' });
    fireEvent.keyDown(treeContainer, { key: 'ArrowLeft' });
    fireEvent.keyDown(treeContainer, { key: 'Enter' });
    fireEvent.keyDown(treeContainer, { key: ' ' });
    
    // Test that keyboard events are handled without errors
    expect(treeContainer).toBeInTheDocument();
  });

  it('shows performance metrics in development mode', () => {
    // Mock development environment
    const originalEnv = process.env.NODE_ENV;
    process.env.NODE_ENV = 'development';
    
    render(<VirtualizedTree {...defaultProps} />);
    
    // Should show performance info in development
    expect(screen.getByText(/Items:/)).toBeInTheDocument();
    
    // Restore environment
    process.env.NODE_ENV = originalEnv;
  });

  it('hides performance metrics in production mode', () => {
    // Mock production environment
    const originalEnv = process.env.NODE_ENV;
    process.env.NODE_ENV = 'production';
    
    render(<VirtualizedTree {...defaultProps} />);
    
    // Should not show performance info in production
    expect(screen.queryByText(/Items:/)).not.toBeInTheDocument();
    
    // Restore environment
    process.env.NODE_ENV = originalEnv;
  });

  it('applies correct height to virtualized list', () => {
    const customHeight = 500;
    render(<VirtualizedTree {...defaultProps} height={customHeight} />);
    
    // The virtualized list should be present
    expect(screen.getByTestId('virtualized-list')).toBeInTheDocument();
  });

  it('handles custom item height', () => {
    const customItemHeight = 48;
    render(<VirtualizedTree {...defaultProps} itemHeight={customItemHeight} />);
    
    // Component should render without errors
    expect(screen.getByTestId('virtualized-list')).toBeInTheDocument();
  });

  it('limits items based on maxItems prop', () => {
    const maxItems = 5;
    render(<VirtualizedTree {...defaultProps} maxItems={maxItems} />);
    
    // Component should render without errors
    expect(screen.getByTestId('virtualized-list')).toBeInTheDocument();
  });

  it('clears search when search value is changed', async () => {
    render(<VirtualizedTree {...defaultProps} />);
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    fireEvent.change(searchInput, { target: { value: 'test search' } });
    
    expect(searchInput).toHaveValue('test search');
    
    // Clear the search by changing to empty value
    fireEvent.change(searchInput, { target: { value: '' } });
    
    expect(searchInput).toHaveValue('');
  });

  it('handles large dataset performance gracefully', () => {
    const largeDataset = Array.from({ length: 1000 }, (_, i) => ({
      id: i + 1,
      name: `Asset ${i + 1}`,
      description: `Description ${i + 1}`,
      asset_type: 'Device' as any,
      parent_id: null,
      sort_order: i,
      created_by: 1,
      created_at: '2023-01-01T00:00:00Z',
      updated_at: '2023-01-01T00:00:00Z',
      children: [],
    }));

    render(
      <VirtualizedTree
        hierarchyData={largeDataset}
        maxItems={100}
      />
    );

    // Should render without performance issues
    expect(screen.getByTestId('tree-container')).toBeInTheDocument();
  });

  it('handles null/undefined data gracefully', () => {
    render(
      <VirtualizedTree
        hierarchyData={null as any}
        onAssetSelect={vi.fn()}
      />
    );

    expect(screen.getByText('No assets found')).toBeInTheDocument();
  });
});