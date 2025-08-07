import React from 'react';
import { Typography, Space, Card, Descriptions, Button, Row, Col, Tag, Divider } from 'antd';
import { 
  CheckCircleOutlined, 
  FolderOutlined, 
  DesktopOutlined, 
  SecurityScanOutlined,
  SettingOutlined,
  HomeOutlined 
} from '@ant-design/icons';
import { BaseStepProps } from '../../../types/workflow';

const { Title, Text, Paragraph } = Typography;

interface ReviewConfirmationStepProps extends BaseStepProps {
  onComplete: () => void;
}

export const ReviewConfirmationStep: React.FC<ReviewConfirmationStepProps> = ({
  workflowId,
  data,
  onComplete
}) => {
  const getAssetTypeIcon = () => {
    return data.asset_type === 'Folder' 
      ? <FolderOutlined style={{ color: '#1890ff', fontSize: '24px' }} />
      : <DesktopOutlined style={{ color: '#52c41a', fontSize: '24px' }} />;
  };

  const getLocationDisplay = () => {
    if (data.parent_id === null) {
      return (
        <Space>
          <HomeOutlined style={{ color: '#1890ff' }} />
          <Text>Root Level</Text>
        </Space>
      );
    }
    return (
      <Space>
        <FolderOutlined style={{ color: '#52c41a' }} />
        <Text>{data.parent_path || 'Selected Folder'}</Text>
      </Space>
    );
  };

  const getSecurityClassificationColor = (classification?: string) => {
    switch (classification?.toLowerCase()) {
      case 'public': return 'green';
      case 'internal': return 'blue';
      case 'confidential': return 'orange';
      case 'restricted': return 'red';
      default: return 'default';
    }
  };

  const formatMetadataValues = (values?: Record<string, any>) => {
    if (!values || Object.keys(values).length === 0) {
      return 'No metadata configured';
    }

    return Object.entries(values).map(([key, value]) => (
      <div key={key} style={{ marginBottom: '4px' }}>
        <Text strong>{key}:</Text> <Text>{String(value)}</Text>
      </div>
    ));
  };

  return (
    <div>
      <div style={{ marginBottom: '32px', textAlign: 'center' }}>
        <Title level={3}>Review & Confirm</Title>
        <Paragraph type="secondary">
          Please review the asset configuration below. Once confirmed, the asset will be created.
        </Paragraph>
      </div>

      <Row gutter={24}>
        <Col span={16}>
          <Card style={{ marginBottom: '24px' }}>
            <div style={{ display: 'flex', alignItems: 'center', marginBottom: '20px' }}>
              {getAssetTypeIcon()}
              <div style={{ marginLeft: '12px' }}>
                <Title level={4} style={{ margin: '0 0 4px 0' }}>
                  {data.asset_name}
                </Title>
                <Tag color={data.asset_type === 'Folder' ? 'blue' : 'green'}>
                  {data.asset_type}
                </Tag>
              </div>
            </div>

            <Descriptions column={1} bordered>
              <Descriptions.Item label="Name">
                {data.asset_name}
              </Descriptions.Item>
              
              <Descriptions.Item label="Description">
                {data.asset_description || <Text type="secondary">No description provided</Text>}
              </Descriptions.Item>
              
              <Descriptions.Item label="Type">
                <Space>
                  {data.asset_type === 'Folder' ? <FolderOutlined /> : <DesktopOutlined />}
                  {data.asset_type}
                </Space>
              </Descriptions.Item>
              
              <Descriptions.Item label="Location">
                {getLocationDisplay()}
              </Descriptions.Item>
              
              <Descriptions.Item label="Security Classification">
                <Space>
                  <SecurityScanOutlined style={{ color: getSecurityClassificationColor(data.security_classification) }} />
                  <Tag color={getSecurityClassificationColor(data.security_classification)}>
                    {data.security_classification || 'Not set'}
                  </Tag>
                </Space>
              </Descriptions.Item>
            </Descriptions>
          </Card>

          <Card>
            <div style={{ display: 'flex', alignItems: 'center', marginBottom: '16px' }}>
              <SettingOutlined style={{ color: '#1890ff', fontSize: '18px', marginRight: '8px' }} />
              <Title level={5} style={{ margin: 0 }}>Metadata Configuration</Title>
            </div>
            
            <div style={{ padding: '12px', backgroundColor: '#fafafa', borderRadius: '6px' }}>
              {data.metadata_schema_id ? (
                <div>
                  <Text strong>Schema ID:</Text> <Text>{data.metadata_schema_id}</Text>
                  <Divider style={{ margin: '12px 0' }} />
                  {formatMetadataValues(data.metadata_values)}
                </div>
              ) : (
                <Text type="secondary">No metadata schema selected</Text>
              )}
            </div>
          </Card>
        </Col>

        <Col span={8}>
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', marginBottom: '16px' }}>
              <CheckCircleOutlined style={{ color: '#52c41a', fontSize: '18px', marginRight: '8px' }} />
              <Title level={5} style={{ margin: 0 }}>Ready to Create</Title>
            </div>
            
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text type="secondary">
                All required information has been provided and validated. 
                The asset will be created with the configuration shown.
              </Text>
              
              <Button
                type="primary"
                size="large"
                block
                onClick={onComplete}
                style={{ marginTop: '16px' }}
              >
                Create Asset
              </Button>
            </Space>
          </Card>

          <Card style={{ marginTop: '16px' }}>
            <Title level={5} style={{ margin: '0 0 12px 0' }}>What happens next?</Title>
            <ul style={{ margin: 0, paddingLeft: '20px' }}>
              <li style={{ marginBottom: '8px' }}>
                <Text type="secondary">Asset will be created in the database</Text>
              </li>
              <li style={{ marginBottom: '8px' }}>
                <Text type="secondary">Hierarchy will be updated</Text>
              </li>
              <li style={{ marginBottom: '8px' }}>
                <Text type="secondary">Metadata will be applied</Text>
              </li>
              <li style={{ marginBottom: '8px' }}>
                <Text type="secondary">Security settings will be enforced</Text>
              </li>
              <li style={{ marginBottom: '8px' }}>
                <Text type="secondary">Audit trail will be created</Text>
              </li>
            </ul>
          </Card>
        </Col>
      </Row>

      <div style={{ marginTop: '24px', padding: '16px', backgroundColor: '#e6f7ff', borderRadius: '6px', border: '1px solid #91d5ff' }}>
        <Text strong style={{ color: '#1890ff' }}>Note:</Text>
        <Text style={{ color: '#1890ff', marginLeft: '8px' }}>
          After creation, you can upload configuration files, manage versions, and update metadata as needed.
          {data.asset_type === 'Folder' && ' You can also create additional assets within this folder.'}
        </Text>
      </div>
    </div>
  );
};