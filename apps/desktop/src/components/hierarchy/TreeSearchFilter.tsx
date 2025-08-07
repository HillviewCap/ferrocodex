import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { Input, Select, Space, Button, Tooltip, Typography } from 'antd';
import { 
  SearchOutlined, 
  ClearOutlined, 
  FolderOutlined, 
  ToolOutlined,
  FilterOutlined 
} from '@ant-design/icons';
import { AssetHierarchy, AssetType } from '../../types/assets';
// Simple debounce utility
const debounce = <T extends (...args: any[]) => any>(
  func: T,
  wait: number
): ((...args: Parameters<T>) => void) => {
  let timeout: NodeJS.Timeout;
  return (...args: Parameters<T>) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
};

const { Search } = Input;
const { Option } = Select;
const { Text } = Typography;

export interface TreeSearchFilterProps {
  hierarchyData: AssetHierarchy[];
  onSearchChange: (query: string, filteredNodes: Set<number>, expandedKeys: React.Key[]) => void;
  onFilterChange: (assetType: AssetType | 'All') => void;
  placeholder?: string;
  showTypeFilter?: boolean;
  debounceMs?: number;
}

export interface SearchResult {
  matchedNodes: Set<number>;
  expandedKeys: React.Key[];
  searchQuery: string;
  assetTypeFilter: AssetType | 'All';
}

