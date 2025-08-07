import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  Space,
  Button,
  Row,
  Col,
  Select,
  Input,
  InputNumber,
  DatePicker,
  Checkbox,
  Tag,
  Collapse,
  Tooltip,
  Popover,
  Switch,
  Divider,
  notification,
} from 'antd';
import {
  PlusOutlined,
  DeleteOutlined,
  FilterOutlined,
  ClearOutlined,
  SaveOutlined,
  QuestionCircleOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import dayjs from 'dayjs';
import useSearchStore from '../../store/search';
import { searchUtils } from '../../store/search';
import { 
  MetadataFilter, 
  FilterOperator, 
  LogicOperator, 
  FilterableField 
} from '../../types/search';

const { Option } = Select;
const { Panel } = Collapse;
const { RangePicker } = DatePicker;

interface FilterPanelProps {
  className?: string;
  showPresetActions?: boolean;
  compact?: boolean;
}

const FilterPanel: React.FC<FilterPanelProps> = ({
  className,
  showPresetActions = true,
  compact = false,
}) => {
  const {
    // State
    query,
    availableFields,
    presets,
    
    // Actions
    addFilter,
    removeFilter,
    updateFilter,
    clearFilters,
    loadAvailableFields,
    loadPresets,
    savePreset,
    applyPreset,
  } = useSearchStore();

  const [showLogicBuilder, setShowLogicBuilder] = useState(false);
  const [newPresetName, setNewPresetName] = useState('');
  const [showSavePreset, setShowSavePreset] = useState(false);

  // Load available fields and presets on mount
  useEffect(() => {
    loadAvailableFields();
    loadPresets();
  }, [loadAvailableFields, loadPresets]);

  const getOperatorOptions = useCallback((fieldType: string): FilterOperator[] => {
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
  }, []);

  const getOperatorLabel = useCallback((operator: FilterOperator): string => {
    const labels: Record<FilterOperator, string> = {
      'Equals': 'equals',
      'NotEquals': 'does not equal',
      'Contains': 'contains',
      'StartsWith': 'starts with',
      'EndsWith': 'ends with',
      'GreaterThan': 'greater than',
      'LessThan': 'less than',
      'GreaterThanOrEqual': 'greater than or equal',
      'LessThanOrEqual': 'less than or equal',
      'IsNull': 'is empty',
      'IsNotNull': 'is not empty',
      'InRange': 'in range',
      'Regex': 'matches pattern',
    };
    return labels[operator] || operator;
  }, []);

  const renderValueInput = useCallback((filter: MetadataFilter, index: number) => {
    const field = availableFields.find(f => f.field_name === filter.field_name);
    const fieldType = field?.field_type || filter.field_type;

    // No value input needed for null checks
    if (filter.operator === 'IsNull' || filter.operator === 'IsNotNull') {
      return null;
    }

    // Range input for InRange operator
    if (filter.operator === 'InRange') {
      if (fieldType === 'number') {
        return (
          <Space.Compact>
            <InputNumber
              placeholder="Min"
              value={Array.isArray(filter.value) ? filter.value[0] : undefined}
              onChange={(min) => {
                const max = Array.isArray(filter.value) ? filter.value[1] : undefined;
                updateFilter(index, { ...filter, value: [min, max] });
              }}
            />
            <InputNumber
              placeholder="Max"
              value={Array.isArray(filter.value) ? filter.value[1] : undefined}
              onChange={(max) => {
                const min = Array.isArray(filter.value) ? filter.value[0] : undefined;
                updateFilter(index, { ...filter, value: [min, max] });
              }}
            />
          </Space.Compact>
        );
      } else if (fieldType === 'date') {
        return (
          <RangePicker
            value={Array.isArray(filter.value) ? [
              filter.value[0] ? dayjs(filter.value[0]) : null,
              filter.value[1] ? dayjs(filter.value[1]) : null,
            ] : [null, null]}
            onChange={(dates) => {
              const value = dates ? [
                dates[0]?.toISOString(),
                dates[1]?.toISOString(),
              ] : [null, null];
              updateFilter(index, { ...filter, value });
            }}
          />
        );
      }
    }

    // Field type specific inputs
    switch (fieldType) {
      case 'number':
        return (
          <InputNumber
            placeholder="Enter number"
            value={filter.value}
            onChange={(value) => updateFilter(index, { ...filter, value })}
            style={{ width: '100%' }}
          />
        );

      case 'date':
        return (
          <DatePicker
            value={filter.value ? dayjs(filter.value) : null}
            onChange={(date) => updateFilter(index, { ...filter, value: date?.toISOString() })}
            style={{ width: '100%' }}
          />
        );

      case 'checkbox':
        return (
          <Select
            value={filter.value}
            onChange={(value) => updateFilter(index, { ...filter, value })}
            style={{ width: '100%' }}
          >
            <Option value={true}>True</Option>
            <Option value={false}>False</Option>
          </Select>
        );

      case 'dropdown':
        const fieldSamples = field?.sample_values || [];
        return (
          <Select
            value={filter.value}
            onChange={(value) => updateFilter(index, { ...filter, value })}
            style={{ width: '100%' }}
            showSearch
            allowClear
            placeholder="Select value"
          >
            {fieldSamples.map((sample) => (
              <Option key={sample} value={sample}>
                {sample}
              </Option>
            ))}
          </Select>
        );

      default:
        return (
          <Input
            placeholder="Enter value"
            value={filter.value}
            onChange={(e) => updateFilter(index, { ...filter, value: e.target.value })}
          />
        );
    }
  }, [availableFields, updateFilter]);

  const handleAddFilter = useCallback(() => {
    if (availableFields.length === 0) {
      notification.warning({
        message: 'No Fields Available',
        description: 'No filterable fields are available. Please ensure you have assets with metadata.',
      });
      return;
    }

    const firstField = availableFields[0];
    const newFilter = searchUtils.createDefaultFilter(firstField.field_name, firstField.field_type);
    addFilter(newFilter);
  }, [availableFields, addFilter]);

  const handleSavePreset = useCallback(async () => {
    if (!newPresetName.trim()) {
      notification.warning({
        message: 'Preset Name Required',
        description: 'Please enter a name for the preset.',
      });
      return;
    }

    if (query.filters.length === 0) {
      notification.warning({
        message: 'No Filters to Save',
        description: 'Please add some filters before saving a preset.',
      });
      return;
    }

    try {
      await savePreset({
        name: newPresetName,
        description: `Filter preset with ${query.filters.length} filters`,
      });
      
      notification.success({
        message: 'Preset Saved',
        description: `Filter preset "${newPresetName}" has been saved.`,
      });
      
      setNewPresetName('');
      setShowSavePreset(false);
    } catch (error) {
      console.error('Failed to save preset:', error);
      notification.error({
        message: 'Save Failed',
        description: error instanceof Error ? error.message : 'Failed to save preset',
      });
    }
  }, [newPresetName, query.filters, savePreset]);

  const filterCount = query.filters.length;
  const hasFilters = filterCount > 0;

  const savePresetContent = (
    <div style={{ width: 250 }}>
      <Space direction="vertical" style={{ width: '100%' }}>
        <Input
          placeholder="Preset name"
          value={newPresetName}
          onChange={(e) => setNewPresetName(e.target.value)}
          onPressEnter={handleSavePreset}
        />
        <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
          <Button size="small" onClick={() => setShowSavePreset(false)}>
            Cancel
          </Button>
          <Button type="primary" size="small" onClick={handleSavePreset}>
            Save
          </Button>
        </Space>
      </Space>
    </div>
  );

  if (compact) {
    return (
      <div className={className}>
        <Space wrap>
          {hasFilters && (
            <Tag color="blue">
              {filterCount} filter{filterCount !== 1 ? 's' : ''}
            </Tag>
          )}
          <Button icon={<PlusOutlined />} onClick={handleAddFilter} size="small">
            Add Filter
          </Button>
          {hasFilters && (
            <Button icon={<ClearOutlined />} onClick={clearFilters} size="small">
              Clear
            </Button>
          )}
        </Space>
      </div>
    );
  }

  return (
    <div className={className}>
      <Card
        title={
          <Space>
            <FilterOutlined />
            <span>Advanced Filters</span>
            {hasFilters && (
              <Tag color="blue">
                {filterCount} active
              </Tag>
            )}
          </Space>
        }
        extra={
          <Space>
            {showPresetActions && (
              <>
                <Popover
                  content={savePresetContent}
                  title="Save Filter Preset"
                  trigger="click"
                  open={showSavePreset}
                  onOpenChange={setShowSavePreset}
                >
                  <Button icon={<SaveOutlined />} disabled={!hasFilters}>
                    Save
                  </Button>
                </Popover>
                <Select
                  placeholder="Load preset"
                  style={{ minWidth: 150 }}
                  onChange={(presetId) => {
                    const preset = presets.find(p => p.id === presetId);
                    if (preset) {
                      applyPreset(preset);
                    }
                  }}
                  allowClear
                >
                  {presets.map((preset) => (
                    <Option key={preset.id} value={preset.id}>
                      {preset.name}
                    </Option>
                  ))}
                </Select>
              </>
            )}
            <Tooltip title="Show logic builder">
              <Switch
                checked={showLogicBuilder}
                onChange={setShowLogicBuilder}
                checkedChildren={<SettingOutlined />}
                unCheckedChildren={<SettingOutlined />}
              />
            </Tooltip>
            <Button
              icon={<ClearOutlined />}
              onClick={clearFilters}
              disabled={!hasFilters}
            >
              Clear All
            </Button>
          </Space>
        }
        size="small"
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          {/* Filter List */}
          {query.filters.map((filter, index) => (
            <Card key={index} size="small" style={{ marginBottom: 8 }}>
              <Row gutter={[8, 8]} align="middle">
                {/* Logic Operator (except for first filter) */}
                {index > 0 && (
                  <Col span={3}>
                    <Select
                      value={filter.logic_operator}
                      onChange={(value: LogicOperator) => 
                        updateFilter(index, { ...filter, logic_operator: value })
                      }
                      style={{ width: '100%' }}
                    >
                      <Option value="And">AND</Option>
                      <Option value="Or">OR</Option>
                      <Option value="Not">NOT</Option>
                    </Select>
                  </Col>
                )}

                {/* Field Name */}
                <Col span={index === 0 ? 6 : 5}>
                  <Select
                    value={filter.field_name}
                    onChange={(fieldName) => {
                      const field = availableFields.find(f => f.field_name === fieldName);
                      if (field) {
                        const operators = getOperatorOptions(field.field_type);
                        updateFilter(index, {
                          ...filter,
                          field_name: fieldName,
                          field_type: field.field_type,
                          operator: operators.includes(filter.operator) ? filter.operator : operators[0],
                          value: '',
                        });
                      }
                    }}
                    style={{ width: '100%' }}
                    showSearch
                    placeholder="Select field"
                  >
                    {availableFields.map((field) => (
                      <Option key={field.field_name} value={field.field_name}>
                        <div>
                          <div>{field.field_name}</div>
                          <small style={{ color: '#666' }}>
                            {field.schema_name} • {field.field_type}
                          </small>
                        </div>
                      </Option>
                    ))}
                  </Select>
                </Col>

                {/* Operator */}
                <Col span={4}>
                  <Select
                    value={filter.operator}
                    onChange={(operator: FilterOperator) => 
                      updateFilter(index, { ...filter, operator, value: '' })
                    }
                    style={{ width: '100%' }}
                  >
                    {getOperatorOptions(filter.field_type).map((op) => (
                      <Option key={op} value={op}>
                        {getOperatorLabel(op)}
                      </Option>
                    ))}
                  </Select>
                </Col>

                {/* Value */}
                <Col span={8}>
                  {renderValueInput(filter, index)}
                </Col>

                {/* Actions */}
                <Col span={2} style={{ textAlign: 'right' }}>
                  <Tooltip title="Remove filter">
                    <Button
                      icon={<DeleteOutlined />}
                      onClick={() => removeFilter(index)}
                      size="small"
                      danger
                    />
                  </Tooltip>
                </Col>
              </Row>
            </Card>
          ))}

          {/* Add Filter Button */}
          <Button
            type="dashed"
            icon={<PlusOutlined />}
            onClick={handleAddFilter}
            style={{ width: '100%' }}
            disabled={availableFields.length === 0}
          >
            Add Filter
          </Button>

          {/* Query Description */}
          {hasFilters && showLogicBuilder && (
            <>
              <Divider />
              <Card size="small">
                <div style={{ fontSize: '12px', color: '#666' }}>
                  <strong>Query:</strong> {searchUtils.buildQueryDescription(query)}
                </div>
              </Card>
            </>
          )}

          {/* Available Fields Info */}
          {availableFields.length === 0 && (
            <div style={{ textAlign: 'center', color: '#666' }}>
              <QuestionCircleOutlined style={{ marginRight: 8 }} />
              No filterable fields available. Create assets with metadata to enable filtering.
            </div>
          )}
        </Space>
      </Card>
    </div>
  );
};

export default FilterPanel;