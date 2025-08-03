import React, { useState, useMemo } from 'react';
import {
  Card,
  List,
  Tag,
  Space,
  Button,
  Typography,
  Tooltip,
  Empty,
  Spin,
  Badge,
  Breadcrumb,
  Collapse,
  Row,
  Col,
  Select,
  Pagination,
  Alert,
  Descriptions,
} from 'antd';
import {
  StarOutlined,
  EyeOutlined,
  CopyOutlined,
  ShareAltOutlined,
  MoreOutlined,
  ClockCircleOutlined,
  FolderOutlined,
  TagOutlined,
  SearchOutlined,
  SortAscendingOutlined,
  FilterOutlined,
} from '@ant-design/icons';
import dayjs from 'dayjs';
import { AssetSearchResult, SearchResultsProps, SortField, MatchType } from '../../types/search';
import useSearchStore from '../../store/search';

const { Text, Title, Paragraph } = Typography;
const { Panel } = Collapse;
const { Option } = Select;

const SearchResultsList: React.FC<SearchResultsProps> = ({
  results,
  loading = false,
  error,
  totalResults = 0,
  searchTime = 0,
  onAssetClick,
  onFindSimilarAssets,
  showMetadataDetails = false,
  className,
}) => {
  const { setSortBy, setPageSize, setCurrentPage, query } = useSearchStore();
  const [currentPage, setCurrentPageLocal] = useState(1);
  const [pageSize, setPageSizeLocal] = useState(20);
  const [expandedResults, setExpandedResults] = useState<Set<number>>(new Set());

  const sortOptions: { value: SortField; label: string }[] = [
    { value: 'Relevance', label: 'Relevance' },
    { value: 'AssetName', label: 'Name' },
    { value: 'LastUpdated', label: 'Last Updated' },
    { value: 'CreatedDate', label: 'Created Date' },
    { value: 'AssetType', label: 'Asset Type' },
  ];

  const getMatchTypeColor = (matchType: MatchType): string => {
    switch (matchType) {
      case 'ExactMatch':
        return 'green';
      case 'PartialMatch':
        return 'blue';
      case 'FuzzyMatch':
        return 'orange';
      case 'FieldName':
        return 'purple';
      default:
        return 'default';
    }
  };

  const getMatchTypeIcon = (matchType: MatchType) => {
    switch (matchType) {
      case 'ExactMatch':
        return '🎯';
      case 'PartialMatch':
        return '🔍';
      case 'FuzzyMatch':
        return '🌟';
      case 'FieldName':
        return '🏷️';
      default:
        return '📄';
    }
  };

  const highlightText = (text: string, highlights: string[]): React.ReactNode => {
    if (!highlights.length) return text;
    
    let result = text;
    highlights.forEach(highlight => {
      const regex = new RegExp(`(${highlight.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
      result = result.replace(regex, '<mark>$1</mark>');
    });
    
    return <span dangerouslySetInnerHTML={{ __html: result }} />;
  };

  const toggleExpanded = (assetId: number) => {
    const newExpanded = new Set(expandedResults);
    if (newExpanded.has(assetId)) {
      newExpanded.delete(assetId);
    } else {
      newExpanded.add(assetId);
    }
    setExpandedResults(newExpanded);
  };

  const handlePageChange = (page: number, size?: number) => {
    setCurrentPageLocal(page);
    setCurrentPage(page);
    if (size && size !== pageSize) {
      setPageSizeLocal(size);
      setPageSize(size);
    }
  };

  const handleSortChange = (sortBy: SortField) => {
    setSortBy(sortBy);
  };

  const paginatedResults = useMemo(() => {
    const start = (currentPage - 1) * pageSize;
    return results.slice(start, start + pageSize);
  }, [results, currentPage, pageSize]);

  if (loading) {
    return (
      <div className={className}>
        <Card>
          <div style={{ textAlign: 'center', padding: '2rem' }}>
            <Spin size="large" />
            <div style={{ marginTop: 16 }}>
              <Text>Searching assets...</Text>
            </div>
          </div>
        </Card>
      </div>
    );
  }

  if (error) {
    return (
      <div className={className}>
        <Alert
          message="Search Error"
          description={error}
          type="error"
          showIcon
          action={
            <Button size="small" onClick={() => window.location.reload()}>
              Retry
            </Button>
          }
        />
      </div>
    );
  }

  if (!results.length) {
    return (
      <div className={className}>
        <Card>
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <Space direction="vertical">
                <Text>No assets found</Text>
                <Text type="secondary">
                  Try adjusting your search terms or filters
                </Text>
              </Space>
            }
          />
        </Card>
      </div>
    );
  }

  return (
    <div className={className}>
      <Card
        title={
          <Space>
            <SearchOutlined />
            <span>Search Results</span>
            <Badge count={totalResults} color="blue" />
          </Space>
        }
        extra={
          <Space>
            <Text type="secondary">
              Found in {searchTime}ms
            </Text>
            <Select
              value={query.sort_by || 'Relevance'}
              onChange={handleSortChange}
              style={{ minWidth: 120 }}
              size="small"
            >
              {sortOptions.map(option => (
                <Option key={option.value} value={option.value}>
                  {option.label}
                </Option>
              ))}
            </Select>
          </Space>
        }
      >
        <List
          itemLayout="vertical"
          dataSource={paginatedResults}
          renderItem={(result: AssetSearchResult) => {
            const isExpanded = expandedResults.has(result.asset_id);
            const relevancePercentage = Math.round(result.relevance_score * 10);
            
            return (
              <List.Item
                key={result.asset_id}
                actions={[
                  <Tooltip title="View asset details">
                    <Button
                      icon={<EyeOutlined />}
                      size="small"
                      onClick={() => onAssetClick?.(result.asset_id)}
                    >
                      View
                    </Button>
                  </Tooltip>,
                  <Tooltip title="Find similar assets">
                    <Button
                      icon={<StarOutlined />}
                      size="small"
                      onClick={() => onFindSimilarAssets?.(result.asset_id)}
                    >
                      Similar
                    </Button>
                  </Tooltip>,
                  <Tooltip title="Copy asset information">
                    <Button
                      icon={<CopyOutlined />}
                      size="small"
                      onClick={() => {
                        const info = `${result.asset_name} (ID: ${result.asset_id})`;
                        navigator.clipboard.writeText(info);
                      }}
                    />
                  </Tooltip>,
                  <Button
                    icon={<MoreOutlined />}
                    size="small"
                    onClick={() => toggleExpanded(result.asset_id)}
                  >
                    {isExpanded ? 'Less' : 'More'}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      <Button
                        type="link"
                        style={{ padding: 0, height: 'auto', fontSize: '16px' }}
                        onClick={() => onAssetClick?.(result.asset_id)}
                      >
                        {result.asset_name}
                      </Button>
                      <Tag color="blue">{result.asset_type}</Tag>
                      <Badge
                        count={`${relevancePercentage}%`}
                        color={relevancePercentage > 80 ? 'green' : relevancePercentage > 50 ? 'orange' : 'red'}
                      />
                    </Space>
                  }
                  description={
                    <Space direction="vertical" style={{ width: '100%' }}>
                      {/* Hierarchy Path */}
                      {result.hierarchy_path.length > 0 && (
                        <div>
                          <FolderOutlined style={{ marginRight: 8, color: '#666' }} />
                          <Breadcrumb separator=">">
                            {result.hierarchy_path.map((path, index) => (
                              <Breadcrumb.Item key={index}>
                                <Text type="secondary">{path}</Text>
                              </Breadcrumb.Item>
                            ))}
                          </Breadcrumb>
                        </div>
                      )}

                      {/* Metadata Matches */}
                      {result.metadata_matches.length > 0 && (
                        <div>
                          <Space wrap>
                            {result.metadata_matches.slice(0, isExpanded ? undefined : 3).map((match, index) => (
                              <Tooltip
                                key={index}
                                title={
                                  <div>
                                    <div><strong>Schema:</strong> {match.schema_name}</div>
                                    <div><strong>Field:</strong> {match.field_name}</div>
                                    <div><strong>Match Type:</strong> {match.match_type}</div>
                                    <div><strong>Value:</strong> {match.field_value}</div>
                                  </div>
                                }
                              >
                                <Tag
                                  color={getMatchTypeColor(match.match_type)}
                                  style={{ marginBottom: 4 }}
                                >
                                  {getMatchTypeIcon(match.match_type)} {match.field_name}:{' '}
                                  <span
                                    dangerouslySetInnerHTML={{
                                      __html: match.highlighted_value || match.field_value
                                    }}
                                  />
                                </Tag>
                              </Tooltip>
                            ))}
                            {!isExpanded && result.metadata_matches.length > 3 && (
                              <Tag>
                                +{result.metadata_matches.length - 3} more
                              </Tag>
                            )}
                          </Space>
                        </div>
                      )}

                      {/* Last Updated */}
                      <div>
                        <ClockCircleOutlined style={{ marginRight: 8, color: '#666' }} />
                        <Text type="secondary">
                          Last updated {dayjs(result.last_updated).format('MMM D, YYYY [at] h:mm A')}
                        </Text>
                      </div>
                    </Space>
                  }
                />

                {/* Expanded Details */}
                {isExpanded && showMetadataDetails && (
                  <div style={{ marginTop: 16 }}>
                    <Collapse size="small">
                      <Panel header="Metadata Details" key="metadata">
                        <Descriptions size="small" column={1}>
                          <Descriptions.Item label="Asset ID">
                            {result.asset_id}
                          </Descriptions.Item>
                          <Descriptions.Item label="Asset Type">
                            {result.asset_type}
                          </Descriptions.Item>
                          <Descriptions.Item label="Relevance Score">
                            {result.relevance_score.toFixed(2)} ({relevancePercentage}%)
                          </Descriptions.Item>
                          <Descriptions.Item label="Full Path">
                            {result.hierarchy_path.join(' > ') || 'Root'}
                          </Descriptions.Item>
                        </Descriptions>

                        {result.metadata_matches.length > 0 && (
                          <div style={{ marginTop: 16 }}>
                            <Title level={5}>Matched Fields</Title>
                            <Space direction="vertical" style={{ width: '100%' }}>
                              {result.metadata_matches.map((match, index) => (
                                <Card size="small" key={index}>
                                  <Row gutter={16}>
                                    <Col span={6}>
                                      <Text strong>{match.field_name}</Text>
                                    </Col>
                                    <Col span={6}>
                                      <Tag color={getMatchTypeColor(match.match_type)}>
                                        {match.match_type}
                                      </Tag>
                                    </Col>
                                    <Col span={12}>
                                      <Text
                                        ellipsis={{ tooltip: match.field_value }}
                                        style={{ fontFamily: 'monospace' }}
                                      >
                                        <span
                                          dangerouslySetInnerHTML={{
                                            __html: match.highlighted_value || match.field_value
                                          }}
                                        />
                                      </Text>
                                    </Col>
                                  </Row>
                                  <div style={{ marginTop: 8, fontSize: '12px', color: '#666' }}>
                                    Schema: {match.schema_name}
                                  </div>
                                </Card>
                              ))}
                            </Space>
                          </div>
                        )}
                      </Panel>
                    </Collapse>
                  </div>
                )}
              </List.Item>
            );
          }}
        />

        {/* Pagination */}
        {results.length > pageSize && (
          <div style={{ marginTop: 16, textAlign: 'center' }}>
            <Pagination
              current={currentPage}
              pageSize={pageSize}
              total={results.length}
              onChange={handlePageChange}
              onShowSizeChange={handlePageChange}
              showSizeChanger
              showQuickJumper
              showTotal={(total, range) =>
                `${range[0]}-${range[1]} of ${total} results`
              }
              pageSizeOptions={['10', '20', '50', '100']}
            />
          </div>
        )}
      </Card>
    </div>
  );
};

export default SearchResultsList;