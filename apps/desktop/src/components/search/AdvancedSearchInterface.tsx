import React, { useState, useEffect, useCallback } from 'react';
import { 
  Input, 
  Button, 
  Card, 
  Row, 
  Col, 
  Space, 
  Divider, 
  Switch, 
  Tooltip,
  Drawer,
  AutoComplete,
  Tag,
  notification
} from 'antd';
import { 
  SearchOutlined, 
  FilterOutlined, 
  SettingOutlined,
  ClearOutlined,
  SaveOutlined,
  HistoryOutlined,
  QuestionCircleOutlined
} from '@ant-design/icons';
import useSearchStore from '../../store/search';
import FilterPanel from './FilterPanel';
import SearchResultsList from './SearchResultsList';
import SavedSearchManager from './SavedSearchManager';
import { SearchSuggestion } from '../../types/search';

const { Search } = Input;

interface AdvancedSearchInterfaceProps {
  className?: string;
  onAssetSelect?: (assetId: number) => void;
  showResultsInDrawer?: boolean;
}

const AdvancedSearchInterface: React.FC<AdvancedSearchInterfaceProps> = ({
  className,
  onAssetSelect,
  showResultsInDrawer = false,
}) => {
  const {
    // State
    query,
    results,
    loading,
    error,
    suggestions,
    isAdvancedMode,
    totalResults,
    searchTime,
    
    // Actions
    performTextSearch,
    performSearch,
    updateQuery,
    clearSearch,
    toggleAdvancedMode,
    loadSuggestions,
    clearSuggestions,
    findSimilarAssets,
  } = useSearchStore();

  const [searchValue, setSearchValue] = useState(query.text_query || '');
  const [showSavedSearches, setShowSavedSearches] = useState(false);
  const [showResults, setShowResults] = useState(false);
  const [searchHistory, setSearchHistory] = useState<string[]>([]);

  // Load search suggestions
  useEffect(() => {
    if (searchValue && searchValue.length >= 2) {
      const timeoutId = setTimeout(() => {
        loadSuggestions(searchValue);
      }, 300);
      
      return () => clearTimeout(timeoutId);
    } else {
      clearSuggestions();
    }
  }, [searchValue, loadSuggestions, clearSuggestions]);

  // Update search value when query changes
  useEffect(() => {
    setSearchValue(query.text_query || '');
  }, [query.text_query]);

  // Show results when we have them
  useEffect(() => {
    if (results.length > 0) {
      setShowResults(true);
    }
  }, [results]);

  // Save search to history
  const saveToHistory = useCallback((searchTerm: string) => {
    if (searchTerm && !searchHistory.includes(searchTerm)) {
      const newHistory = [searchTerm, ...searchHistory.slice(0, 9)]; // Keep last 10
      setSearchHistory(newHistory);
      localStorage.setItem('ferrocodex-search-history', JSON.stringify(newHistory));
    }
  }, [searchHistory]);

  // Load search history on mount
  useEffect(() => {
    const savedHistory = localStorage.getItem('ferrocodex-search-history');
    if (savedHistory) {
      try {
        setSearchHistory(JSON.parse(savedHistory));
      } catch (error) {
        console.error('Failed to load search history:', error);
      }
    }
  }, []);

  const handleSearch = useCallback(async (value: string) => {
    if (!value.trim()) {
      notification.warning({
        message: 'Search Required',
        description: 'Please enter a search term or apply filters.',
      });
      return;
    }

    try {
      saveToHistory(value);
      await performTextSearch(value);
      setShowResults(true);
    } catch (error) {
      console.error('Search failed:', error);
      notification.error({
        message: 'Search Failed',
        description: error instanceof Error ? error.message : 'An unexpected error occurred',
      });
    }
  }, [performTextSearch, saveToHistory]);

  const handleAdvancedSearch = useCallback(async () => {
    if (!query.text_query && query.filters.length === 0) {
      notification.warning({
        message: 'Search Criteria Required',
        description: 'Please enter a search term or add filters.',
      });
      return;
    }

    try {
      await performSearch(query);
      setShowResults(true);
    } catch (error) {
      console.error('Advanced search failed:', error);
      notification.error({
        message: 'Search Failed',
        description: error instanceof Error ? error.message : 'An unexpected error occurred',
      });
    }
  }, [query, performSearch]);

  const handleSuggestionSelect = useCallback((value: string, option: { suggestion: SearchSuggestion }) => {
    setSearchValue(value);
    clearSuggestions();
    
    // If it's a field name suggestion, switch to advanced mode
    if (option.suggestion.suggestion_type === 'FieldName') {
      if (!isAdvancedMode) {
        toggleAdvancedMode();
      }
    }
  }, [clearSuggestions, isAdvancedMode, toggleAdvancedMode]);

  const handleClearSearch = useCallback(() => {
    setSearchValue('');
    clearSearch();
    setShowResults(false);
  }, [clearSearch]);

  const handleFindSimilar = useCallback(async (assetId: number) => {
    try {
      const similarAssets = await findSimilarAssets(assetId, 0.7);
      notification.success({
        message: 'Similar Assets Found',
        description: `Found ${similarAssets.length} similar assets`,
      });
      setShowResults(true);
    } catch (error) {
      console.error('Find similar failed:', error);
      notification.error({
        message: 'Find Similar Failed',
        description: error instanceof Error ? error.message : 'An unexpected error occurred',
      });
    }
  }, [findSimilarAssets]);

  // Prepare suggestions for AutoComplete
  const suggestionOptions = suggestions.map((suggestion) => ({
    value: suggestion.text,
    label: (
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>{suggestion.text}</span>
        <Tag size="small" color={
          suggestion.suggestion_type === 'FieldName' ? 'blue' :
          suggestion.suggestion_type === 'FieldValue' ? 'green' :
          suggestion.suggestion_type === 'AssetName' ? 'orange' :
          'default'
        }>
          {suggestion.suggestion_type}
        </Tag>
      </div>
    ),
    suggestion,
  }));

  // Add search history to suggestions
  const historyOptions = searchHistory
    .filter(h => h.toLowerCase().includes(searchValue.toLowerCase()))
    .slice(0, 3)
    .map(h => ({
      value: h,
      label: (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>{h}</span>
          <Tag size="small" color="default" icon={<HistoryOutlined />}>
            Recent
          </Tag>
        </div>
      ),
      suggestion: {
        text: h,
        suggestion_type: 'RecentSearch' as const,
        description: 'Recent search',
        usage_count: 0,
      },
    }));

  const allOptions = [...historyOptions, ...suggestionOptions];

  const searchResultsContent = (
    <SearchResultsList
      results={results}
      loading={loading}
      error={error}
      totalResults={totalResults}
      searchTime={searchTime}
      onAssetClick={onAssetSelect}
      onFindSimilarAssets={handleFindSimilar}
      showMetadataDetails={true}
    />
  );

  return (
    <div className={className}>
      <Card
        title={
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>
              <SearchOutlined style={{ marginRight: 8 }} />
              Asset Search
            </span>
            <Space>
              <Tooltip title="View saved searches">
                <Button
                  icon={<SaveOutlined />}
                  onClick={() => setShowSavedSearches(true)}
                >
                  Saved
                </Button>
              </Tooltip>
              <Tooltip title={isAdvancedMode ? 'Switch to simple search' : 'Switch to advanced search'}>
                <Switch
                  checked={isAdvancedMode}
                  onChange={toggleAdvancedMode}
                  checkedChildren={<FilterOutlined />}
                  unCheckedChildren={<SearchOutlined />}
                />
              </Tooltip>
            </Space>
          </div>
        }
        extra={
          <Space>
            <Tooltip title="Clear search">
              <Button
                icon={<ClearOutlined />}
                onClick={handleClearSearch}
                disabled={!searchValue && query.filters.length === 0}
              >
                Clear
              </Button>
            </Tooltip>
            <Tooltip title="Search help">
              <Button icon={<QuestionCircleOutlined />} />
            </Tooltip>
          </Space>
        }
      >
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          {/* Search Input */}
          <Row gutter={[16, 16]}>
            <Col flex="auto">
              <AutoComplete
                options={allOptions}
                onSelect={handleSuggestionSelect}
                onSearch={setSearchValue}
                value={searchValue}
                style={{ width: '100%' }}
              >
                <Search
                  placeholder="Search assets by name, metadata, or properties..."
                  enterButton={
                    <Button type="primary" icon={<SearchOutlined />}>
                      Search
                    </Button>
                  }
                  onSearch={handleSearch}
                  loading={loading}
                  size="large"
                />
              </AutoComplete>
            </Col>
          </Row>

          {/* Advanced Filters */}
          {isAdvancedMode && (
            <>
              <Divider />
              <FilterPanel />
              <Row>
                <Col span={24}>
                  <Button
                    type="primary"
                    icon={<SearchOutlined />}
                    onClick={handleAdvancedSearch}
                    loading={loading}
                    block
                    size="large"
                  >
                    Search with Filters
                  </Button>
                </Col>
              </Row>
            </>
          )}

          {/* Search Results Summary */}
          {(results.length > 0 || loading) && !showResultsInDrawer && (
            <>
              <Divider />
              <div>
                {loading ? (
                  <div style={{ textAlign: 'center', color: '#666' }}>
                    Searching...
                  </div>
                ) : (
                  <div style={{ textAlign: 'center', color: '#666', marginBottom: 16 }}>
                    Found {totalResults} results in {searchTime}ms
                  </div>
                )}
              </div>
            </>
          )}
        </Space>
      </Card>

      {/* Search Results */}
      {!showResultsInDrawer && (results.length > 0 || loading) && (
        <div style={{ marginTop: 16 }}>
          {searchResultsContent}
        </div>
      )}

      {/* Results Drawer */}
      {showResultsInDrawer && (
        <Drawer
          title={`Search Results (${totalResults} found)`}
          open={showResults}
          onClose={() => setShowResults(false)}
          width={800}
          placement="right"
        >
          {searchResultsContent}
        </Drawer>
      )}

      {/* Saved Searches Drawer */}
      <Drawer
        title="Saved Searches"
        open={showSavedSearches}
        onClose={() => setShowSavedSearches(false)}
        width={600}
        placement="left"
      >
        <SavedSearchManager />
      </Drawer>
    </div>
  );
};

export default AdvancedSearchInterface;