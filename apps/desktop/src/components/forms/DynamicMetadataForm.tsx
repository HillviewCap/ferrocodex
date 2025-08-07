import React, { useEffect, useState, useCallback } from 'react';
import { 
  Form, 
  Button, 
  Card, 
  message, 
  Spin, 
  Row, 
  Col, 
  Divider,
  Input,
  InputNumber,
  DatePicker,
  Select,
  Checkbox,
  Tooltip
} from 'antd';
import { SaveOutlined, ReloadOutlined, InfoCircleOutlined } from '@ant-design/icons';
import { ConditionalLogicProvider, useConditionalLogic, ConditionalRule } from './ConditionalLogicEngine';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import dayjs from 'dayjs';

const { TextArea } = Input;
const { Option } = Select;

interface DynamicMetadataFormProps {
  schemaId: number;
  assetId?: number;
  initialValues?: Record<string, any>;
  onSubmit: (values: Record<string, any>) => Promise<void>;
  onCancel?: () => void;
  readonly?: boolean;
  showTitle?: boolean;
}

interface JSONSchemaProperty {
  type: string;
  title?: string;
  description?: string;
  enum?: any[];
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  format?: string;
  default?: any;
}

interface FormField {
  name: string;
  type: 'text' | 'number' | 'date' | 'dropdown' | 'checkbox' | 'textarea';
  label: string;
  description?: string;
  required?: boolean;
  placeholder?: string;
  validation?: {
    minLength?: number;
    maxLength?: number;
    min?: number;
    max?: number;
    pattern?: string;
    enum?: string[];
  };
  options?: Array<{ label: string; value: string | number }>;
  defaultValue?: any;
}

interface JSONSchema {
  type: 'object';
  title?: string;
  properties: Record<string, JSONSchemaProperty>;
  required?: string[];
  fieldOrder?: string[];
  conditionalLogic?: ConditionalRule[];
}

interface AssetMetadataSchema {
  id: number;
  name: string;
  description: string;
  schema_json: string;
  asset_type_filter?: string;
  version: number;
}

