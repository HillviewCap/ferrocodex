import React, { useState, useEffect } from 'react';
import {
  Form,
  Select,
  Input,
  InputNumber,
  DatePicker,
  Button,
  Space,
  Card,
  Divider,
  Typography,
  Tag,
  Modal,
  message,
  Tooltip,
  Collapse,
  Radio,
  Switch,
} from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  FilterOutlined,
  SaveOutlined,
  LoadingOutlined,
  QuestionCircleOutlined,
} from '@ant-design/icons';
import dayjs from 'dayjs';
import {
  FilterBuilderProps,
  MetadataFilter,
  FilterOperator,
  LogicOperator,
  FilterPreset,
  FilterableField,
} from '../../types/search';
import { useSearch } from '../../contexts/SearchContext';

const { Option } = Select;
const { Text, Title } = Typography;
const { Panel } = Collapse;
const { TextArea } = Input;

const MetadataFilterBuilder: React.FC<FilterBuilderProps> = ({
  filters,
  availableFields,
  onFiltersChange,
  presets = [],
  onPresetSave,
  onPresetLoad,
  className = '',
}) => {
  const { filterState, addFilter, removeFilter, updateFilter, clearFilters } = useSearch();
  const [form] = Form.useForm();
  const [presetModalVisible, setPresetModalVisible] = useState(false);
  const [presetForm] = Form.useForm();

  // Filter operators based on field type
  const getOperatorsForFieldType = (fieldType: string): FilterOperator[] => {
    switch (fieldType.toLowerCase()) {
      case 'text':
      case 'textarea':
        return ['Equals', 'NotEquals', 'Contains', 'StartsWith', 'EndsWith', 'IsNull', 'IsNotNull', 'Regex'];
      case 'number':
        return ['Equals', 'NotEquals', 'GreaterThan', 'LessThan', 'GreaterThanOrEqual', 'LessThanOrEqual', 'InRange', 'IsNull', 'IsNotNull'];
      case 'date':
        return ['Equals', 'NotEquals', 'GreaterThan', 'LessThan', 'GreaterThanOrEqual', 'LessThanOrEqual', 'InRange', 'IsNull', 'IsNotNull'];
      case 'dropdown':
        return ['Equals', 'NotEquals', 'IsNull', 'IsNotNull'];
      case 'checkbox':
        return ['Equals', 'NotEquals'];
      default:
        return ['Equals', 'NotEquals', 'Contains', 'IsNull', 'IsNotNull'];
    }
  };

  // Get human-readable operator labels
  const getOperatorLabel = (operator: FilterOperator): string => {
    const labels: Record<FilterOperator, string> = {
      Equals: 'equals',
      NotEquals: 'does not equal',
      Contains: 'contains',
      StartsWith: 'starts with',
      EndsWith: 'ends with',
      GreaterThan: 'is greater than',
      LessThan: 'is less than',
      GreaterThanOrEqual: 'is greater than or equal to',
      LessThanOrEqual: 'is less than or equal to',
      IsNull: 'is empty',
      IsNotNull: 'is not empty',
      InRange: 'is in range',
      Regex: 'matches pattern',
    };
    return labels[operator] || operator;
  };

  // Add a new filter
  const handleAddFilter = () => {
    if (availableFields.length === 0) {
      message.warning('No filterable fields available');
      return;
    }

    const firstField = availableFields[0];
    const availableOperators = getOperatorsForFieldType(firstField.field_type);
    
    const newFilter: MetadataFilter = {
      field_name: firstField.field_name,
      field_type: firstField.field_type,
      operator: availableOperators[0],
      value: '',
      logic_operator: filters.length > 0 ? 'And' : 'And',
    };

    addFilter(newFilter);
  };

  // Remove a filter
  const handleRemoveFilter = (index: number) => {
    removeFilter(index);
  };

  // Update a filter
  const handleFilterChange = (index: number, field: keyof MetadataFilter, value: any) => {
    const updatedFilter = { ...filters[index], [field]: value };
    
    // If field type changed, reset operator and value
    if (field === 'field_name') {
      const fieldInfo = availableFields.find(f => f.field_name === value);
      if (fieldInfo) {
        const availableOperators = getOperatorsForFieldType(fieldInfo.field_type);
        updatedFilter.field_type = fieldInfo.field_type;
        updatedFilter.operator = availableOperators[0];
        updatedFilter.value = '';
      }
    }

    // If operator changed to IsNull or IsNotNull, clear value
    if (field === 'operator' && (value === 'IsNull' || value === 'IsNotNull')) {
      updatedFilter.value = null;
    }

    updateFilter(index, updatedFilter);
  };

  // Render value input based on field type and operator
  const renderValueInput = (filter: MetadataFilter, index: number) => {
    const fieldInfo = availableFields.find(f => f.field_name === filter.field_name);
    
    // No input needed for IsNull/IsNotNull operators
    if (filter.operator === 'IsNull' || filter.operator === 'IsNotNull') {
      return <Text type="secondary" style={{ fontStyle: 'italic' }}>No value needed</Text>;
    }

    // Range input for InRange operator
    if (filter.operator === 'InRange') {
      return (
        <Input.Group compact style={{ display: 'flex' }}>
          <InputNumber
            placeholder="Min"
            style={{ flex: 1 }}
            value={filter.value?.min}
            onChange={(value) => handleFilterChange(index, 'value', { ...filter.value, min: value })}
          />
          <InputNumber
            placeholder="Max"
            style={{ flex: 1, marginLeft: '8px' }}
            value={filter.value?.max}
            onChange={(value) => handleFilterChange(index, 'value', { ...filter.value, max: value })}
          />
        </Input.Group>
      );
    }

    // Input based on field type
    switch (filter.field_type.toLowerCase()) {
      case 'number':
        return (
          <InputNumber
            placeholder="Enter number"
            style={{ width: '100%' }}
            value={filter.value}
            onChange={(value) => handleFilterChange(index, 'value', value)}
          />
        );

      case 'date':
        return (
          <DatePicker
            placeholder="Select date"
            style={{ width: '100%' }}
            value={filter.value ? dayjs(filter.value) : null}
            onChange={(date) => handleFilterChange(index, 'value', date?.toISOString())}
          />
        );

      case 'dropdown':
        return (
          <Select
            placeholder="Select value"
            style={{ width: '100%' }}
            value={filter.value}
            onChange={(value) => handleFilterChange(index, 'value', value)}
            showSearch
            allowClear
          >
            {fieldInfo?.sample_values.map(value => (
              <Option key={value} value={value}>{value}</Option>
            ))}
          </Select>
        );

      case 'checkbox':
        return (
          <Radio.Group
            value={filter.value}
            onChange={(e) => handleFilterChange(index, 'value', e.target.value)}
          >
            <Radio value={true}>True</Radio>
            <Radio value={false}>False</Radio>
          </Radio.Group>
        );

      case 'textarea':
        return (
          <TextArea
            placeholder="Enter text"
            rows={2}
            value={filter.value}
            onChange={(e) => handleFilterChange(index, 'value', e.target.value)}
          />
        );

      default:
        return (
          <Input
            placeholder="Enter value"
            value={filter.value}
            onChange={(e) => handleFilterChange(index, 'value', e.target.value)}
          />
        );
    }
  };

  // Render individual filter
  const renderFilter = (filter: MetadataFilter, index: number) => {
    const fieldInfo = availableFields.find(f => f.field_name === filter.field_name);
    const availableOperators = getOperatorsForFieldType(filter.field_type);

    return (
      <Card
        key={index}
        size="small"
        style={{ marginBottom: '12px' }}
        extra={
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            onClick={() => handleRemoveFilter(index)}
            size="small"
          />
        }
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          {/* Logic operator (except for first filter) */}
          {index > 0 && (
            <div>
              <Text strong style={{ fontSize: '12px', color: '#666', marginRight: '8px' }}>
                Logic:
              </Text>
              <Radio.Group
                size="small"
                value={filter.logic_operator}
                onChange={(e) => handleFilterChange(index, 'logic_operator', e.target.value)}
              >
                <Radio.Button value="And">AND</Radio.Button>
                <Radio.Button value="Or">OR</Radio.Button>
                <Radio.Button value="Not">NOT</Radio.Button>
              </Radio.Group>
            </div>
          )}

          {/* Field selection */}
          <div>
            <Text strong style={{ fontSize: '12px', color: '#666', marginRight: '8px' }}>
              Field:
            </Text>
            <Select
              placeholder="Select field"
              style={{ width: '200px' }}
              value={filter.field_name}
              onChange={(value) => handleFilterChange(index, 'field_name', value)}
              showSearch
              filterOption={(input, option) =>
                option?.children.toLowerCase().includes(input.toLowerCase())
              }
            >
              {availableFields.map(field => (
                <Option key={field.field_name} value={field.field_name}>
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <span>{field.field_name}</span>
                    <span style={{ fontSize: '11px', color: '#999' }}>
                      {field.schema_name}
                    </span>
                  </div>
                </Option>
              ))}
            </Select>
          </div>

          {/* Operator selection */}
          <div>
            <Text strong style={{ fontSize: '12px', color: '#666', marginRight: '8px' }}>
              Operator:
            </Text>
            <Select
              placeholder="Select operator"
              style={{ width: '200px' }}
              value={filter.operator}
              onChange={(value) => handleFilterChange(index, 'operator', value)}
            >
              {availableOperators.map(operator => (
                <Option key={operator} value={operator}>
                  {getOperatorLabel(operator)}
                </Option>
              ))}
            </Select>
          </div>

          {/* Value input */}
          <div>
            <Text strong style={{ fontSize: '12px', color: '#666', marginRight: '8px' }}>
              Value:
            </Text>
            <div style={{ marginTop: '4px' }}>
              {renderValueInput(filter, index)}
            </div>
          </div>

          {/* Field info */}
          {fieldInfo && (
            <div style={{ fontSize: '11px', color: '#999', marginTop: '4px' }}>
              <Text type="secondary">
                Type: {fieldInfo.field_type} • Schema: {fieldInfo.schema_name} • 
                Used in {fieldInfo.usage_count} asset{fieldInfo.usage_count !== 1 ? 's' : ''}
              </Text>
            </div>
          )}
        </Space>
      </Card>
    );
  };

  // Save filter preset
  const handleSavePreset = async () => {
    if (filters.length === 0) {
      message.warning('No filters to save');
      return;
    }

    setPresetModalVisible(true);
  };

  // Handle preset save form submission
  const handlePresetSaveSubmit = async (values: { name: string; description: string }) => {
    try {
      if (onPresetSave) {
        await onPresetSave({
          name: values.name,
          description: values.description,
          filters: filters,
        });
      }
      
      setPresetModalVisible(false);
      presetForm.resetFields();
      message.success('Filter preset saved successfully');
    } catch (error) {
      message.error('Failed to save filter preset');
    }
  };

  // Load filter preset
  const handleLoadPreset = (preset: FilterPreset) => {
    clearFilters();
    preset.filters.forEach(filter => addFilter(filter));
    
    if (onPresetLoad) {
      onPresetLoad(preset);
    }
    
    message.success(`Loaded preset: ${preset.name}`);
  };

  return (
    <div className={`metadata-filter-builder ${className}`}>
      <Card title={
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <FilterOutlined />
            <span>Advanced Filters</span>
            {filters.length > 0 && (
              <Tag color="blue">{filters.length} filter{filters.length !== 1 ? 's' : ''}</Tag>
            )}
          </div>
          <Space>
            <Button
              size="small"
              icon={<SaveOutlined />}
              onClick={handleSavePreset}
              disabled={filters.length === 0}
            >
              Save Preset
            </Button>
            <Button
              size="small"
              danger
              onClick={clearFilters}
              disabled={filters.length === 0}
            >
              Clear All
            </Button>
          </Space>
        </div>
      }>
        {/* Filter presets */}
        {presets.length > 0 && (
          <div style={{ marginBottom: '16px' }}>
            <Title level={5} style={{ marginBottom: '8px', fontSize: '14px' }}>
              Filter Presets
            </Title>
            <Space wrap>
              {presets.map(preset => (
                <Tooltip
                  key={preset.id}
                  title={
                    <div>
                      <div><strong>{preset.name}</strong></div>
                      <div style={{ fontSize: '12px', marginTop: '4px' }}>
                        {preset.description}
                      </div>
                      <div style={{ fontSize: '11px', marginTop: '4px', color: '#ccc' }}>
                        {preset.filters.length} filter{preset.filters.length !== 1 ? 's' : ''} • 
                        Used {preset.usage_count} time{preset.usage_count !== 1 ? 's' : ''}
                      </div>
                    </div>
                  }
                >
                  <Tag
                    style={{ cursor: 'pointer', padding: '4px 8px' }}
                    onClick={() => handleLoadPreset(preset)}
                  >
                    {preset.name} ({preset.filters.length})
                  </Tag>
                </Tooltip>
              ))}
            </Space>
            <Divider style={{ margin: '12px 0' }} />
          </div>
        )}

        {/* Current filters */}
        <div>
          {filters.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 20px', color: '#999' }}>
              <FilterOutlined style={{ fontSize: '24px', marginBottom: '8px' }} />
              <div>No filters applied</div>
              <div style={{ fontSize: '12px', marginTop: '4px' }}>
                Add filters to refine your search results
              </div>
            </div>
          ) : (
            <div>
              {filters.map((filter, index) => renderFilter(filter, index))}
            </div>
          )}
        </div>

        {/* Add filter button */}
        <div style={{ marginTop: '16px' }}>
          <Button
            type="dashed"
            icon={<PlusOutlined />}
            onClick={handleAddFilter}
            style={{ width: '100%' }}
            disabled={availableFields.length === 0}
          >
            {availableFields.length === 0 ? 'No filterable fields available' : 'Add Filter'}
          </Button>
        </div>

        {/* Help text */}
        <div style={{ marginTop: '16px', fontSize: '12px', color: '#999' }}>
          <QuestionCircleOutlined style={{ marginRight: '4px' }} />
          Filters are applied in order with the specified logic operators (AND, OR, NOT).
        </div>
      </Card>

      {/* Save preset modal */}
      <Modal
        title="Save Filter Preset"
        open={presetModalVisible}
        onCancel={() => {
          setPresetModalVisible(false);
          presetForm.resetFields();
        }}
        footer={null}
      >
        <Form
          form={presetForm}
          layout="vertical"
          onFinish={handlePresetSaveSubmit}
        >
          <Form.Item
            name="name"
            label="Preset Name"
            rules={[
              { required: true, message: 'Please enter a preset name' },
              { min: 2, message: 'Name must be at least 2 characters' },
              { max: 50, message: 'Name cannot exceed 50 characters' },
            ]}
          >
            <Input placeholder="Enter a descriptive name" />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description"
            rules={[
              { required: true, message: 'Please enter a description' },
              { max: 200, message: 'Description cannot exceed 200 characters' },
            ]}
          >
            <TextArea
              rows={3}
              placeholder="Describe what this filter preset is used for"
            />
          </Form.Item>

          <div style={{ marginBottom: '16px' }}>
            <Text strong>Filters to save ({filters.length}):</Text>
            <div style={{ marginTop: '8px', fontSize: '12px', color: '#666' }}>
              {filters.map((filter, index) => (
                <div key={index} style={{ marginBottom: '4px' }}>
                  {index > 0 && `${filter.logic_operator.toUpperCase()} `}
                  <Text code>{filter.field_name}</Text> {getOperatorLabel(filter.operator)} 
                  {filter.value && ` "${filter.value}"`}
                </div>
              ))}
            </div>
          </div>

          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button
                onClick={() => {
                  setPresetModalVisible(false);
                  presetForm.resetFields();
                }}
              >
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Save Preset
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default MetadataFilterBuilder;