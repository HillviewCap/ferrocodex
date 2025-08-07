import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Descriptions, 
  Empty, 
  Button, 
  Space, 
  Spin, 
  Alert, 
  Typography,
  Divider,
  Tag,
  Row,
  Col,
  message
} from 'antd';
import { 
  EditOutlined, 
  ReloadOutlined,
  FileTextOutlined,
  CalendarOutlined,
  UserOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import DynamicMetadataForm from '../forms/DynamicMetadataForm';
import dayjs from 'dayjs';

const { Text, Title } = Typography;

interface MetadataDisplayProps {
  assetId: number;
  schemaId?: number;
  onRefresh?: () => void;
}

interface MetadataField {
  name: string;
  label: string;
  value: any;
  type: string;
  category?: string;
}

interface AssetMetadataFull {
  current: {
    asset_id: number;
    schema_id: number;
    metadata_values_json: string;  // This is a JSON string from Rust
    schema_version: number;
    created_at: string;
    updated_at: string;
    created_by?: number;
    version?: number;
  } | null;
  schema: MetadataSchema | null;
  history: any[];
  validation_status: any;
  related_assets: any[];
}

interface MetadataSchema {
  id: number;
  name: string;
  description: string;
  schema_json: string;
}

const MetadataDisplay: React.FC<MetadataDisplayProps> = ({
  assetId,
  schemaId = 1,
  onRefresh
}) => {
  const { token } = useAuthStore();
  const [loading, setLoading] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [metadata, setMetadata] = useState<AssetMetadataFull['current'] | null>(null);
  const [schema, setSchema] = useState<MetadataSchema | null>(null);
  const [fields, setFields] = useState<MetadataField[]>([]);
  const [error, setError] = useState<string | null>(null);

  const loadMetadata = async () => {
    if (!token || !assetId) return;

    setLoading(true);
    setError(null);

    try {
      // Load the schema first
      const schemaData = await invoke<MetadataSchema>('get_metadata_schema_by_id', {
        token,
        schemaId
      });
      setSchema(schemaData);

      // Try to load existing metadata
      try {
        const metadataData = await invoke<AssetMetadataFull>('get_asset_metadata_full', {
          token,
          assetId: assetId,
          includeHistory: false
        });
        
        // Extract the current metadata from the full response
        if (metadataData.current) {
          // Parse the metadata_values_json string
          let metadataValues = {};
          try {
            if (metadataData.current.metadata_values_json) {
              metadataValues = JSON.parse(metadataData.current.metadata_values_json);
            }
          } catch (e) {
            console.error('Failed to parse metadata_values_json:', e);
          }
          
          // Store metadata with parsed values
          const metadataWithParsedValues = {
            ...metadataData.current,
            metadata_values: metadataValues
          };
          setMetadata(metadataWithParsedValues as any);
          
          // Parse schema and extract field definitions
          if (schemaData) {
            const parsedSchema = JSON.parse(schemaData.schema_json);
            const fieldList = parseFieldsFromSchema(parsedSchema, metadataValues);
            setFields(fieldList);
          }
        } else {
          // No metadata exists yet - parse schema to show empty fields
          console.log('No metadata exists for asset:', assetId);
          setMetadata(null);
          
          // Still parse the schema to show available fields
          if (schemaData) {
            const parsedSchema = JSON.parse(schemaData.schema_json);
            const fieldList = parseFieldsFromSchema(parsedSchema, {});
            setFields(fieldList);
          } else {
            setFields([]);
          }
        }
      } catch (metadataError) {
        // Error loading metadata
        console.error('Failed to load metadata:', metadataError);
        setMetadata(null);
        setFields([]);
      }
    } catch (err) {
      console.error('Failed to load metadata:', err);
      setError(err instanceof Error ? err.message : 'Failed to load metadata');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadMetadata();
  }, [assetId, token]);

  const parseFieldsFromSchema = (
    schema: any, 
    values: Record<string, any> | null | undefined
  ): MetadataField[] => {
    const fields: MetadataField[] = [];
    const properties = schema.properties || {};
    const fieldOrder = schema.fieldOrder || Object.keys(properties);
    const safeValues = values || {};

    fieldOrder.forEach((fieldName: string) => {
      const property = properties[fieldName];
      if (!property) return;

      // Map schema types to our field types
      let fieldType = property.type;
      
      // Check for date/datetime format
      if (property.type === 'string' && (property.format === 'date' || property.format === 'date-time')) {
        fieldType = 'date';
      }
      
      // Check for boolean
      if (property.type === 'boolean') {
        fieldType = 'boolean';
      }
      
      // Check for array
      if (property.type === 'array') {
        fieldType = 'array';
      }

      const field: MetadataField = {
        name: fieldName,
        label: property.title || fieldName,
        value: safeValues[fieldName] ?? null,
        type: fieldType,
        category: property.category || 'General'
      };

      fields.push(field);
    });

    return fields;
  };

  const handleEdit = () => {
    setEditMode(true);
  };

  const handleSave = async (values: Record<string, any>) => {
    try {
      // Convert values to field updates format - filter out undefined/null values
      const fieldUpdates = Object.entries(values)
        .filter(([_, value]) => value !== undefined && value !== null)
        .map(([field, value]) => ({
          field_path: field,
          new_value: value,
          operation: 'set'
        }));

      // If no fields to update, just return
      if (fieldUpdates.length === 0) {
        message.warning('No fields to update');
        setEditMode(false);
        return;
      }

      // Save metadata using the appropriate command
      await invoke('update_asset_metadata_partial', {
        token,
        assetId: assetId,
        updates: {
          schema_id: schemaId,  // Always provide schema_id for both create and update
          field_updates: fieldUpdates,
          preserve_history: true,
          change_reason: metadata ? 'User updated metadata' : 'Initial metadata creation'
        }
      });

      message.success('Metadata updated successfully');
      setEditMode(false);
      await loadMetadata();
      onRefresh?.();
    } catch (err) {
      console.error('Failed to save metadata:', err);
      message.error('Failed to save metadata');
    }
  };

  const handleCancel = () => {
    setEditMode(false);
  };

  const formatValue = (value: any, type: string): React.ReactNode => {
    if (value === null || value === undefined) {
      return <Text type="secondary">Not set</Text>;
    }

    switch (type) {
      case 'boolean':
        return value ? <Tag color="success">Yes</Tag> : <Tag color="default">No</Tag>;
      case 'number':
        return <Text strong>{value}</Text>;
      case 'date':
        // Format date values for display
        try {
          const date = dayjs(value);
          if (date.isValid()) {
            return <Text>{date.format('YYYY-MM-DD')}</Text>;
          }
        } catch (e) {
          console.error('Invalid date:', value);
        }
        return <Text>{String(value)}</Text>;
      case 'array':
        return (
          <Space wrap>
            {value.map((item: any, index: number) => (
              <Tag key={index}>{item}</Tag>
            ))}
          </Space>
        );
      default:
        return <Text>{String(value)}</Text>;
    }
  };

  const groupFieldsByCategory = (fields: MetadataField[]) => {
    const grouped: Record<string, MetadataField[]> = {};
    
    fields.forEach(field => {
      const category = field.category || 'General';
      if (!grouped[category]) {
        grouped[category] = [];
      }
      grouped[category].push(field);
    });

    return grouped;
  };

  if (loading) {
    return (
      <div style={{ padding: '24px', textAlign: 'center' }}>
        <Spin size="large" />
        <div style={{ marginTop: 16 }}>
          <Text type="secondary">Loading metadata...</Text>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: '24px' }}>
        <Alert
          message="Error Loading Metadata"
          description={error}
          type="error"
          showIcon
          action={
            <Button size="small" onClick={loadMetadata}>
              Retry
            </Button>
          }
        />
      </div>
    );
  }

  if (editMode) {
    // Pass values directly to the form - DynamicMetadataForm will handle date conversion
    const preparedValues = metadata?.metadata_values ? { ...metadata.metadata_values } : {};

    return (
      <div style={{ padding: '16px' }}>
        <DynamicMetadataForm
          assetId={assetId}
          schemaId={schemaId}
          initialValues={preparedValues}
          onSubmit={handleSave}
          onCancel={handleCancel}
          showTitle={false}
        />
      </div>
    );
  }

  if (!metadata || fields.length === 0) {
    // Show the empty state with option to add metadata

    return (
      <div style={{ padding: '24px' }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <Space direction="vertical">
              <Text>No metadata configured for this asset</Text>
              <Button 
                type="primary" 
                icon={<EditOutlined />}
                onClick={handleEdit}
              >
                Add Metadata
              </Button>
            </Space>
          }
        />
      </div>
    );
  }

  const groupedFields = groupFieldsByCategory(fields);

  return (
    <div style={{ padding: '16px' }}>
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Header */}
        <Row justify="space-between" align="middle">
          <Col>
            <Space>
              <FileTextOutlined style={{ fontSize: 18, color: '#1890ff' }} />
              <Title level={5} style={{ margin: 0 }}>
                {schema?.name || 'Asset Metadata'}
              </Title>
            </Space>
          </Col>
          <Col>
            <Space>
              <Button
                icon={<ReloadOutlined />}
                onClick={loadMetadata}
                size="small"
              >
                Refresh
              </Button>
              <Button
                type="primary"
                icon={<EditOutlined />}
                onClick={handleEdit}
                size="small"
              >
                Edit
              </Button>
            </Space>
          </Col>
        </Row>

        {/* Metadata Fields by Category */}
        {Object.entries(groupedFields).map(([category, categoryFields]) => (
          <Card 
            key={category}
            size="small"
            title={category}
            style={{ marginBottom: 16 }}
          >
            <Descriptions
              column={{ xs: 1, sm: 1, md: 2, lg: 2, xl: 2 }}
              size="small"
              labelStyle={{ fontWeight: 500, color: '#595959' }}
              contentStyle={{ color: '#262626' }}
            >
              {categoryFields.map(field => (
                <Descriptions.Item 
                  key={field.name} 
                  label={field.label}
                  span={field.type === 'array' ? 2 : 1}
                >
                  {formatValue(field.value, field.type)}
                </Descriptions.Item>
              ))}
            </Descriptions>
          </Card>
        ))}

        {/* Metadata Info */}
        <Card size="small" title="Metadata Information" style={{ marginTop: 8 }}>
          <Descriptions size="small" column={2}>
            <Descriptions.Item label="Schema Version">
              <Tag>{metadata.schema_version || metadata.version || 1}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Schema ID">
              <Text type="secondary">{metadata.schema_id}</Text>
            </Descriptions.Item>
            <Descriptions.Item label="Last Updated">
              <Space size="small">
                <CalendarOutlined style={{ color: '#8c8c8c' }} />
                <Text type="secondary">
                  {new Date(metadata.updated_at).toLocaleString()}
                </Text>
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="Updated By">
              <Space size="small">
                <UserOutlined style={{ color: '#8c8c8c' }} />
                <Text type="secondary">User #{metadata.created_by || 'Unknown'}</Text>
              </Space>
            </Descriptions.Item>
          </Descriptions>
        </Card>
      </Space>
    </div>
  );
};

export default MetadataDisplay;