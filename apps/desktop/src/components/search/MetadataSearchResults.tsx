import React, { useState } from 'react';
import { 
  List, 
  Card, 
  Tag, 
  Button, 
  Space, 
  Typography, 
  Tooltip, 
  Badge, 
  Collapse, 
  Empty,
  Spin,
  Alert,
  Breadcrumb
} from 'antd';
import { 
  FolderOutlined, 
  HddOutlined, 
  SearchOutlined, 
  EyeOutlined,
  SettingOutlined,
  CopyOutlined,
  StarOutlined,
  ClockCircleOutlined
} from '@ant-design/icons';
import { AssetSearchResult, SearchResultsProps, MetadataMatch, MatchType } from '../../types/search';
import { useSearch } from '../../contexts/SearchContext';

const { Text, Title } = Typography;
const { Panel } = Collapse;

const MetadataSearchResults: React.FC<SearchResultsProps> = ({
  results,
  loading = false,
  error,
  totalResults,
  searchTime,
  onAssetClick,
  onFindSimilarAssets,
  showMetadataDetails = true,
  className = '',
}) => {
  const { findSimilarAssets } = useSearch();
  const [expandedAssets, setExpandedAssets] = useState<Set<number>>(new Set());
  const [loadingSimilar, setLoadingSimilar] = useState<Set<number>>(new Set());

  // Handle asset click
  const handleAssetClick = (assetId: number) => {
    if (onAssetClick) {
      onAssetClick(assetId);
    }
  };

  // Handle find similar assets
  const handleFindSimilar = async (assetId: number) => {
    setLoadingSimilar(prev => new Set(prev).add(assetId));
    
    try {
      const similarAssets = await findSimilarAssets(assetId, 0.3);
      
      if (onFindSimilarAssets) {
        onFindSimilarAssets(assetId);
      }
      
      // Could show similar assets in a modal or navigate to them
      console.log('Similar assets found:', similarAssets);
    } finally {
      setLoadingSimilar(prev => {
        const newSet = new Set(prev);
        newSet.delete(assetId);
        return newSet;
      });
    }
  };

  // Toggle expanded metadata details
  const toggleExpanded = (assetId: number) => {
    setExpandedAssets(prev => {
      const newSet = new Set(prev);
      if (newSet.has(assetId)) {
        newSet.delete(assetId);
      } else {
        newSet.add(assetId);
      }
      return newSet;
    });
  };

  // Get asset type icon
  const getAssetTypeIcon = (assetType: string) => {
    switch (assetType.toLowerCase()) {
      case 'folder':
        return <FolderOutlined style={{ color: '#faad14' }} />;
      case 'device':
        return <HddOutlined style={{ color: '#1890ff' }} />;
      default:
        return <SettingOutlined style={{ color: '#52c41a' }} />;
    }
  };

  // Get match type color and icon
  const getMatchTypeDisplay = (matchType: MatchType) => {
    switch (matchType) {
      case 'ExactMatch':
        return { color: '#52c41a', icon: '🎯', label: 'Exact Match' };
      case 'PartialMatch':
        return { color: '#1890ff', icon: '🔍', label: 'Partial Match' };
      case 'FuzzyMatch':
        return { color: '#faad14', icon: '🌟', label: 'Fuzzy Match' };
      case 'FieldName':
        return { color: '#722ed1', icon: '🏷️', label: 'Field Name' };
      default:
        return { color: '#666', icon: '❓', label: 'Unknown' };
    }
  };

  // Render hierarchy breadcrumb
  const renderHierarchyPath = (path: string[]) => {
    if (path.length === 0) return null;

    return (
      <Breadcrumb style={{ fontSize: '12px' }}>
        {path.map((segment, index) => (
          <Breadcrumb.Item key={index}>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              {segment}
            </Text>
          </Breadcrumb.Item>
        ))}
      </Breadcrumb>
    );
  };

  // Render metadata matches
  const renderMetadataMatches = (matches: MetadataMatch[]) => {
    if (matches.length === 0) return null;

    return (
      <div style={{ marginTop: '12px' }}>
        <Title level={5} style={{ marginBottom: '8px' }}>
          Metadata Matches ({matches.length})
        </Title>
        <Space wrap>
          {matches.map((match, index) => {
            const matchDisplay = getMatchTypeDisplay(match.match_type);
            return (
              <Tooltip
                key={index}
                title={
                  <div>
                    <div><strong>Field:</strong> {match.field_name}</div>
                    <div><strong>Schema:</strong> {match.schema_name}</div>
                    <div><strong>Match Type:</strong> {matchDisplay.label}</div>
                    <div><strong>Value:</strong> {match.field_value}</div>
                  </div>
                }
              >
                <Tag color={matchDisplay.color} style={{ cursor: 'help' }}>
                  <span style={{ marginRight: '4px' }}>{matchDisplay.icon}</span>
                  {match.field_name}
                  <Text 
                    style={{ 
                      marginLeft: '6px', 
                      fontWeight: 'normal',
                      fontSize: '11px' 
                    }}
                  >
                    {match.field_value.length > 20 
                      ? `${match.field_value.substring(0, 20)}...` 
                      : match.field_value
                    }
                  </Text>
                </Tag>
              </Tooltip>
            );
          })}
        </Space>
      </div>
    );
  };

  // Render individual search result
  const renderSearchResult = (result: AssetSearchResult) => {
    const isExpanded = expandedAssets.has(result.asset_id);
    const isLoadingSimilar = loadingSimilar.has(result.asset_id);

    return (
      <Card
        key={result.asset_id}
        size="small"
        style={{ marginBottom: '12px' }}
        actions={[
          <Button
            key="view"
            type="text"
            icon={<EyeOutlined />}
            onClick={() => handleAssetClick(result.asset_id)}
          >
            View Details
          </Button>,
          <Button
            key="similar"
            type="text"
            icon={<SearchOutlined />}
            loading={isLoadingSimilar}
            onClick={() => handleFindSimilar(result.asset_id)}
          >
            Find Similar
          </Button>,
          showMetadataDetails && (
            <Button
              key="metadata"
              type="text"
              icon={isExpanded ? 'Collapse' : 'Expand'}
              onClick={() => toggleExpanded(result.asset_id)}
            >
              {isExpanded ? 'Less' : 'More'}
            </Button>
          ),
        ].filter(Boolean)}
      >
        <Card.Meta
          avatar={
            <Badge count={result.metadata_matches.length} size="small">
              {getAssetTypeIcon(result.asset_type)}
            </Badge>
          }
          title={
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Text strong style={{ fontSize: '16px' }}>
                {result.asset_name}
              </Text>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Tooltip title={`Relevance Score: ${result.relevance_score.toFixed(2)}`}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <StarOutlined style={{ color: '#faad14', fontSize: '12px' }} />
                    <Text style={{ fontSize: '12px', color: '#666' }}>
                      {result.relevance_score.toFixed(1)}
                    </Text>
                  </div>
                </Tooltip>
                <Tooltip title={`Last Updated: ${new Date(result.last_updated).toLocaleString()}`}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <ClockCircleOutlined style={{ color: '#666', fontSize: '12px' }} />
                    <Text style={{ fontSize: '11px', color: '#999' }}>
                      {new Date(result.last_updated).toLocaleDateString()}
                    </Text>
                  </div>
                </Tooltip>
              </div>
            </div>
          }
          description={
            <div>
              <div style={{ marginBottom: '8px' }}>
                <Tag color="blue" style={{ marginRight: '8px' }}>
                  {result.asset_type}
                </Tag>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  ID: {result.asset_id}
                </Text>
              </div>
              
              {result.hierarchy_path.length > 0 && (
                <div style={{ marginBottom: '8px' }}>
                  {renderHierarchyPath(result.hierarchy_path)}
                </div>
              )}

              {result.metadata_matches.length > 0 && (
                <div style={{ marginBottom: '8px' }}>
                  <Text style={{ fontSize: '12px', color: '#666' }}>
                    {result.metadata_matches.length} metadata match{result.metadata_matches.length !== 1 ? 'es' : ''}
                  </Text>
                </div>
              )}

              {showMetadataDetails && isExpanded && renderMetadataMatches(result.metadata_matches)}
            </div>
          }
        />
      </Card>
    );
  };

  // Show loading state
  if (loading) {
    return (
      <div className={`metadata-search-results ${className}`} style={{ textAlign: 'center', padding: '40px' }}>
        <Spin size="large" />
        <div style={{ marginTop: '16px' }}>
          <Text type="secondary">Searching assets...</Text>
        </div>
      </div>
    );
  }

  // Show error state
  if (error) {
    return (
      <div className={`metadata-search-results ${className}`}>
        <Alert
          message="Search Error"
          description={error}
          type="error"
          showIcon
          style={{ marginBottom: '16px' }}
        />
      </div>
    );
  }

  // Show empty state
  if (!results || results.length === 0) {
    return (
      <div className={`metadata-search-results ${className}`}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <div>
              <Text type="secondary">No assets found matching your search criteria</Text>
              <div style={{ marginTop: '8px', fontSize: '12px', color: '#999' }}>
                Try adjusting your search terms or filters
              </div>
            </div>
          }
        />
      </div>
    );
  }

  return (
    <div className={`metadata-search-results ${className}`}>
      {/* Results summary */}
      <div style={{ marginBottom: '16px', padding: '12px', backgroundColor: '#fafafa', borderRadius: '6px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <Text strong>
              {totalResults || results.length} result{(totalResults || results.length) !== 1 ? 's' : ''}
            </Text>
            {searchTime !== undefined && searchTime > 0 && (
              <Text type="secondary" style={{ marginLeft: '8px', fontSize: '12px' }}>
                ({searchTime.toFixed(0)}ms)
              </Text>
            )}
          </div>
          
          <Space>
            <Button
              size="small"
              icon={<CopyOutlined />}
              onClick={() => {
                const resultSummary = results.map(r => 
                  `${r.asset_name} (${r.asset_type}) - ${r.metadata_matches.length} matches`
                ).join('\n');
                navigator.clipboard.writeText(resultSummary);
              }}
            >
              Copy Results
            </Button>
          </Space>
        </div>
      </div>

      {/* Results list */}
      <div>
        {results.map(renderSearchResult)}
      </div>

      {/* Load more button (if pagination is implemented) */}
      {results.length >= 50 && (
        <div style={{ textAlign: 'center', marginTop: '20px' }}>
          <Button type="default" size="large">
            Load More Results
          </Button>
        </div>
      )}
    </div>
  );
};

export default MetadataSearchResults;