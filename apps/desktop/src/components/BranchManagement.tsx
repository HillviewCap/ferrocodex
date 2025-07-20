import React, { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Typography,
  Card,
  Space,
  Spin,
  Empty,
  message,
  Input,
  Button,
  Row,
  Col,
  Divider,
  Tag,
  Alert,
  Tooltip,
  Tree
} from 'antd';
import {
  BranchesOutlined,
  SearchOutlined,
  PlusOutlined,
  ReloadOutlined,
  InfoCircleOutlined,
  FilterOutlined,
  NodeIndexOutlined,
  UnorderedListOutlined,
  FileOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo, ConfigurationVersionInfo } from '../types/assets';
import { BranchInfo, sortBranchesByCreated } from '../types/branches';
import BranchCard from './BranchCard';
import ImportVersionToBranchModal from './ImportVersionToBranchModal';
import BranchVersionHistory from './BranchVersionHistory';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import useBranchStore from '../store/branches';

const { Title, Text } = Typography;
const { Search } = Input;

interface BranchManagementProps {
  asset: AssetInfo;
  onCreateBranch?: () => void;
  onSelectBranch?: (branch: BranchInfo) => void;
  onViewBranchDetails?: (branch: BranchInfo) => void;
  showCreateButton?: boolean;
  showSelectActions?: boolean;
}

