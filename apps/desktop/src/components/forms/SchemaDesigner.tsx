import React, { useState, useCallback, useEffect } from 'react';
import {
  Card,
  Row,
  Col,
  Button,
  Form,
  Input,
  Select,
  Switch,
  Modal,
  message,
  Tabs,
  Space,
  Divider,
  Tooltip,
  Popconfirm,
  InputNumber
} from 'antd';
import {
  DragOutlined,
  PlusOutlined,
  DeleteOutlined,
  EyeOutlined,
  SaveOutlined,
  DownloadOutlined,
  UploadOutlined,
  CodeOutlined
} from '@ant-design/icons';
import { DragDropContext, Droppable, Draggable } from 'react-beautiful-dnd';
import DynamicMetadataForm from './DynamicMetadataForm';
import { invoke } from '@tauri-apps/api/core';

const { TextArea } = Input;
const { Option } = Select;
const { TabPane } = Tabs;

interface FieldDefinition {
  id: string;
  name: string;
  type: 'text' | 'number' | 'date' | 'dropdown' | 'checkbox' | 'textarea';
  label: string;
  description?: string;
  required: boolean;
  validation: {
    minLength?: number;
    maxLength?: number;
    min?: number;
    max?: number;
    pattern?: string;
  };
  options?: string[]; // For dropdown fields
  defaultValue?: any;
}

interface SchemaDesignerProps {
  schemaId?: number; // For editing existing schema
  onSave?: (schemaId: number) => void;
  onCancel?: () => void;
}

const FIELD_TYPES = [
  { value: 'text', label: 'Text Input', description: 'Single line text' },
  { value: 'textarea', label: 'Text Area', description: 'Multi-line text' },
  { value: 'number', label: 'Number', description: 'Numeric input' },
  { value: 'date', label: 'Date', description: 'Date picker' },
  { value: 'dropdown', label: 'Dropdown', description: 'Select from options' },
  { value: 'checkbox', label: 'Checkbox', description: 'True/false value' }
];

