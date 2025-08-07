import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Input, AutoComplete, Button, Space, Tooltip, Spin } from 'antd';
import { SearchOutlined, FilterOutlined, ClearOutlined, HistoryOutlined } from '@ant-design/icons';
import { SearchBarProps, SearchSuggestion, SuggestionType } from '../../types/search';
import { useSearch } from '../../contexts/SearchContext';
import { debounce } from 'lodash';

const { Search } = Input;

interface MetadataSearchBarProps extends SearchBarProps {
  showSuggestions?: boolean;
  showFilterCount?: boolean;
  allowClear?: boolean;
  showSearchHistory?: boolean;
}

const MetadataSearchBar: React.FC<MetadataSearchBarProps> = ({
  placeholder = "Search assets by metadata...",
  onSearch,
  onSuggestionSelect,
  loading = false,
  showAdvancedButton = true,
  onAdvancedClick,
  className = '',
  showSuggestions = true,
  showFilterCount = true,
  allowClear = true,
  showSearchHistory = false,
}) => {
  const { 
    searchState, 
    filterState, 
    loadSuggestions, 
    performSearch, 
    updateQuery 
  } = useSearch();

  const [searchValue, setSearchValue] = useState(searchState.query.text_query || '');
  const [autoCompleteOptions, setAutoCompleteOptions] = useState<{ value: string; label: React.ReactNode }[]>([]);
  const [suggestionsLoading, setSuggestionsLoading] = useState(false);
  const [searchHistory, setSearchHistory] = useState<string[]>([]);
  
  const searchInputRef = useRef<any>(null);

  // Debounced suggestion loading
  const debouncedLoadSuggestions = useCallback(
    debounce(async (query: string) => {
      if (!query.trim() || !showSuggestions) {
        setAutoCompleteOptions([]);
        return;
      }

      setSuggestionsLoading(true);
      try {
        await loadSuggestions(query);
      } finally {
        setSuggestionsLoading(false);
      }
    }, 300),
    [loadSuggestions, showSuggestions]
  );

  // Update auto-complete options when suggestions change
  useEffect(() => {
    const options = searchState.suggestions.map((suggestion: SearchSuggestion) => {
      let icon;
      let color;
      
      switch (suggestion.suggestion_type) {
        case 'FieldName':
          icon = '🏷️';
          color = '#1890ff';
          break;
        case 'FieldValue':
          icon = '💾';
          color = '#52c41a';
          break;
        case 'AssetName':
          icon = '🔧';
          color = '#fa8c16';
          break;
        case 'SchemaName':
          icon = '📋';
          color = '#722ed1';
          break;
        case 'RecentSearch':
          icon = '🕒';
          color = '#666';
          break;
        default:
          icon = '🔍';
          color = '#666';
      }

      return {
        value: suggestion.text,
        label: (
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span style={{ fontSize: '14px' }}>{icon}</span>
              <div>
                <div style={{ color, fontWeight: 500 }}>{suggestion.text}</div>
                <div style={{ fontSize: '12px', color: '#999' }}>
                  {suggestion.description}
                  {suggestion.field_name && ` in ${suggestion.field_name}`}
                </div>
              </div>
            </div>
            <div style={{ fontSize: '11px', color: '#ccc' }}>
              {suggestion.usage_count > 0 && `${suggestion.usage_count} uses`}
            </div>
          </div>
        ),
      };
    });

    // Add search history if enabled
    if (showSearchHistory && searchHistory.length > 0 && searchValue.length > 0) {
      const historyOptions = searchHistory
        .filter(item => item.toLowerCase().includes(searchValue.toLowerCase()))
        .slice(0, 3)
        .map(item => ({
          value: item,
          label: (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <HistoryOutlined style={{ color: '#999' }} />
              <span style={{ color: '#666' }}>{item}</span>
            </div>
          ),
        }));
      
      if (historyOptions.length > 0) {
        options.unshift(...historyOptions);
      }
    }

    setAutoCompleteOptions(options);
  }, [searchState.suggestions, showSearchHistory, searchHistory, searchValue]);

  // Handle search input change
  const handleSearchChange = (value: string) => {
    setSearchValue(value);
    updateQuery({ text_query: value });
    
    if (showSuggestions) {
      debouncedLoadSuggestions(value);
    }
  };

  // Handle search execution
  const handleSearch = useCallback(async (value: string) => {
    const trimmedValue = value.trim();
    
    // Add to search history
    if (trimmedValue && !searchHistory.includes(trimmedValue)) {
      const newHistory = [trimmedValue, ...searchHistory.slice(0, 9)]; // Keep last 10 searches
      setSearchHistory(newHistory);
      localStorage.setItem('metadata_search_history', JSON.stringify(newHistory));
    }

    // Update query and perform search
    const searchQuery = {
      ...searchState.query,
      text_query: trimmedValue || undefined,
    };

    await performSearch(searchQuery);
    onSearch(trimmedValue);
  }, [searchState.query, performSearch, onSearch, searchHistory]);

  // Handle suggestion selection
  const handleSuggestionSelect = (value: string, option: any) => {
    setSearchValue(value);
    
    // Find the original suggestion object
    const suggestion = searchState.suggestions.find(s => s.text === value);
    if (suggestion && onSuggestionSelect) {
      onSuggestionSelect(suggestion);
    }
    
    // Perform search immediately
    handleSearch(value);
  };

  // Handle clear
  const handleClear = () => {
    setSearchValue('');
    updateQuery({ text_query: undefined });
    setAutoCompleteOptions([]);
  };

  // Load search history on mount
  useEffect(() => {
    if (showSearchHistory) {
      const stored = localStorage.getItem('metadata_search_history');
      if (stored) {
        try {
          setSearchHistory(JSON.parse(stored));
        } catch (error) {
          console.error('Failed to load search history:', error);
        }
      }
    }
  }, [showSearchHistory]);

  // Focus on search input when component mounts
  useEffect(() => {
    if (searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, []);

  const filterCount = filterState.filters.length;
  const hasActiveFilters = filterCount > 0;

  return (
    <div className={`metadata-search-bar ${className}`}>
      <Space.Compact style={{ width: '100%' }}>
        <AutoComplete
          style={{ flex: 1 }}
          options={autoCompleteOptions}
          onSelect={handleSuggestionSelect}
          onSearch={handleSearchChange}
          value={searchValue}
          notFoundContent={suggestionsLoading ? <Spin size="small" /> : null}
        >
          <Search
            ref={searchInputRef}
            placeholder={placeholder}
            enterButton={
              <Button 
                type="primary" 
                icon={<SearchOutlined />}
                loading={loading}
              >
                Search
              </Button>
            }
            size="large"
            onSearch={handleSearch}
            allowClear={allowClear}
            onClear={handleClear}
          />
        </AutoComplete>

        {showAdvancedButton && (
          <Tooltip title={hasActiveFilters ? `${filterCount} filter${filterCount !== 1 ? 's' : ''} active` : 'Advanced filters'}>
            <Button
              size="large"
              icon={<FilterOutlined />}
              onClick={onAdvancedClick}
              type={hasActiveFilters ? 'primary' : 'default'}
            >
              {showFilterCount && hasActiveFilters && (
                <span style={{ marginLeft: 4 }}>{filterCount}</span>
              )}
            </Button>
          </Tooltip>
        )}
      </Space.Compact>

      {/* Search status/info */}
      {searchState.results.length > 0 && (
        <div style={{ marginTop: '8px', fontSize: '12px', color: '#666' }}>
          Found {searchState.totalResults} result{searchState.totalResults !== 1 ? 's' : ''} 
          {searchState.searchTime > 0 && ` in ${searchState.searchTime.toFixed(0)}ms`}
          {hasActiveFilters && ` with ${filterCount} filter${filterCount !== 1 ? 's' : ''}`}
        </div>
      )}

      {searchState.error && (
        <div style={{ marginTop: '8px', fontSize: '12px', color: '#ff4d4f' }}>
          Search error: {searchState.error}
        </div>
      )}
    </div>
  );
};

export default MetadataSearchBar;