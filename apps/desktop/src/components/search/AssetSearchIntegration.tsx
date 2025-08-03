import React, { useState, useEffect } from 'react';
import { 
  Row, 
  Col, 
  Card, 
  Button, 
  Space, 
  Drawer, 
  message,
  Typography,
  Badge,
  Tag,
  Tooltip
} from 'antd';
import { 
  SearchOutlined, 
  FilterOutlined, 
  PlusOutlined,
  EyeOutlined,
  SettingOutlined
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import MetadataSearchBar from './MetadataSearchBar';
import MetadataSearchResults from './MetadataSearchResults';
import MetadataFilterBuilder from './MetadataFilterBuilder';
import { useSearch } from '../../contexts/SearchContext';
import { AssetSearchResult } from '../../types/search';
import { AssetInfo } from '../../types/assets';
import useAuthStore from '../../store/auth';
import useAssetStore from '../../store/assets';

const { Title, Text } = Typography;

interface AssetSearchIntegrationProps {
  showCreateButton?: boolean;
  onAssetCreate?: () => void;
  onAssetSelect?: (asset: AssetSearchResult) => void;
  embedded?: boolean;
  className?: string;
}

const AssetSearchIntegration: React.FC<AssetSearchIntegrationProps> = ({
  showCreateButton = true,
  onAssetCreate,
  onAssetSelect,
  embedded = false,
  className = '',
}) => {
  const navigate = useNavigate();
  const { token } = useAuthStore();
  const { fetchAssets } = useAssetStore();
  
  const {
    searchState,
    filterState,
    performSearch,
    updateQuery,
    findSimilarAssets,
  } = useSearch();

  const [filterDrawerVisible, setFilterDrawerVisible] = useState(false);
  const [searchActive, setSearchActive] = useState(false);

  // Handle search initiation
  const handleSearch = async (query: string) => {
    setSearchActive(!!query.trim());
    
    const searchQuery = {
      ...searchState.query,
      text_query: query.trim() || undefined,
    };

    if (query.trim() || filterState.filters.length > 0) {
      await performSearch(searchQuery);
    }
  };

  // Handle asset selection from search results
  const handleAssetClick = async (assetId: number) => {
    try {
      // Get full asset info from the backend
      const assetDetails = await invoke<AssetInfo>('get_asset_details', {
        token,
        assetId,
      });

      if (onAssetSelect) {
        // Convert AssetInfo to AssetSearchResult for consistency
        const searchResult: AssetSearchResult = {
          asset_id: assetId,
          asset_name: assetDetails.name,
          asset_type: assetDetails.asset_type.toLowerCase(),
          hierarchy_path: [], // Would need to fetch this separately if needed
          metadata_matches: [],
          relevance_score: 1.0,
          last_updated: assetDetails.created_at,
        };
        onAssetSelect(searchResult);
      } else {
        // Navigate to asset details
        navigate(`/assets/${assetId}`);
      }
    } catch (error) {
      message.error('Failed to load asset details');
      console.error('Error loading asset details:', error);
    }
  };

  // Handle finding similar assets
  const handleFindSimilarAssets = async (assetId: number) => {
    try {
      const similarAssets = await findSimilarAssets(assetId, 0.5);
      
      if (similarAssets.length > 0) {
        message.success(`Found ${similarAssets.length} similar assets`);
        // The search context will update with similar assets as new results
      } else {
        message.info('No similar assets found');
      }
    } catch (error) {
      message.error('Failed to find similar assets');
    }
  };

  // Handle filter application
  const handleFiltersChange = () => {
    if (searchState.query.text_query || filterState.filters.length > 0) {
      performSearch(searchState.query);
      setSearchActive(true);
    }
  };

  // Handle create asset button
  const handleCreateAsset = () => {
    if (onAssetCreate) {
      onAssetCreate();
    } else {
      navigate('/assets/create');
    }
  };

  // Render search section
  const renderSearchSection = () => (
    <Card style={{ marginBottom: '16px' }}>
      <Row gutter={[16, 16]} align="middle">
        <Col flex="auto">
          <MetadataSearchBar
            onSearch={handleSearch}
            showAdvancedButton={true}
            onAdvancedClick={() => setFilterDrawerVisible(true)}
            loading={searchState.loading}
            placeholder="Search assets by metadata, names, or properties..."
            showSuggestions={true}
            showFilterCount={true}
            allowClear={true}
            showSearchHistory={true}
          />
        </Col>
        
        {showCreateButton && (
          <Col>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={handleCreateAsset}
              size="large"
            >
              Create Asset
            </Button>
          </Col>
        )}
      </Row>

      {/* Search status */}
      {searchActive && (
        <div style={{ marginTop: '12px' }}>
          <Row justify="space-between" align="middle">
            <Col>
              <Space>
                <Badge 
                  count={searchState.results.length} 
                  style={{ backgroundColor: '#52c41a' }}
                >
                  <Text type="secondary">Search Results</Text>
                </Badge>
                
                {filterState.filters.length > 0 && (
                  <Tag color="blue" icon={<FilterOutlined />}>
                    {filterState.filters.length} filter{filterState.filters.length !== 1 ? 's' : ''}
                  </Tag>
                )}
                
                {searchState.searchTime > 0 && (
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    ({searchState.searchTime.toFixed(0)}ms)
                  </Text>
                )}
              </Space>
            </Col>
            
            <Col>
              <Button
                size="small"
                onClick={() => {
                  updateQuery({ text_query: undefined, filters: [] });
                  setSearchActive(false);
                }}
              >
                Clear Search
              </Button>
            </Col>
          </Row>
        </div>
      )}
    </Card>
  );

  // Render results section
  const renderResultsSection = () => {
    if (!searchActive) {
      return (
        <Card>
          <div style={{ textAlign: 'center', padding: '40px' }}>
            <SearchOutlined style={{ fontSize: '48px', color: '#d9d9d9', marginBottom: '16px' }} />
            <Title level={3} type="secondary">
              Search Your Assets
            </Title>
            <Text type="secondary">
              Use the search bar above to find assets by metadata, names, or properties.
              <br />
              You can also use advanced filters for more precise results.
            </Text>
          </div>
        </Card>
      );
    }

    return (
      <Card>
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
      </Card>
    );
  };

  // Render filter drawer
  const renderFilterDrawer = () => (
    <Drawer
      title="Advanced Search Filters"
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

  return (
    <div className={`asset-search-integration ${className}`}>
      {renderSearchSection()}
      {renderResultsSection()}
      {renderFilterDrawer()}
    </div>
  );
};

export default AssetSearchIntegration;