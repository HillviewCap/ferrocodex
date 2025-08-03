import React, { useEffect, useState, useCallback } from 'react';
import { Form, Button, Card, message, Spin, Row, Col, Divider } from 'antd';
import { SaveOutlined, ReloadOutlined } from '@ant-design/icons';
import FormFieldComponent, { FormField } from './FormFieldComponent';
import { ConditionalLogicProvider, useConditionalLogic, ConditionalRule } from './ConditionalLogicEngine';
import { invoke } from '@tauri-apps/api/core';

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
const DynamicMetadataFormInner: React.FC<DynamicMetadataFormProps> = ({
  schemaId,
  assetId,
  initialValues = {},
  onSubmit,
  onCancel,
  readonly = false,
  showTitle = true
}) => {
  const conditionalLogic = useConditionalLogic();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [schema, setSchema] = useState<AssetMetadataSchema | null>(null);
  const [formFields, setFormFields] = useState<FormField[]>([]);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [isDirty, setIsDirty] = useState(false);

  // Load metadata schema and convert to form fields
  const loadSchema = useCallback(async () => {
    try {
      setLoading(true);
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('No authentication token');
      }

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

      // Set initial form values
      const formValues = { ...initialValues };
      fields.forEach(field => {
        if (formValues[field.name] === undefined && field.defaultValue !== undefined) {
          formValues[field.name] = field.defaultValue;
        }
      });
      
      form.setFieldsValue(formValues);
      setIsDirty(false);
    } catch (error) {
      console.error('Failed to load metadata schema:', error);
      message.error('Failed to load form schema');
    } finally {
      setLoading(false);
    }
  }, [schemaId, initialValues, form]);

  useEffect(() => {
    loadSchema();
  }, [loadSchema]);

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

  // Format field names for display
  const formatFieldName = (name: string): string => {
    return name
      .replace(/([A-Z])/g, ' $1')
      .replace(/_/g, ' ')
      .replace(/^./, str => str.toUpperCase())
      .trim();
  };

  // Handle form value changes
  const handleFieldChange = useCallback(async (changedFields: any, allFields: any) => {
    setIsDirty(true);
    
    // Get current form values
    const formValues = form.getFieldsValue();
    
    // Evaluate conditional logic rules
    conditionalLogic.evaluateRules(formValues);
    
    // Clear field-specific errors when user starts typing
    const newErrors = { ...fieldErrors };
    changedFields.forEach((field: any) => {
      if (newErrors[field.name]) {
        delete newErrors[field.name];
      }
    });
    setFieldErrors(newErrors);

    // Real-time validation
    await validateForm(formValues);
  }, [fieldErrors, form, conditionalLogic]);

  // Validate form against JSON Schema
  const validateForm = async (formValues?: Record<string, any>) => {
    if (!schema) return true;

    try {
      const values = formValues || form.getFieldsValue();
      const token = localStorage.getItem('auth_token');
      
      if (!token) return false;

      await invoke('validate_metadata_values', {
        token,
        schemaId: schema.id,
        valuesJson: JSON.stringify(values)
      });

      setFieldErrors({});
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
      } else {
        message.error('Form validation failed');
      }
      
      setFieldErrors(errors);
      return false;
    }
  };

  // Handle form submission
  const handleSubmit = async (values: Record<string, any>) => {
    if (!schema) return;

    setSubmitting(true);
    try {
      // Validate before submitting
      const isValid = await validateForm(values);
      if (!isValid) {
        message.error('Please fix form errors before submitting');
        return;
      }

      await onSubmit(values);
      setIsDirty(false);
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
          <Button onClick={loadSchema} style={{ marginTop: 16 }}>
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
        onFieldsChange={handleFieldChange}
        disabled={readonly}
      >
        <Row gutter={[16, 16]}>
          {formFields.map((field, index) => {
            // Check conditional logic for field visibility and state
            const isVisible = conditionalLogic.isFieldVisible(field.name);
            const isDisabled = readonly || conditionalLogic.isFieldDisabled(field.name);
            const isRequired = field.required || conditionalLogic.isFieldRequired(field.name);
            
            if (!isVisible) return null;
            
            const handleFieldValueChange = (value: any) => {
              form.setFieldValue(field.name, value);
              const formValues = form.getFieldsValue();
              conditionalLogic.evaluateRules(formValues);
              setIsDirty(true);
            };

            return (
              <Col key={field.name} xs={24} sm={12} md={field.type === 'textarea' ? 24 : 12}>
                <FormFieldComponent
                  field={{ ...field, required: isRequired }}
                  value={form.getFieldValue(field.name)}
                  onChange={handleFieldValueChange}
                  error={fieldErrors[field.name]}
                  disabled={isDisabled}
                />
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
};

// Main component with conditional logic provider
const DynamicMetadataForm: React.FC<DynamicMetadataFormProps> = (props) => {
  return (
    <ConditionalLogicProvider>
      <DynamicMetadataFormInner {...props} />
    </ConditionalLogicProvider>
  );
};

export default DynamicMetadataForm;