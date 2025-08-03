import React from 'react';
import { Input, InputNumber, DatePicker, Select, Checkbox, Form, Tooltip } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';

const { TextArea } = Input;
const { Option } = Select;

export interface FormFieldProps {
  field: FormField;
  value?: any;
  onChange: (value: any) => void;
  onBlur?: () => void;
  error?: string;
  disabled?: boolean;
}

export interface FormField {
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

const FormFieldComponent: React.FC<FormFieldProps> = ({
  field,
  value,
  onChange = () => {},
  onBlur = () => {},
  error,
  disabled = false
}) => {
  const renderField = () => {
    switch (field.type) {
      case 'text':
        return (
          <Input
            placeholder={field.placeholder}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onBlur={onBlur}
            disabled={disabled}
            status={error ? 'error' : undefined}
            maxLength={field.validation?.maxLength}
          />
        );

      case 'textarea':
        return (
          <TextArea
            placeholder={field.placeholder}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onBlur={onBlur}
            disabled={disabled}
            status={error ? 'error' : undefined}
            maxLength={field.validation?.maxLength}
            rows={4}
            showCount
          />
        );

      case 'number':
        return (
          <InputNumber
            placeholder={field.placeholder}
            value={value}
            onChange={onChange}
            onBlur={onBlur}
            disabled={disabled}
            status={error ? 'error' : undefined}
            min={field.validation?.min}
            max={field.validation?.max}
            style={{ width: '100%' }}
          />
        );

      case 'date':
        return (
          <DatePicker
            placeholder={field.placeholder}
            value={value ? dayjs(value) : null}
            onChange={(date) => onChange(date ? date.format('YYYY-MM-DD') : null)}
            onBlur={onBlur}
            disabled={disabled}
            status={error ? 'error' : undefined}
            style={{ width: '100%' }}
          />
        );

      case 'dropdown':
        return (
          <Select
            placeholder={field.placeholder}
            value={value}
            onChange={onChange}
            onBlur={onBlur}
            disabled={disabled}
            status={error ? 'error' : undefined}
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
          <Checkbox
            checked={value}
            onChange={(e) => onChange(e.target.checked)}
            onBlur={onBlur}
            disabled={disabled}
          >
            {field.label}
          </Checkbox>
        );

      default:
        return <Input disabled placeholder="Unsupported field type" />;
    }
  };

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

  if (field.type === 'checkbox') {
    return (
      <Form.Item
        name={field.name}
        valuePropName="checked"
        help={error}
        validateStatus={error ? 'error' : ''}
      >
        {renderField()}
      </Form.Item>
    );
  }

  return (
    <Form.Item
      label={labelWithTooltip}
      name={field.name}
      help={error}
      validateStatus={error ? 'error' : ''}
      required={field.required}
    >
      {renderField()}
    </Form.Item>
  );
};

export default FormFieldComponent;