const SchemaDesigner: React.FC<SchemaDesignerProps> = ({
  schemaId,
  onSave,
  onCancel
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [fields, setFields] = useState<FieldDefinition[]>([]);
  const [activeTab, setActiveTab] = useState('designer');
  const [previewVisible, setPreviewVisible] = useState(false);
  const [editingField, setEditingField] = useState<FieldDefinition | null>(null);
  const [fieldFormVisible, setFieldFormVisible] = useState(false);
  const [jsonSchema, setJsonSchema] = useState('');
  const [schemaName, setSchemaName] = useState('');
  const [schemaDescription, setSchemaDescription] = useState('');
  const [assetTypeFilter, setAssetTypeFilter] = useState<string>('');

  // Load existing schema if editing
  useEffect(() => {
    if (schemaId) {
      loadExistingSchema();
    }
  }, [schemaId]);

  const loadExistingSchema = async () => {
    try {
      setLoading(true);
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const schema = await invoke<any>('get_metadata_schema_by_id', {
        token,
        schemaId
      });

      if (schema) {
        setSchemaName(schema.name);
        setSchemaDescription(schema.description);
        setAssetTypeFilter(schema.asset_type_filter || '');
        
        const parsedSchema = JSON.parse(schema.schema_json);
        setJsonSchema(JSON.stringify(parsedSchema, null, 2));
        
        // Convert schema to field definitions
        const fieldDefs = convertSchemaToFieldDefinitions(parsedSchema);
        setFields(fieldDefs);
      }
    } catch (error) {
      console.error('Failed to load schema:', error);
      message.error('Failed to load schema');
    } finally {
      setLoading(false);
    }
  };

  // Convert JSON Schema to field definitions
  const convertSchemaToFieldDefinitions = (schema: any): FieldDefinition[] => {
    const fieldDefs: FieldDefinition[] = [];
    const { properties, required = [] } = schema;

    Object.entries(properties || {}).forEach(([name, property]: [string, any]) => {
      const fieldDef: FieldDefinition = {
        id: `field_${Date.now()}_${Math.random()}`,
        name,
        type: mapSchemaTypeToFieldType(property),
        label: property.title || name,
        description: property.description,
        required: required.includes(name),
        validation: {
          minLength: property.minLength,
          maxLength: property.maxLength,
          min: property.minimum,
          max: property.maximum,
          pattern: property.pattern
        },
        options: property.enum || [],
        defaultValue: property.default
      };
      fieldDefs.push(fieldDef);
    });

    return fieldDefs;
  };

  // Map schema types to field types
  const mapSchemaTypeToFieldType = (property: any): FieldDefinition['type'] => {
    if (property.enum) return 'dropdown';
    
    switch (property.type) {
      case 'string':
        if (property.format === 'date') return 'date';
        if (property.maxLength && property.maxLength > 255) return 'textarea';
        return 'text';
      case 'number':
      case 'integer':
        return 'number';
      case 'boolean':
        return 'checkbox';
      default:
        return 'text';
    }
  };

  // Generate JSON Schema from field definitions
  const generateJsonSchema = useCallback(() => {
    const properties: Record<string, any> = {};
    const required: string[] = [];

    fields.forEach(field => {
      const property: any = {
        type: getJsonSchemaType(field.type),
        title: field.label,
        description: field.description
      };

      // Add validation rules
      if (field.validation.minLength) property.minLength = field.validation.minLength;
      if (field.validation.maxLength) property.maxLength = field.validation.maxLength;
      if (field.validation.min) property.minimum = field.validation.min;
      if (field.validation.max) property.maximum = field.validation.max;
      if (field.validation.pattern) property.pattern = field.validation.pattern;

      // Handle field-specific properties
      if (field.type === 'date') {
        property.format = 'date';
      } else if (field.type === 'dropdown' && field.options) {
        property.enum = field.options;
      }

      if (field.defaultValue !== undefined) {
        property.default = field.defaultValue;
      }

      properties[field.name] = property;

      if (field.required) {
        required.push(field.name);
      }
    });

    const schema = {
      type: 'object',
      title: schemaName,
      description: schemaDescription,
      properties,
      required,
      fieldOrder: fields.map(f => f.name)
    };

    const schemaJson = JSON.stringify(schema, null, 2);
    setJsonSchema(schemaJson);
    return schemaJson;
  }, [fields, schemaName, schemaDescription]);

  // Get JSON Schema type from field type
  const getJsonSchemaType = (fieldType: FieldDefinition['type']): string => {
    switch (fieldType) {
      case 'text':
      case 'textarea':
      case 'date':
      case 'dropdown':
        return 'string';
      case 'number':
        return 'number';
      case 'checkbox':
        return 'boolean';
      default:
        return 'string';
    }
  };

  // Update JSON schema when fields change
  useEffect(() => {
    generateJsonSchema();
  }, [generateJsonSchema]);

  // Handle drag and drop reordering
  const handleDragEnd = (result: any) => {
    if (!result.destination) return;

    const items = Array.from(fields);
    const [reorderedItem] = items.splice(result.source.index, 1);
    items.splice(result.destination.index, 0, reorderedItem);

    setFields(items);
  };

  // Add new field
  const addField = () => {
    const newField: FieldDefinition = {
      id: `field_${Date.now()}_${Math.random()}`,
      name: `field_${fields.length + 1}`,
      type: 'text',
      label: `Field ${fields.length + 1}`,
      required: false,
      validation: {}
    };
    setEditingField(newField);
    setFieldFormVisible(true);
  };

  // Edit existing field
  const editField = (field: FieldDefinition) => {
    setEditingField({ ...field });
    setFieldFormVisible(true);
  };

  // Save field changes
  const saveField = (fieldData: FieldDefinition) => {
    if (fields.find(f => f.id !== fieldData.id && f.name === fieldData.name)) {
      message.error('Field name must be unique');
      return;
    }

    const updatedFields = editingField?.id && fields.find(f => f.id === editingField.id)
      ? fields.map(f => f.id === fieldData.id ? fieldData : f)
      : [...fields, fieldData];

    setFields(updatedFields);
    setFieldFormVisible(false);
    setEditingField(null);
  };

  // Delete field
  const deleteField = (fieldId: string) => {
    setFields(fields.filter(f => f.id !== fieldId));
  };

  // Save schema
  const saveSchema = async () => {
    if (!schemaName.trim()) {
      message.error('Schema name is required');
      return;
    }

    if (fields.length === 0) {
      message.error('At least one field is required');
      return;
    }

    try {
      setLoading(true);
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const schemaJson = generateJsonSchema();

      if (schemaId) {
        // Update existing schema
        await invoke('update_metadata_schema', {
          token,
          schemaId,
          name: schemaName,
          description: schemaDescription,
          schemaJson,
          assetTypeFilter: assetTypeFilter || null
        });
        message.success('Schema updated successfully');
      } else {
        // Create new schema
        const result = await invoke<any>('create_metadata_schema', {
          token,
          name: schemaName,
          description: schemaDescription,
          schemaJson,
          assetTypeFilter: assetTypeFilter || null
        });
        message.success('Schema created successfully');
        if (onSave && result.id) {
          onSave(result.id);
        }
      }
    } catch (error) {
      console.error('Failed to save schema:', error);
      message.error('Failed to save schema');
    } finally {
      setLoading(false);
    }
  };

  // Export schema
  const exportSchema = () => {
    const schemaData = {
      name: schemaName,
      description: schemaDescription,
      schema: JSON.parse(jsonSchema),
      fields: fields
    };

    const blob = new Blob([JSON.stringify(schemaData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${schemaName.replace(/\s+/g, '_').toLowerCase()}_schema.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div style={{ height: '100%' }}>
      <Card 
        title="Schema Designer"
        extra={
          <Space>
            <Button icon={<EyeOutlined />} onClick={() => setPreviewVisible(true)}>
              Preview
            </Button>
            <Button icon={<DownloadOutlined />} onClick={exportSchema}>
              Export
            </Button>
            <Button 
              type="primary" 
              icon={<SaveOutlined />} 
              loading={loading}
              onClick={saveSchema}
            >
              Save Schema
            </Button>
          </Space>
        }
      >
        <Row gutter={16} style={{ marginBottom: 16 }}>
          <Col span={8}>
            <Input
              placeholder="Schema Name"
              value={schemaName}
              onChange={(e) => setSchemaName(e.target.value)}
            />
          </Col>
          <Col span={8}>
            <Input
              placeholder="Asset Type Filter (optional)"
              value={assetTypeFilter}
              onChange={(e) => setAssetTypeFilter(e.target.value)}
            />
          </Col>
          <Col span={8}>
            <TextArea
              placeholder="Schema Description"
              value={schemaDescription}
              onChange={(e) => setSchemaDescription(e.target.value)}
              rows={1}
            />
          </Col>
        </Row>

        <Tabs activeKey={activeTab} onChange={setActiveTab}>
          <TabPane tab="Visual Designer" key="designer">
            <Row gutter={16}>
              <Col span={16}>
                <Card 
                  title="Form Fields"
                  extra={
                    <Button 
                      type="primary" 
                      icon={<PlusOutlined />} 
                      onClick={addField}
                    >
                      Add Field
                    </Button>
                  }
                >
                  <DragDropContext onDragEnd={handleDragEnd}>
                    <Droppable droppableId="fields">
                      {(provided) => (
                        <div {...provided.droppableProps} ref={provided.innerRef}>
                          {fields.map((field, index) => (
                            <Draggable key={field.id} draggableId={field.id} index={index}>
                              {(provided) => (
                                <Card
                                  size="small"
                                  style={{ marginBottom: 8 }}
                                  ref={provided.innerRef}
                                  {...provided.draggableProps}
                                >
                                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                    <div {...provided.dragHandleProps}>
                                      <DragOutlined style={{ color: '#999' }} />
                                    </div>
                                    <div style={{ flex: 1 }}>
                                      <strong>{field.label}</strong>
                                      <div style={{ fontSize: 12, color: '#666' }}>
                                        {field.name} ({field.type})
                                        {field.required && <span style={{ color: 'red' }}> *</span>}
                                      </div>
                                    </div>
                                    <Space>
                                      <Button 
                                        size="small" 
                                        onClick={() => editField(field)}
                                      >
                                        Edit
                                      </Button>
                                      <Popconfirm
                                        title="Are you sure you want to delete this field?"
                                        onConfirm={() => deleteField(field.id)}
                                      >
                                        <Button 
                                          size="small" 
                                          danger 
                                          icon={<DeleteOutlined />}
                                        />
                                      </Popconfirm>
                                    </Space>
                                  </div>
                                </Card>
                              )}
                            </Draggable>
                          ))}
                          {provided.placeholder}
                        </div>
                      )}
                    </Droppable>
                  </DragDropContext>
                  
                  {fields.length === 0 && (
                    <div style={{ textAlign: 'center', padding: '40px 0', color: '#999' }}>
                      No fields added yet. Click "Add Field" to get started.
                    </div>
                  )}
                </Card>
              </Col>
              
              <Col span={8}>
                <Card title="Field Types" size="small">
                  {FIELD_TYPES.map(type => (
                    <div key={type.value} style={{ marginBottom: 8, padding: 8, border: '1px dashed #d9d9d9', borderRadius: 4 }}>
                      <div><strong>{type.label}</strong></div>
                      <div style={{ fontSize: 12, color: '#666' }}>{type.description}</div>
                    </div>
                  ))}
                </Card>
              </Col>
            </Row>
          </TabPane>

          <TabPane tab="JSON Schema" key="json">
            <TextArea
              value={jsonSchema}
              onChange={(e) => setJsonSchema(e.target.value)}
              rows={20}
              style={{ fontFamily: 'monospace' }}
            />
          </TabPane>
        </Tabs>
      </Card>

      {/* Field Configuration Modal */}
      <FieldConfigModal
        visible={fieldFormVisible}
        field={editingField}
        onSave={saveField}
        onCancel={() => {
          setFieldFormVisible(false);
          setEditingField(null);
        }}
      />

      {/* Form Preview Modal */}
      <Modal
        title="Form Preview"
        open={previewVisible}
        onCancel={() => setPreviewVisible(false)}
        footer={null}
        width={800}
      >
        {jsonSchema && schemaName && (
          <div>
            <h3>{schemaName}</h3>
            <p>{schemaDescription}</p>
            {/* Mock form preview - would use DynamicMetadataForm with mock schema ID */}
            <div style={{ border: '1px solid #d9d9d9', padding: 16, borderRadius: 4 }}>
              Form preview would go here...
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
};

// Field Configuration Modal Component
interface FieldConfigModalProps {
  visible: boolean;
  field: FieldDefinition | null;
  onSave: (field: FieldDefinition) => void;
  onCancel: () => void;
}

const FieldConfigModal: React.FC<FieldConfigModalProps> = ({
  visible,
  field,
  onSave,
  onCancel
}) => {
  const [form] = Form.useForm();
  const [fieldType, setFieldType] = useState<FieldDefinition['type']>('text');

  useEffect(() => {
    if (field) {
      form.setFieldsValue(field);
      setFieldType(field.type);
    } else {
      form.resetFields();
      setFieldType('text');
    }
  }, [field, form]);

  const handleSave = () => {
    form.validateFields().then(values => {
      const fieldData: FieldDefinition = {
        id: field?.id || `field_${Date.now()}_${Math.random()}`,
        ...values,
        type: fieldType,
        validation: values.validation || {},
        options: fieldType === 'dropdown' ? (values.options || []) : undefined
      };
      onSave(fieldData);
    });
  };

  return (
    <Modal
      title={field ? 'Edit Field' : 'Add Field'}
      open={visible}
      onOk={handleSave}
      onCancel={onCancel}
      width={600}
    >
      <Form form={form} layout="vertical">
        <Row gutter={16}>
          <Col span={12}>
            <Form.Item
              label="Field Name"
              name="name"
              rules={[{ required: true, message: 'Field name is required' }]}
            >
              <Input placeholder="field_name" />
            </Form.Item>
          </Col>
          <Col span={12}>
            <Form.Item
              label="Field Type"
              name="type"
              rules={[{ required: true }]}
            >
              <Select value={fieldType} onChange={setFieldType}>
                {FIELD_TYPES.map(type => (
                  <Option key={type.value} value={type.value}>
                    {type.label}
                  </Option>
                ))}
              </Select>
            </Form.Item>
          </Col>
        </Row>

        <Form.Item
          label="Display Label"
          name="label"
          rules={[{ required: true, message: 'Label is required' }]}
        >
          <Input placeholder="Field Label" />
        </Form.Item>

        <Form.Item label="Description" name="description">
          <TextArea placeholder="Optional field description" rows={2} />
        </Form.Item>

        <Form.Item name="required" valuePropName="checked">
          <Switch /> Required Field
        </Form.Item>

        {/* Validation Rules */}
        <Card title="Validation Rules" size="small">
          {(fieldType === 'text' || fieldType === 'textarea') && (
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item label="Min Length" name={['validation', 'minLength']}>
                  <InputNumber min={0} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item label="Max Length" name={['validation', 'maxLength']}>
                  <InputNumber min={0} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
            </Row>
          )}

          {fieldType === 'number' && (
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item label="Minimum" name={['validation', 'min']}>
                  <InputNumber style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item label="Maximum" name={['validation', 'max']}>
                  <InputNumber style={{ width: '100%' }} />
                </Form.Item>
              </Col>
            </Row>
          )}

          {(fieldType === 'text' || fieldType === 'textarea') && (
            <Form.Item label="Pattern (Regex)" name={['validation', 'pattern']}>
              <Input placeholder="^[a-zA-Z0-9]*$" />
            </Form.Item>
          )}
        </Card>

        {/* Dropdown Options */}
        {fieldType === 'dropdown' && (
          <Form.Item label="Options">
            <Form.List name="options">
              {(fields, { add, remove }) => (
                <>
                  {fields.map(({ key, name, ...restField }) => (
                    <Space key={key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                      <Form.Item
                        {...restField}
                        name={name}
                        rules={[{ required: true, message: 'Option is required' }]}
                      >
                        <Input placeholder="Option value" />
                      </Form.Item>
                      <Button onClick={() => remove(name)} icon={<DeleteOutlined />} />
                    </Space>
                  ))}
                  <Button type="dashed" onClick={() => add()} block icon={<PlusOutlined />}>
                    Add Option
                  </Button>
                </>
              )}
            </Form.List>
          </Form.Item>
        )}

        <Form.Item label="Default Value" name="defaultValue">
          {fieldType === 'checkbox' ? (
            <Switch />
          ) : fieldType === 'number' ? (
            <InputNumber style={{ width: '100%' }} />
          ) : (
            <Input placeholder="Default value" />
          )}
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default SchemaDesigner;