// Inner form component that uses conditional logic
const DynamicMetadataFormInner: React.FC<DynamicMetadataFormProps> = React.memo(({
  schemaId,
  assetId,
  initialValues = {},
  onSubmit,
  onCancel,
  readonly = false,
  showTitle = true
}) => {
  const conditionalLogic = useConditionalLogic();
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [schema, setSchema] = useState<AssetMetadataSchema | null>(null);
  const [formFields, setFormFields] = useState<FormField[]>([]);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [isDirty, setIsDirty] = useState(false);

  // Load metadata schema and convert to form fields
  useEffect(() => {
    if (schemaId && token) {
      // Directly call the loading logic here to avoid dependency issues
      (async () => {
        try {
          setLoading(true);
          
          const schemaData = await invoke<AssetMetadataSchema | null>('get_metadata_schema_by_id', {
            token,
            schemaId
          });

          if (!schemaData) {
            throw new Error('Schema not found');
          }

          setSchema(schemaData);
          
          // Parse JSON Schema and convert to form fields
          const jsonSchema: JSONSchema = JSON.parse(schemaData.schema_json);
          const fields = convertSchemaToFields(jsonSchema);
          setFormFields(fields);

          // Set up conditional logic rules if present
          if (jsonSchema.conditionalLogic) {
            conditionalLogic.clearRules();
            conditionalLogic.addRules(jsonSchema.conditionalLogic);
          }

          setIsDirty(false);
        } catch (error) {
          console.error('Failed to load metadata schema:', error);
          message.error('Failed to load form schema');
        } finally {
          setLoading(false);
        }
      })();
    }
  }, [schemaId, token, conditionalLogic]); // Only depend on schemaId, token, and conditionalLogic

  // Separate effect to handle initial values and form field setting
  useEffect(() => {
    if (formFields.length > 0 && !loading) {
      const formValues: Record<string, any> = {};
      
      // Process initial values and convert dates
      formFields.forEach(field => {
        const value = initialValues[field.name];
        
        if (field.type === 'date' && value) {
          // Convert date strings to dayjs objects for DatePicker
          const dateValue = dayjs.isDayjs(value) ? value : dayjs(value);
          formValues[field.name] = dateValue.isValid() ? dateValue : null;
        } else if (value !== undefined) {
          formValues[field.name] = value;
        } else if (field.defaultValue !== undefined) {
          formValues[field.name] = field.defaultValue;
        }
      });
      
      form.setFieldsValue(formValues);
    }
  }, [formFields, initialValues, loading, form])

  // Convert JSON Schema to FormField array
  const convertSchemaToFields = (jsonSchema: JSONSchema): FormField[] => {
    const { properties, required = [], fieldOrder } = jsonSchema;
    const fields: FormField[] = [];

    const propertyNames = fieldOrder || Object.keys(properties);

    propertyNames.forEach(name => {
      const property = properties[name];
      if (!property) return;

      const field: FormField = {
        name,
        type: mapJsonSchemaTypeToFieldType(property),
        label: property.title || formatFieldName(name),
        description: property.description,
        required: required.includes(name),
        placeholder: `Enter ${property.title || formatFieldName(name)}`,
        defaultValue: property.default,
        validation: {
          minLength: property.minLength,
          maxLength: property.maxLength,
          min: property.minimum,
          max: property.maximum,
          pattern: property.pattern,
          enum: property.enum
        }
      };

      // Handle dropdown options
      if (property.enum) {
        field.options = property.enum.map(value => ({
          label: String(value),
          value
        }));
      }

      fields.push(field);
    });

    return fields;
  };

  // Map JSON Schema types to form field types
  const mapJsonSchemaTypeToFieldType = (property: JSONSchemaProperty): FormField['type'] => {
    if (property.enum) return 'dropdown';
    
    switch (property.type) {
      case 'string':
        if (property.format === 'date' || property.format === 'date-time') return 'date';
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

  // Format field names for display
  const formatFieldName = (name: string): string => {
    return name
      .replace(/([A-Z])/g, ' $1')
      .replace(/_/g, ' ')
      .replace(/^./, str => str.toUpperCase())
      .trim();
  };


  // Validate form against JSON Schema
  const validateForm = async (formValues?: Record<string, any>, specificField?: string) => {
    if (!schema) return true;

    try {
      const values = formValues || form.getFieldsValue();
      
      if (!token) return false;

      await invoke('validate_metadata_values', {
        token,
        schemaId: schema.id,
        valuesJson: JSON.stringify(values)
      });

      // Clear errors if validation passes
      if (specificField) {
        // Only clear the specific field error
        setFieldErrors(prev => {
          const newErrors = { ...prev };
          delete newErrors[specificField];
          return newErrors;
        });
      } else {
        setFieldErrors({});
      }
      return true;
    } catch (error: any) {
      // Parse validation errors and set field-specific errors
      const errors: Record<string, string> = {};
      if (error.includes(':')) {
        const errorLines = error.split(', ');
        errorLines.forEach((line: string) => {
          const [fieldPath, message] = line.split(': ', 2);
          if (fieldPath && message) {
            errors[fieldPath] = message;
          }
        });
      } else if (!specificField) {
        // Only show general error if not validating a specific field
        message.error('Form validation failed');
      }
      
      // If validating a specific field, only update that field's error
      if (specificField) {
        setFieldErrors(prev => ({
          ...prev,
          ...(errors[specificField] ? { [specificField]: errors[specificField] } : {})
        }));
      } else {
        setFieldErrors(errors);
      }
      return false;
    }
  };

  // Handle form submission
  const handleSubmit = async (values: Record<string, any>) => {
    if (!schema) return;

    setSubmitting(true);
    try {
      // Process values, converting dates to strings
      const processedValues: Record<string, any> = {};
      Object.entries(values).forEach(([key, value]) => {
        if (dayjs.isDayjs(value)) {
          processedValues[key] = value.format('YYYY-MM-DD');
        } else {
          processedValues[key] = value;
        }
      });
      
      // Evaluate conditional logic before validation
      conditionalLogic.evaluateRules(processedValues);
      
      // Validate before submitting
      const isValid = await validateForm(processedValues);
      if (!isValid) {
        message.error('Please fix form errors before submitting');
        setSubmitting(false);
        return;
      }

      await onSubmit(processedValues);
      setIsDirty(false);
      setFieldErrors({});
      message.success('Metadata saved successfully');
    } catch (error) {
      console.error('Failed to submit form:', error);
      message.error('Failed to save metadata');
    } finally {
      setSubmitting(false);
    }
  };

  // Handle form reset
  const handleReset = () => {
    form.resetFields();
    setFieldErrors({});
    setIsDirty(false);
  };
  
  // Handle form value changes
  const handleValuesChange = useCallback(() => {
    setIsDirty(true);
  }, []);

  // Render field based on type
  const renderField = (field: FormField) => {
    const commonProps = {
      placeholder: field.placeholder,
      disabled: readonly
    };

    switch (field.type) {
      case 'text':
        return (
          <Input
            {...commonProps}
            maxLength={field.validation?.maxLength}
          />
        );

      case 'textarea':
        return (
          <TextArea
            {...commonProps}
            maxLength={field.validation?.maxLength}
            rows={4}
            showCount
          />
        );

      case 'number':
        return (
          <InputNumber
            {...commonProps}
            min={field.validation?.min}
            max={field.validation?.max}
            style={{ width: '100%' }}
          />
        );

      case 'date':
        return (
          <DatePicker
            {...commonProps}
            style={{ width: '100%' }}
            format="YYYY-MM-DD"
          />
        );

      case 'dropdown':
        return (
          <Select
            {...commonProps}
            style={{ width: '100%' }}
            allowClear
          >
            {field.options?.map((option) => (
              <Option key={option.value} value={option.value}>
                {option.label}
              </Option>
            ))}
          </Select>
        );

      case 'checkbox':
        return (
          <Checkbox disabled={readonly}>
            {field.label}
          </Checkbox>
        );

      default:
        return <Input disabled placeholder="Unsupported field type" />;
    }
  };

  if (loading) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '50px 0' }}>
          <Spin size="large" />
          <div style={{ marginTop: 16 }}>Loading form schema...</div>
        </div>
      </Card>
    );
  }

  if (!schema) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '50px 0' }}>
          <div>Schema not found</div>
          <Button onClick={() => window.location.reload()} style={{ marginTop: 16 }}>
            Retry
          </Button>
        </div>
      </Card>
    );
  }

  return (
    <Card title={showTitle ? schema.name : undefined}>
      {schema.description && (
        <div style={{ marginBottom: 24, color: '#666' }}>
          {schema.description}
        </div>
      )}
      
      <Form
        form={form}
        layout="vertical"
        onFinish={handleSubmit}
        onValuesChange={handleValuesChange}
        disabled={readonly}
      >
        <Row gutter={[16, 16]}>
          {formFields.map((field) => {
            const isCheckbox = field.type === 'checkbox';
            const labelWithTooltip = field.description ? (
              <span>
                {field.label}
                {field.required && <span style={{ color: 'red', marginLeft: 4 }}>*</span>}
                <Tooltip title={field.description}>
                  <InfoCircleOutlined style={{ marginLeft: 8, color: '#1890ff' }} />
                </Tooltip>
              </span>
            ) : (
              <span>
                {field.label}
                {field.required && <span style={{ color: 'red', marginLeft: 4 }}>*</span>}
              </span>
            );

            return (
              <Col key={field.name} xs={24} sm={12} md={field.type === 'textarea' ? 24 : 12}>
                <Form.Item
                  name={field.name}
                  label={!isCheckbox ? labelWithTooltip : undefined}
                  valuePropName={isCheckbox ? 'checked' : 'value'}
                  rules={[
                    {
                      required: field.required,
                      message: `${field.label} is required`
                    },
                    ...(field.validation?.pattern ? [{
                      pattern: new RegExp(field.validation.pattern),
                      message: `${field.label} format is invalid`
                    }] : [])
                  ]}
                  help={fieldErrors[field.name]}
                  validateStatus={fieldErrors[field.name] ? 'error' : undefined}
                >
                  {renderField(field)}
                </Form.Item>
              </Col>
            );
          })}
        </Row>

        {!readonly && (
          <>
            <Divider />
            <div style={{ display: 'flex', gap: 12, justifyContent: 'flex-end' }}>
              <Button onClick={onCancel}>
                Cancel
              </Button>
              <Button onClick={handleReset} icon={<ReloadOutlined />}>
                Reset
              </Button>
              <Button
                type="primary"
                htmlType="submit"
                loading={submitting}
                icon={<SaveOutlined />}
                disabled={!isDirty}
              >
                Save Metadata
              </Button>
            </div>
          </>
        )}
      </Form>
    </Card>
  );
});

// Main component with conditional logic provider
const DynamicMetadataForm: React.FC<DynamicMetadataFormProps> = (props) => {
  return (
    <ConditionalLogicProvider>
      <DynamicMetadataFormInner {...props} />
    </ConditionalLogicProvider>
  );
};

export default DynamicMetadataForm;