const BranchManagement: React.FC<BranchManagementProps> = ({
  asset,
  onCreateBranch,
  onSelectBranch,
  onViewBranchDetails,
  showCreateButton = true,
  showSelectActions = true
}) => {
  const { token } = useAuthStore();
  const { versions } = useAssetStore();
  const { branchVersions, fetchBranchVersions } = useBranchStore();
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [filteredBranches, setFilteredBranches] = useState<BranchInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [showActiveOnly, setShowActiveOnly] = useState(false);
  const [showTreeView, setShowTreeView] = useState(false);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  
  // Branch version management state
  const [importModalVisible, setImportModalVisible] = useState(false);
  const [historyModalVisible, setHistoryModalVisible] = useState(false);
  const [selectedBranchForImport, setSelectedBranchForImport] = useState<BranchInfo | null>(null);
  const [selectedBranchForHistory, setSelectedBranchForHistory] = useState<BranchInfo | null>(null);

  useEffect(() => {
    fetchBranches();
  }, [asset.id, token]);

  useEffect(() => {
    filterBranches();
  }, [branches, searchTerm, showActiveOnly]);

  const fetchBranches = useCallback(async () => {
    if (!token) return;

    setLoading(true);
    try {
      const response = await invoke<BranchInfo[]>('get_branches', {
        token,
        assetId: asset.id
      });
      
      const sortedBranches = sortBranchesByCreated(response);
      setBranches(sortedBranches);
      
    } catch (error) {
      console.error('Failed to fetch branches:', error);
      message.error(`Failed to fetch branches: ${error}`);
    } finally {
      setLoading(false);
    }
  }, [token, asset.id]);

  const filterBranches = useCallback(() => {
    let filtered = branches;

    // Filter by search term
    if (searchTerm) {
      filtered = filtered.filter(branch => 
        branch.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        branch.description?.toLowerCase().includes(searchTerm.toLowerCase()) ||
        branch.created_by_username.toLowerCase().includes(searchTerm.toLowerCase())
      );
    }

    // Filter by active status
    if (showActiveOnly) {
      filtered = filtered.filter(branch => branch.is_active);
    }

    setFilteredBranches(filtered);
  }, [branches, searchTerm, showActiveOnly]);

  const handleRefresh = () => {
    fetchBranches();
  };

  const handleCreateBranch = () => {
    if (onCreateBranch) {
      onCreateBranch();
    }
  };

  const handleSearchChange = (value: string) => {
    setSearchTerm(value);
  };

  const handleFilterToggle = () => {
    setShowActiveOnly(!showActiveOnly);
  };

  const handleViewToggle = () => {
    setShowTreeView(!showTreeView);
  };

  const handleUpdateBranch = (branch: BranchInfo) => {
    setSelectedBranchForImport(branch);
    setImportModalVisible(true);
  };

  const handleViewBranchHistory = (branch: BranchInfo) => {
    setSelectedBranchForHistory(branch);
    setHistoryModalVisible(true);
  };

  const handleImportSuccess = () => {
    setImportModalVisible(false);
    setSelectedBranchForImport(null);
    message.success('Version imported successfully!');
    // Refresh branch versions if needed
    if (selectedBranchForImport && token) {
      fetchBranchVersions(token, selectedBranchForImport.id);
    }
  };

  const handleImportCancel = () => {
    setImportModalVisible(false);
    setSelectedBranchForImport(null);
  };

  const handleHistoryClose = () => {
    setHistoryModalVisible(false);
    setSelectedBranchForHistory(null);
  };

  const branchTreeData = useMemo(() => {
    // Create a map of version ID to version info
    const versionMap = new Map<number, ConfigurationVersionInfo>();
    versions.forEach(version => {
      versionMap.set(version.id, version);
    });

    // Create tree structure
    const treeData: any[] = [];
    
    // Group branches by parent version
    const branchGroups = new Map<number, BranchInfo[]>();
    filteredBranches.forEach(branch => {
      const parentId = branch.parent_version_id;
      if (!branchGroups.has(parentId)) {
        branchGroups.set(parentId, []);
      }
      branchGroups.get(parentId)!.push(branch);
    });

    // Create tree nodes
    branchGroups.forEach((branches, parentVersionId) => {
      const parentVersion = versionMap.get(parentVersionId);
      if (parentVersion) {
        const versionNode = {
          title: (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <FileOutlined style={{ color: '#1890ff' }} />
              <Text strong>{parentVersion.version_number}</Text>
              <Text type="secondary">({parentVersion.file_name})</Text>
              <Tag color="blue">{branches.length} branches</Tag>
            </div>
          ),
          key: `version-${parentVersionId}`,
          icon: <FileOutlined />,
          children: branches.map(branch => ({
            title: (
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <BranchesOutlined style={{ color: branch.is_active ? '#52c41a' : '#ff7875' }} />
                <Text strong={branch.is_active}>{branch.name}</Text>
                <Tag color={branch.is_active ? 'green' : 'red'}>
                  {branch.is_active ? 'Active' : 'Inactive'}
                </Tag>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  by {branch.created_by_username}
                </Text>
              </div>
            ),
            key: `branch-${branch.id}`,
            icon: <BranchesOutlined />,
            isLeaf: true,
            data: branch
          }))
        };
        treeData.push(versionNode);
      }
    });

    return treeData;
  }, [filteredBranches, versions]);

  const { activeBranchCount, totalBranchCount } = useMemo(() => {
    const active = branches.filter(b => b.is_active).length;
    const total = branches.length;
    return { activeBranchCount: active, totalBranchCount: total };
  }, [branches]);

  const getBranchVersionInfo = useCallback((branchId: number) => {
    const versions = branchVersions[branchId] || [];
    const count = versions.length;
    const latest = versions.find(v => v.is_branch_latest);
    return {
      count,
      latestVersionNumber: latest?.branch_version_number
    };
  }, [branchVersions]);

  return (
    <div style={{ padding: '24px' }}>
      {/* Header */}
      <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '16px' }}>
          <div>
            <Title level={4} style={{ margin: 0, marginBottom: '8px' }}>
              <BranchesOutlined /> Branch Management
            </Title>
            <Text type="secondary">
              Manage configuration branches for {asset.name}
            </Text>
          </div>
          
          {showCreateButton && (
            <Button 
              type="primary" 
              icon={<PlusOutlined />}
              onClick={handleCreateBranch}
            >
              Create Branch
            </Button>
          )}
        </div>

        {/* Statistics */}
        <Row gutter={16} style={{ marginBottom: '16px' }}>
          <Col span={8}>
            <Card size="small" style={{ textAlign: 'center' }}>
              <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#52c41a' }}>
                {activeBranchCount}
              </div>
              <Text type="secondary">Active Branches</Text>
            </Card>
          </Col>
          <Col span={8}>
            <Card size="small" style={{ textAlign: 'center' }}>
              <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#1890ff' }}>
                {totalBranchCount}
              </div>
              <Text type="secondary">Total Branches</Text>
            </Card>
          </Col>
          <Col span={8}>
            <Card size="small" style={{ textAlign: 'center' }}>
              <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#722ed1' }}>
                {asset.version_count}
              </div>
              <Text type="secondary">Total Versions</Text>
            </Card>
          </Col>
        </Row>

        {/* Controls */}
        <Row gutter={16} align="middle" style={{ marginBottom: '16px' }}>
          <Col flex="auto">
            <Search
              placeholder="Search branches by name, description, or creator..."
              allowClear
              onChange={(e) => handleSearchChange(e.target.value)}
              style={{ width: '100%' }}
              prefix={<SearchOutlined />}
            />
          </Col>
          <Col>
            <Space>
              <Button
                type={showActiveOnly ? 'primary' : 'default'}
                icon={<FilterOutlined />}
                onClick={handleFilterToggle}
                size="middle"
              >
                {showActiveOnly ? 'All Branches' : 'Active Only'}
              </Button>
              <Tooltip title={showTreeView ? 'Switch to list view' : 'Switch to tree view'}>
                <Button
                  type={showTreeView ? 'primary' : 'default'}
                  icon={showTreeView ? <UnorderedListOutlined /> : <NodeIndexOutlined />}
                  onClick={handleViewToggle}
                  size="middle"
                >
                  {showTreeView ? 'List View' : 'Tree View'}
                </Button>
              </Tooltip>
              <Button 
                icon={<ReloadOutlined />} 
                onClick={handleRefresh}
                loading={loading}
                size="middle"
              >
                Refresh
              </Button>
            </Space>
          </Col>
        </Row>

        {/* Info Alert */}
        <Alert
          message="Branch Management"
          description="Branches allow you to safely experiment with configuration changes without affecting the main development line. Each branch maintains its own history and can be merged or kept separate."
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          style={{ marginBottom: '16px' }}
        />
      </div>

      <Divider style={{ margin: '24px 0' }} />

      {/* Branch List */}
      <div>
        {loading ? (
          <div style={{ textAlign: 'center', padding: '48px' }}>
            <Spin size="large" />
            <div style={{ marginTop: '16px' }}>
              <Text type="secondary">Loading branches...</Text>
            </div>
          </div>
        ) : filteredBranches.length === 0 ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <div>
                <Title level={5}>
                  {branches.length === 0 ? 'No Branches Yet' : 'No Matching Branches'}
                </Title>
                <Text type="secondary">
                  {branches.length === 0 
                    ? 'This asset has no branches yet. Create your first branch to start experimenting with configurations.'
                    : 'No branches match your current search and filter criteria.'
                  }
                </Text>
              </div>
            }
            style={{ padding: '48px' }}
          />
        ) : (
          <div>
            <div style={{ marginBottom: '16px' }}>
              <Space>
                <Text type="secondary">
                  Showing {filteredBranches.length} of {totalBranchCount} branches
                </Text>
                {searchTerm && (
                  <Tag color="blue">
                    Search: "{searchTerm}"
                  </Tag>
                )}
                {showActiveOnly && (
                  <Tag color="green">
                    Active Only
                  </Tag>
                )}
              </Space>
            </div>
            
            {showTreeView ? (
              <div style={{ marginTop: '16px' }}>
                <Alert
                  message="Tree View"
                  description="This view shows branches organized by their parent configuration version, making it easier to understand the branching structure."
                  type="info"
                  showIcon
                  icon={<NodeIndexOutlined />}
                  style={{ marginBottom: '16px' }}
                />
                <Tree
                  treeData={branchTreeData}
                  expandedKeys={expandedKeys}
                  onExpand={(keys) => setExpandedKeys(keys)}
                  defaultExpandAll
                  showIcon
                  style={{ 
                    background: '#fafafa',
                    padding: '16px',
                    borderRadius: '8px',
                    border: '1px solid #f0f0f0'
                  }}
                  aria-label="Branch relationship tree"
                />
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0px' }}>
                {filteredBranches.map((branch) => {
                  const versionInfo = getBranchVersionInfo(branch.id);
                  return (
                    <BranchCard
                      key={branch.id}
                      branch={branch}
                      onViewDetails={onViewBranchDetails}
                      onSelectBranch={showSelectActions ? onSelectBranch : undefined}
                      onUpdateBranch={handleUpdateBranch}
                      onViewHistory={handleViewBranchHistory}
                      showActions={showSelectActions}
                      versionCount={versionInfo.count}
                      latestVersionNumber={versionInfo.latestVersionNumber}
                    />
                  );
                })}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Import Version Modal */}
      {selectedBranchForImport && (
        <ImportVersionToBranchModal
          visible={importModalVisible}
          onCancel={handleImportCancel}
          onSuccess={handleImportSuccess}
          branch={selectedBranchForImport}
        />
      )}

      {/* Branch Version History Modal */}
      {selectedBranchForHistory && (
        <Card
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            zIndex: 1000,
            background: 'rgba(0, 0, 0, 0.5)',
            display: historyModalVisible ? 'flex' : 'none',
            alignItems: 'center',
            justifyContent: 'center'
          }}
          onClick={handleHistoryClose}
        >
          <div
            style={{
              background: 'white',
              borderRadius: '8px',
              maxWidth: '90vw',
              maxHeight: '90vh',
              overflow: 'auto',
              padding: '0',
              position: 'relative'
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ padding: '16px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                <Title level={4} style={{ margin: 0 }}>
                  Branch Version History
                </Title>
                <Button type="text" onClick={handleHistoryClose}>
                  ✕
                </Button>
              </div>
              <BranchVersionHistory branch={selectedBranchForHistory} />
            </div>
          </div>
        </Card>
      )}
    </div>
  );
};

export default BranchManagement;