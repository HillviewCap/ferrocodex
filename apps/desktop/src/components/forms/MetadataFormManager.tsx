import React, { useState } from 'react';
import { Card, Tabs, message, Button, Space } from 'antd';
import { PlusOutlined, EditOutlined, EyeOutlined } from '@ant-design/icons';
import DynamicMetadataForm from './DynamicMetadataForm';
import SchemaDesigner from './SchemaDesigner';

const { TabPane } = Tabs;

interface MetadataFormManagerProps {
  assetId?: number;
  assetType?: string;
  mode?: 'form' | 'designer' | 'both';
  onClose?: () => void;
}

const MetadataFormManager: React.FC<MetadataFormManagerProps> = ({
  assetId,
  assetType,
  mode = 'both',
  onClose
}) => {
  const [activeTab, setActiveTab] = useState(mode === 'form' ? 'form' : 'designer');
  const [selectedSchemaId, setSelectedSchemaId] = useState<number | undefined>();
  const [formData, setFormData] = useState<Record<string, any>>({});

  const handleSchemaCreate = (schemaId: number) => {
    message.success('Schema created successfully');
    setSelectedSchemaId(schemaId);
    if (mode === 'both') {
      setActiveTab('form');
    }
  };

  const handleFormSubmit = async (values: Record<string, any>) => {
    try {
      // Here you would typically save the metadata to your backend
      console.log('Saving metadata:', values);
      
      // For demo purposes, we'll just show a success message
      message.success('Metadata saved successfully');
      setFormData(values);
    } catch (error) {
      console.error('Failed to save metadata:', error);
      message.error('Failed to save metadata');
    }
  };

  const handleFormCancel = () => {
    if (onClose) {
      onClose();
    } else {
      setFormData({});
    }
  };

  const renderFormTab = () => (
    <TabPane tab="Metadata Form" key="form" icon={<EditOutlined />}>
      {selectedSchemaId ? (
        <DynamicMetadataForm
          schemaId={selectedSchemaId}
          assetId={assetId}
          initialValues={formData}
          onSubmit={handleFormSubmit}
          onCancel={handleFormCancel}
          showTitle={false}
        />
      ) : (
        <Card>
          <div style={{ textAlign: 'center', padding: '50px 0' }}>
            <div style={{ marginBottom: 16 }}>
              No schema selected. Please create or select a metadata schema first.
            </div>
            <Button 
              type="primary" 
              icon={<PlusOutlined />}
              onClick={() => setActiveTab('designer')}
            >
              Create Schema
            </Button>
          </div>
        </Card>
      )}
    </TabPane>
  );

  const renderDesignerTab = () => (
    <TabPane tab="Schema Designer" key="designer" icon={<PlusOutlined />}>
      <SchemaDesigner
        onSave={handleSchemaCreate}
        onCancel={handleFormCancel}
      />
    </TabPane>
  );

  const renderPreviewTab = () => (
    <TabPane tab="Form Preview" key="preview" icon={<EyeOutlined />}>
      {selectedSchemaId ? (
        <DynamicMetadataForm
          schemaId={selectedSchemaId}
          initialValues={formData}
          onSubmit={handleFormSubmit}
          onCancel={handleFormCancel}
          readonly={true}
          showTitle={false}
        />
      ) : (
        <Card>
          <div style={{ textAlign: 'center', padding: '50px 0' }}>
            <div>No schema to preview. Please create a schema first.</div>
          </div>
        </Card>
      )}
    </TabPane>
  );

  if (mode === 'form' && selectedSchemaId) {
    return (
      <DynamicMetadataForm
        schemaId={selectedSchemaId}
        assetId={assetId}
        initialValues={formData}
        onSubmit={handleFormSubmit}
        onCancel={handleFormCancel}
      />
    );
  }

  if (mode === 'designer') {
    return (
      <SchemaDesigner
        onSave={handleSchemaCreate}
        onCancel={handleFormCancel}
      />
    );
  }

  return (
    <div style={{ height: '100%' }}>
      <Card 
        title="Metadata Management"
        extra={
          <Space>
            {formData && Object.keys(formData).length > 0 && (
              <Button onClick={() => setFormData({})}>
                Clear Data
              </Button>
            )}
            {onClose && (
              <Button onClick={onClose}>
                Close
              </Button>
            )}
          </Space>
        }
      >
        <Tabs 
          activeKey={activeTab} 
          onChange={setActiveTab}
          style={{ height: 'calc(100vh - 200px)' }}
        >
          {renderFormTab()}
          {renderDesignerTab()}
          {renderPreviewTab()}
        </Tabs>
      </Card>
    </div>
  );
};

export default MetadataFormManager;