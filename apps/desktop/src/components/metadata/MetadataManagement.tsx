import React, { useState } from 'react';
import { Tabs, Typography } from 'antd';
import { FileTextOutlined, AppstoreOutlined, SearchOutlined } from '@ant-design/icons';
import SchemaDesigner from '../forms/SchemaDesigner';
import FieldTemplateLibrary from './FieldTemplateLibrary';
import MetadataSearchPage from '../search/MetadataSearchPage';

const { Title, Paragraph } = Typography;

const MetadataManagement: React.FC = () => {
  const [activeTab, setActiveTab] = useState('templates');

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '24px 24px 0 24px' }}>
        <Title level={2} style={{ marginBottom: '8px' }}>Metadata Management</Title>
        <Paragraph type="secondary" style={{ marginBottom: '24px' }}>
          Design custom metadata schemas, browse field templates, and search assets by metadata.
        </Paragraph>
      </div>
      
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        style={{ flex: 1, padding: '0 24px' }}
        size="large"
        items={[
          {
            key: 'templates',
            label: (
              <span>
                <FileTextOutlined />
                Field Templates
              </span>
            ),
            children: <FieldTemplateLibrary />,
          },
          {
            key: 'designer',
            label: (
              <span>
                <AppstoreOutlined />
                Schema Designer
              </span>
            ),
            children: <SchemaDesigner />,
          },
          {
            key: 'search',
            label: (
              <span>
                <SearchOutlined />
                Metadata Search
              </span>
            ),
            children: <MetadataSearchPage />,
          },
        ]}
      />
    </div>
  );
};

export default MetadataManagement;