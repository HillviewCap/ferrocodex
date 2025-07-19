import React, { useEffect, useState } from 'react';
import {
  Card,
  List,
  Typography,
  Space,
  Tag,
  Button,
  Modal,
  Empty,
  Spin,
  Alert,
  Row,
  Col,
  Statistic,
  Divider
} from 'antd';
import {
  FileOutlined,
  CalendarOutlined,
  SwapOutlined,
  HistoryOutlined,
  TagOutlined,
  FileTextOutlined
} from '@ant-design/icons';
import { BranchInfo, BranchVersionInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import useBranchStore from '../store/branches';
import BranchVersionCard from './BranchVersionCard';

const { Text } = Typography;

interface BranchVersionHistoryProps {
  branch: BranchInfo;
}

const BranchVersionHistory: React.FC<BranchVersionHistoryProps> = ({
  branch
}) => {
  const { token } = useAuthStore();
  const { 
    branchVersions, 
    isLoadingVersions, 
    versionsError,
    fetchBranchVersions,
    clearVersionsError 
  } = useBranchStore();
  
  const [selectedVersions, setSelectedVersions] = useState<BranchVersionInfo[]>([]);
  const [compareModalVisible, setCompareModalVisible] = useState(false);
  const [compareResult, setCompareResult] = useState<string>('');
  const [isComparing, setIsComparing] = useState(false);

  const versions = branchVersions[branch.id] || [];

  useEffect(() => {
    if (token && branch.id) {
      fetchBranchVersions(token, branch.id);
    }
  }, [token, branch.id, fetchBranchVersions]);

  useEffect(() => {
    if (versionsError) {
      clearVersionsError();
    }
  }, [versionsError, clearVersionsError]);

  const handleVersionSelect = (version: BranchVersionInfo, selected: boolean) => {
    if (selected) {
      if (selectedVersions.length < 2) {
        setSelectedVersions([...selectedVersions, version]);
      }
    } else {
      setSelectedVersions(selectedVersions.filter(v => v.id !== version.id));
    }
  };

  const handleCompareVersions = async () => {
    if (selectedVersions.length !== 2 || !token) return;

    setIsComparing(true);
    setCompareModalVisible(true);

    try {
      const { compareBranchVersions } = useBranchStore.getState();
      const result = await compareBranchVersions(
        token,
        branch.id,
        selectedVersions[0].version_id,
        selectedVersions[1].version_id
      );
      setCompareResult(result);
    } catch (error) {
      console.error('Compare failed:', error);
      setCompareResult(`Error comparing versions: ${error}`);
    } finally {
      setIsComparing(false);
    }
  };

  const handleCloseCompare = () => {
    setCompareModalVisible(false);
    setCompareResult('');
    setSelectedVersions([]);
  };

  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const getVersionColor = (versionNumber: string) => {
    if (versionNumber.includes('branch-v1')) return 'green';
    if (versionNumber.includes('branch-v')) {
      const num = parseInt(versionNumber.replace('branch-v', ''));
      if (num <= 3) return 'blue';
      if (num <= 6) return 'orange';
      return 'purple';
    }
    return 'default';
  };

  const latestVersion = versions.find(v => v.is_branch_latest);
  const totalVersions = versions.length;
  const totalSize = versions.reduce((sum, v) => sum + v.file_size, 0);

  if (isLoadingVersions) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '40px' }}>
          <Spin size="large" />
          <div style={{ marginTop: '16px' }}>
            <Text type="secondary">Loading branch versions...</Text>
          </div>
        </div>
      </Card>
    );
  }

  if (versionsError) {
    return (
      <Card>
        <Alert
          message="Error Loading Versions"
          description={versionsError}
          type="error"
          showIcon
          action={
            <Button 
              size="small" 
              type="primary" 
              onClick={() => token && fetchBranchVersions(token, branch.id)}
            >
              Retry
            </Button>
          }
        />
      </Card>
    );
  }

  return (
    <div>
      <Card
        title={
          <Space>
            <HistoryOutlined />
            <span>Branch Version History</span>
            <Tag color="green">{branch.name}</Tag>
          </Space>
        }
        extra={
          selectedVersions.length === 2 && (
            <Button
              type="primary"
              icon={<SwapOutlined />}
              onClick={handleCompareVersions}
            >
              Compare Selected
            </Button>
          )
        }
      >
        {/* Branch Version Statistics */}
        <Row gutter={16} style={{ marginBottom: '24px' }}>
          <Col span={6}>
            <Statistic
              title="Total Versions"
              value={totalVersions}
              prefix={<TagOutlined />}
            />
          </Col>
          <Col span={6}>
            <Statistic
              title="Latest Version"
              value={latestVersion?.branch_version_number || 'None'}
              prefix={<FileTextOutlined />}
            />
          </Col>
          <Col span={6}>
            <Statistic
              title="Total Size"
              value={formatFileSize(totalSize)}
              prefix={<FileOutlined />}
            />
          </Col>
          <Col span={6}>
            <Statistic
              title="Last Updated"
              value={latestVersion ? formatDate(latestVersion.created_at) : 'Never'}
              prefix={<CalendarOutlined />}
            />
          </Col>
        </Row>

        <Divider />

        {selectedVersions.length > 0 && (
          <Alert
            message={`${selectedVersions.length} version${selectedVersions.length > 1 ? 's' : ''} selected for comparison`}
            description={
              selectedVersions.length === 2 
                ? 'Click "Compare Selected" to view differences between the selected versions.'
                : selectedVersions.length === 1
                ? 'Select one more version to compare.'
                : 'You can compare up to 2 versions.'
            }
            type="info"
            showIcon
            style={{ marginBottom: '16px' }}
            action={
              selectedVersions.length > 0 && (
                <Button size="small" onClick={() => setSelectedVersions([])}>
                  Clear Selection
                </Button>
              )
            }
          />
        )}

        {versions.length === 0 ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <div>
                <Text type="secondary">No versions found for this branch</Text>
                <br />
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  Import a configuration file to create the first version
                </Text>
              </div>
            }
          />
        ) : (
          <List
            itemLayout="vertical"
            size="small"
            dataSource={versions}
            renderItem={(version, index) => (
              <BranchVersionCard
                key={version.id}
                version={version}
                isFirst={index === 0}
                isLast={index === versions.length - 1}
                isSelected={selectedVersions.some(v => v.id === version.id)}
                onSelect={(selected) => handleVersionSelect(version, selected)}
                selectionMode={selectedVersions.length > 0 || selectedVersions.length < 2}
                onView={() => {
                  // Handle view version
                  console.log('View version:', version);
                }}
              />
            )}
          />
        )}
      </Card>

      {/* Compare Modal */}
      <Modal
        title={
          <Space>
            <SwapOutlined />
            Compare Branch Versions
          </Space>
        }
        open={compareModalVisible}
        onCancel={handleCloseCompare}
        footer={[
          <Button key="close" onClick={handleCloseCompare}>
            Close
          </Button>
        ]}
        width={1000}
      >
        {isComparing ? (
          <div style={{ textAlign: 'center', padding: '40px' }}>
            <Spin size="large" />
            <div style={{ marginTop: '16px' }}>
              <Text>Comparing versions...</Text>
            </div>
          </div>
        ) : (
          <div>
            {selectedVersions.length === 2 && (
              <div style={{ marginBottom: '16px' }}>
                <Row gutter={16}>
                  <Col span={12}>
                    <Card size="small" title="Version 1">
                      <Space direction="vertical" size="small" style={{ width: '100%' }}>
                        <Space>
                          <Tag color={getVersionColor(selectedVersions[0].branch_version_number)}>
                            {selectedVersions[0].branch_version_number}
                          </Tag>
                          <Text strong>{selectedVersions[0].file_name}</Text>
                        </Space>
                        <Text type="secondary" style={{ fontSize: '12px' }}>
                          {formatDate(selectedVersions[0].created_at)} • {selectedVersions[0].author_username}
                        </Text>
                      </Space>
                    </Card>
                  </Col>
                  <Col span={12}>
                    <Card size="small" title="Version 2">
                      <Space direction="vertical" size="small" style={{ width: '100%' }}>
                        <Space>
                          <Tag color={getVersionColor(selectedVersions[1].branch_version_number)}>
                            {selectedVersions[1].branch_version_number}
                          </Tag>
                          <Text strong>{selectedVersions[1].file_name}</Text>
                        </Space>
                        <Text type="secondary" style={{ fontSize: '12px' }}>
                          {formatDate(selectedVersions[1].created_at)} • {selectedVersions[1].author_username}
                        </Text>
                      </Space>
                    </Card>
                  </Col>
                </Row>
              </div>
            )}
            
            <Card size="small" title="Comparison Result">
              <pre style={{ 
                backgroundColor: '#f5f5f5', 
                padding: '16px', 
                borderRadius: '4px',
                maxHeight: '400px',
                overflow: 'auto',
                fontSize: '12px',
                fontFamily: 'monospace'
              }}>
                {compareResult || 'No comparison result available'}
              </pre>
            </Card>
          </div>
        )}
      </Modal>
    </div>
  );
};

export default BranchVersionHistory;