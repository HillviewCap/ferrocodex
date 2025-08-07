import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TreeSearchFilter } from '../TreeSearchFilter';
import { AssetHierarchy, AssetType } from '../../../types/assets';

// Mock data
const mockHierarchyData: AssetHierarchy[] = [
  {
    id: 1,
    name: 'Production Line A',
    description: 'Main production line',
    asset_type: 'Folder',
    parent_id: null,
    sort_order: 1,
    created_by: 1,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    children: [
      {
        id: 2,
        name: 'PLC Controller',
        description: 'Primary PLC controller',
        asset_type: 'Device',
        parent_id: 1,
        sort_order: 1,
        created_by: 1,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        children: [],
      },
      {
        id: 3,
        name: 'HMI Station',
        description: 'Human Machine Interface',
        asset_type: 'Device',
        parent_id: 1,
        sort_order: 2,
        created_by: 1,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        children: [],
      },
    ],
  },
  {
    id: 4,
    name: 'Quality Control',
    description: 'Quality control systems',
    asset_type: 'Folder',
    parent_id: null,
    sort_order: 2,
    created_by: 1,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    children: [
      {
        id: 5,
        name: 'Scanner Unit',
        description: 'Barcode scanner for quality checks',
        asset_type: 'Device',
        parent_id: 4,
        sort_order: 1,
        created_by: 1,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        children: [],
      },
    ],
  },
];

describe('TreeSearchFilter', () => {
  const mockOnSearchChange = vi.fn();
  const mockOnFilterChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  const renderComponent = (props = {}) => {
    return render(
      <TreeSearchFilter
        hierarchyData={mockHierarchyData}
        onSearchChange={mockOnSearchChange}
        onFilterChange={mockOnFilterChange}
        {...props}
      />
    );
  };

  it('renders search input with placeholder', () => {
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    expect(searchInput).toBeInTheDocument();
  });

  it('renders custom placeholder when provided', () => {
    renderComponent({ placeholder: 'Custom search placeholder' });
    
    const searchInput = screen.getByPlaceholderText('Custom search placeholder');
    expect(searchInput).toBeInTheDocument();
  });

  it('renders type filter dropdown when showTypeFilter is true', () => {
    renderComponent({ showTypeFilter: true });
    
    const filterDropdown = screen.getByRole('combobox');
    expect(filterDropdown).toBeInTheDocument();
  });

  it('does not render type filter dropdown when showTypeFilter is false', () => {
    renderComponent({ showTypeFilter: false });
    
    const filterDropdowns = screen.queryAllByRole('combobox');
    expect(filterDropdowns).toHaveLength(0);
  });

  it('calls onSearchChange when typing in search input', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'PLC');

    // Wait for debounced search
    await waitFor(() => {
      expect(mockOnSearchChange).toHaveBeenCalled();
    }, { timeout: 500 });
  });

  it('calls onFilterChange when changing asset type filter', async () => {
    const user = userEvent.setup();
    renderComponent({ showTypeFilter: true });
    
    const filterDropdown = screen.getByRole('combobox');
    await user.click(filterDropdown);
    
    const deviceOption = screen.getByText('Devices');
    await user.click(deviceOption);

    expect(mockOnFilterChange).toHaveBeenCalledWith('Device');
  });

  it('shows search results count when search is active', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'PLC');

    // Wait for search results to be displayed
    await waitFor(() => {
      const resultsText = screen.queryByText(/Found \d+ asset/);
      expect(resultsText).toBeInTheDocument();
    }, { timeout: 500 });
  });

  it('shows clear button when search is active', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'test');

    // Clear button should be visible (it's part of the Input component)
    const clearButton = screen.getByRole('button', { name: /clear/i });
    expect(clearButton).toBeInTheDocument();
  });

  it('clears search when clear button is clicked', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'test');
    
    const clearButton = screen.getByRole('button', { name: /clear/i });
    await user.click(clearButton);

    expect(searchInput).toHaveValue('');
  });

  it('shows search tips when no search is active', () => {
    renderComponent();
    
    const searchTips = screen.getByText(/Search by asset name or description/);
    expect(searchTips).toBeInTheDocument();
  });

  it('filters assets by name correctly', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'PLC');

    await waitFor(() => {
      expect(mockOnSearchChange).toHaveBeenCalledWith(
        'PLC',
        expect.any(Set),
        expect.any(Array)
      );
    }, { timeout: 500 });

    // Verify that the search found the PLC Controller
    const lastCall = mockOnSearchChange.mock.calls[mockOnSearchChange.mock.calls.length - 1];
    const matchedNodes = lastCall[1] as Set<number>;
    expect(matchedNodes.has(2)).toBe(true); // PLC Controller ID
  });

  it('filters assets by description correctly', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'quality');

    await waitFor(() => {
      expect(mockOnSearchChange).toHaveBeenCalled();
    }, { timeout: 500 });

    // Verify that the search found assets with "quality" in description
    const lastCall = mockOnSearchChange.mock.calls[mockOnSearchChange.mock.calls.length - 1];
    const matchedNodes = lastCall[1] as Set<number>;
    expect(matchedNodes.has(4)).toBe(true); // Quality Control folder
    expect(matchedNodes.has(5)).toBe(true); // Scanner Unit (has quality in description)
  });

  it('expands parent folders when search results are found', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'Scanner');

    await waitFor(() => {
      expect(mockOnSearchChange).toHaveBeenCalled();
    }, { timeout: 500 });

    // Verify that parent folders are included in expanded keys
    const lastCall = mockOnSearchChange.mock.calls[mockOnSearchChange.mock.calls.length - 1];
    const expandedKeys = lastCall[2] as React.Key[];
    expect(expandedKeys).toContain('4'); // Quality Control folder should be expanded
  });

  it('handles empty search gracefully', async () => {
    const user = userEvent.setup();
    renderComponent();
    
    const searchInput = screen.getByPlaceholderText('Search assets...');
    await user.type(searchInput, 'nonexistent');

    await waitFor(() => {
      expect(mockOnSearchChange).toHaveBeenCalled();
    }, { timeout: 500 });

    // Should show message about no results
    const noResultsText = screen.getByText(/Try adjusting your search terms/);
    expect(noResultsText).toBeInTheDocument();
  });
});