export const TreeSearchFilter: React.FC<TreeSearchFilterProps> = ({
  hierarchyData,
  onSearchChange,
  onFilterChange,
  placeholder = "Search assets...",
  showTypeFilter = true,
  debounceMs = 300,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [assetTypeFilter, setAssetTypeFilter] = useState<AssetType | 'All'>('All');
  const [searchResults, setSearchResults] = useState<SearchResult | null>(null);

  // Search function that finds matching nodes and their paths
  const performSearch = useCallback((query: string, typeFilter: AssetType | 'All' = 'All') => {
    if (!query.trim() && typeFilter === 'All') {
      setSearchResults(null);
      onSearchChange('', new Set(), []);
      return;
    }

    const matchedNodes = new Set<number>();
    const expandedKeys = new Set<React.Key>();
    const queryLower = query.toLowerCase();

    // Recursive search function
    const searchInHierarchy = (nodes: AssetHierarchy[], parentPath: AssetHierarchy[] = []): boolean => {
      let hasMatches = false;

      for (const node of nodes) {
        const currentPath = [...parentPath, node];
        let nodeMatches = false;

        // Check if node matches search criteria
        const nameMatches = !query.trim() || node.name.toLowerCase().includes(queryLower);
        const descriptionMatches = !query.trim() || node.description.toLowerCase().includes(queryLower);
        const typeMatches = typeFilter === 'All' || node.asset_type === typeFilter;
        
        const isDirectMatch = (nameMatches || descriptionMatches) && typeMatches;

        // Search in children
        const hasChildMatches = node.children.length > 0 && 
          searchInHierarchy(node.children, currentPath);

        // Node should be included if it matches directly or has matching children
        if (isDirectMatch || hasChildMatches) {
          nodeMatches = true;
          hasMatches = true;
          matchedNodes.add(node.id);

          // Add all parent nodes to expanded keys to show the path
          currentPath.forEach(pathNode => {
            if (pathNode.asset_type === 'Folder') {
              expandedKeys.add(pathNode.id.toString());
            }
          });
        }
      }

      return hasMatches;
    };

    searchInHierarchy(hierarchyData);

    const result: SearchResult = {
      matchedNodes,
      expandedKeys: Array.from(expandedKeys),
      searchQuery: query,
      assetTypeFilter: typeFilter,
    };

    setSearchResults(result);
    onSearchChange(query, matchedNodes, Array.from(expandedKeys));
  }, [hierarchyData, onSearchChange]);

  // Debounced search
  const debouncedSearch = useMemo(
    () => debounce((query: string, typeFilter: AssetType | 'All') => {
      performSearch(query, typeFilter);
    }, debounceMs),
    [performSearch, debounceMs]
  );

  // Handle search input change
  const handleSearchChange = (value: string) => {
    setSearchQuery(value);
    debouncedSearch(value, assetTypeFilter);
  };

  // Handle type filter change
  const handleTypeFilterChange = (value: AssetType | 'All') => {
    setAssetTypeFilter(value);
    onFilterChange(value);
    debouncedSearch(searchQuery, value);
  };

  // Handle clear search
  const handleClear = () => {
    setSearchQuery('');
    setAssetTypeFilter('All');
    setSearchResults(null);
    onSearchChange('', new Set(), []);
    onFilterChange('All');
  };

  // Get search stats
  const getSearchStats = () => {
    if (!searchResults || (!searchQuery.trim() && assetTypeFilter === 'All')) {
      return null;
    }

    const matchCount = searchResults.matchedNodes.size;
    const hasQuery = searchQuery.trim().length > 0;
    const hasFilter = assetTypeFilter !== 'All';

    let description = '';
    if (hasQuery && hasFilter) {
      description = `Found ${matchCount} ${assetTypeFilter.toLowerCase()}${matchCount !== 1 ? 's' : ''} matching "${searchQuery}"`;
    } else if (hasQuery) {
      description = `Found ${matchCount} asset${matchCount !== 1 ? 's' : ''} matching "${searchQuery}"`;
    } else if (hasFilter) {
      description = `Showing ${matchCount} ${assetTypeFilter.toLowerCase()}${matchCount !== 1 ? 's' : ''}`;
    }

    return { count: matchCount, description };
  };

  const searchStats = getSearchStats();
  const hasActiveSearch = searchQuery.trim().length > 0 || assetTypeFilter !== 'All';

  return (
    <div style={{ marginBottom: '16px' }}>
      <Space.Compact style={{ width: '100%', marginBottom: '8px' }}>
        <Search
          placeholder={placeholder}
          value={searchQuery}
          onChange={(e) => handleSearchChange(e.target.value)}
          onSearch={(value) => performSearch(value, assetTypeFilter)}
          allowClear
          style={{ flex: 1 }}
          enterButton={
            <Button 
              type={hasActiveSearch ? 'primary' : 'default'} 
              icon={<SearchOutlined />}
            />
          }
        />

        {showTypeFilter && (
          <Select
            value={assetTypeFilter}
            onChange={handleTypeFilterChange}
            style={{ width: 120 }}
          >
            <Option value="All">
              <Space>
                <FilterOutlined />
                All
              </Space>
            </Option>
            <Option value="Folder">
              <Space>
                <FolderOutlined style={{ color: '#1890ff' }} />
                Folders
              </Space>
            </Option>
            <Option value="Device">
              <Space>
                <ToolOutlined style={{ color: '#52c41a' }} />
                Devices
              </Space>
            </Option>
          </Select>
        )}

        {hasActiveSearch && (
          <Tooltip title="Clear search and filters">
            <Button
              icon={<ClearOutlined />}
              onClick={handleClear}
            />
          </Tooltip>
        )}
      </Space.Compact>

      {/* Search results info */}
      {searchStats && (
        <div style={{ fontSize: '12px', color: '#666', marginBottom: '8px' }}>
          <Text type="secondary">{searchStats.description}</Text>
          {searchStats.count === 0 && (
            <Text type="secondary" style={{ marginLeft: '8px' }}>
              Try adjusting your search terms or filters.
            </Text>
          )}
        </div>
      )}

      {/* Search tips */}
      {!hasActiveSearch && (
        <div style={{ fontSize: '12px', color: '#999' }}>
          <Text type="secondary">
            Search by asset name or description. Use filters to narrow by type.
          </Text>
        </div>
      )}
    </div>
  );
};

export default TreeSearchFilter;