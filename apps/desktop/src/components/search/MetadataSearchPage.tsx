import React, { useState, useEffect } from 'react';
import { Layout, Row, Col, Card, Drawer, Button, Space, Affix, BackTop } from 'antd';
import { SearchOutlined, FilterOutlined, BarChartOutlined } from '@ant-design/icons';
import { useNavigate, useLocation } from 'react-router-dom';
import MetadataSearchBar from './MetadataSearchBar';
import MetadataSearchResults from './MetadataSearchResults';
import MetadataFilterBuilder from './MetadataFilterBuilder';
import { useSearch } from '../../contexts/SearchContext';
import { SearchContextProvider } from '../../contexts/SearchContext';
import { AssetSearchResult } from '../../types/search';

const { Content } = Layout;

interface MetadataSearchPageProps {
  embedded?: boolean;
  onAssetSelect?: (assetId: number) => void;
  initialQuery?: string;
  showAnalytics?: boolean;
  className?: string;
}

const MetadataSearchPageInner: React.FC<MetadataSearchPageProps> = ({
  embedded = false,
  onAssetSelect,
  initialQuery,
  showAnalytics = false,
  className = '',
}) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { 
    searchState, 
    filterState, 
    performSearch, 
    updateQuery,
    loadPresets 
  } = useSearch();

  const [filterDrawerVisible, setFilterDrawerVisible] = useState(false);
  const [searchInitialized, setSearchInitialized] = useState(false);

  // Initialize search from URL parameters or initial query
  useEffect(() => {
    if (!searchInitialized) {
      const urlParams = new URLSearchParams(location.search);
      const queryFromUrl = urlParams.get('q');
      const hierarchyScope = urlParams.get('scope');
      
      const initialSearchQuery = queryFromUrl || initialQuery;
      
      if (initialSearchQuery) {
        updateQuery({ 
          text_query: initialSearchQuery,
          hierarchy_scope: hierarchyScope ? parseInt(hierarchyScope) : undefined
        });
        
        performSearch({
          ...searchState.query,
          text_query: initialSearchQuery,
          hierarchy_scope: hierarchyScope ? parseInt(hierarchyScope) : undefined
        });
      }
      
      setSearchInitialized(true);
    }
  }, [location.search, initialQuery, searchInitialized, updateQuery, performSearch, searchState.query]);

  // Load filter presets on mount
  useEffect(() => {
    loadPresets();
  }, [loadPresets]);

  // Handle search
  const handleSearch = async (query: string) => {
    const searchQuery = {
      ...searchState.query,
      text_query: query || undefined,
      offset: 0, // Reset pagination
    };

    await performSearch(searchQuery);

    // Update URL if not embedded
    if (!embedded && query) {
      const params = new URLSearchParams();
      params.set('q', query);
      if (searchQuery.hierarchy_scope) {
        params.set('scope', searchQuery.hierarchy_scope.toString());
      }
      navigate(`${location.pathname}?${params.toString()}`, { replace: true });
    }
  };

  // Handle asset selection
  const handleAssetClick = (assetId: number) => {
    if (onAssetSelect) {
      onAssetSelect(assetId);
    } else if (!embedded) {
      // Navigate to asset details page
      navigate(`/assets/${assetId}`);
    }
  };

  // Handle find similar assets
  const handleFindSimilarAssets = (assetId: number) => {
    // This could open a modal with similar assets or update the search
    console.log('Find similar assets for:', assetId);
  };

  // Toggle filter drawer
  const toggleFilterDrawer = () => {
    setFilterDrawerVisible(!filterDrawerVisible);
  };

  // Handle filter changes
  const handleFiltersChange = () => {
    // Automatically search when filters change
    if (searchState.query.text_query || filterState.filters.length > 0) {
      performSearch(searchState.query);
    }
  };

  // Render search controls
  const renderSearchControls = () => (
    <Card style={{ marginBottom: '16px' }}>
      <MetadataSearchBar
        onSearch={handleSearch}
        showAdvancedButton={true}
        onAdvancedClick={toggleFilterDrawer}
        loading={searchState.loading}
        placeholder="Search assets by metadata fields, values, or asset names..."
        showSuggestions={true}
        showFilterCount={true}
        allowClear={true}
        showSearchHistory={true}
      />
    </Card>
  );

  // Render search results
  const renderSearchResults = () => (
    <MetadataSearchResults
      results={searchState.results}
      loading={searchState.loading}
      error={searchState.error}
      totalResults={searchState.totalResults}
      searchTime={searchState.searchTime}
      onAssetClick={handleAssetClick}
      onFindSimilarAssets={handleFindSimilarAssets}
      showMetadataDetails={true}
    />
  );

  // Render filter drawer
  const renderFilterDrawer = () => (
    <Drawer
      title="Advanced Filters"
      placement="right"
      width={500}
      onClose={() => setFilterDrawerVisible(false)}
      open={filterDrawerVisible}
      bodyStyle={{ padding: 0 }}
      extra={
        <Space>
          <Button
            type="primary"
            icon={<SearchOutlined />}
            onClick={() => {
              handleFiltersChange();
              setFilterDrawerVisible(false);
            }}
            disabled={searchState.loading}
          >
            Apply Filters
          </Button>
        </Space>
      }
    >
      <div style={{ padding: '16px' }}>
        <MetadataFilterBuilder
          filters={filterState.filters}
          availableFields={filterState.availableFields}
          onFiltersChange={handleFiltersChange}
          presets={filterState.presets}
        />
      </div>
    </Drawer>
  );

  // Render analytics button (if enabled)
  const renderAnalyticsButton = () => {
    if (!showAnalytics || embedded) return null;

    return (
      <Affix offsetTop={80} style={{ position: 'fixed', right: '24px', zIndex: 1000 }}>
        <Button
          type="primary"
          icon={<BarChartOutlined />}
          onClick={() => navigate('/search/analytics')}
          style={{ marginBottom: '8px' }}
        >
          Analytics
        </Button>
      </Affix>
    );
  };

  const containerStyle = embedded 
    ? { height: '100%', overflow: 'auto' }
    : { minHeight: 'calc(100vh - 64px)', padding: '24px' };

  return (
    <div className={`metadata-search-page ${className}`} style={containerStyle}>
      <Layout style={{ background: 'transparent' }}>
        <Content>
          <Row gutter={[16, 16]}>
            <Col span={24}>
              {renderSearchControls()}
            </Col>
            
            <Col span={24}>
              {renderSearchResults()}
            </Col>
          </Row>
        </Content>
      </Layout>

      {renderFilterDrawer()}
      {renderAnalyticsButton()}
      
      <BackTop />
    </div>
  );
};

// Main component with search context provider
const MetadataSearchPage: React.FC<MetadataSearchPageProps> = (props) => {
  return (
    <SearchContextProvider>
      <MetadataSearchPageInner {...props} />
    </SearchContextProvider>
  );
};

export default MetadataSearchPage;