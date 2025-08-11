import React, { useState, useEffect } from 'react';
import { Typography, Space, Card, Alert, Row, Col, Divider } from 'antd';
import { SecurityScanOutlined, CheckCircleOutlined, ExclamationCircleOutlined, LockOutlined } from '@ant-design/icons';
import { BaseStepProps, SecurityValidationData } from '../../../types/workflow';
import SecurityClassificationSelector from '../../security/SecurityClassificationSelector';
import { useWorkflowStore } from '../../../store/workflow';
import { SecurityClassificationLevel } from '../../../types/security';

const { Title, Text, Paragraph } = Typography;

export const SecurityValidationStep: React.FC<BaseStepProps> = ({
  data,
  onDataChange,
  onValidation
}) => {
  const { updateStep } = useWorkflowStore();
  const [localData, setLocalData] = useState<Partial<SecurityValidationData>>({
    security_classification: data.security_classification,
    naming_compliance: false,
    validation_passed: false
  });
  const [namingValidation, setNamingValidation] = useState<{
    isValid: boolean;
    message: string;
    suggestions?: string[];
  }>({ isValid: false, message: 'Checking...' });

  // Validate form whenever data changes
  useEffect(() => {
    validateForm();
  }, [localData, namingValidation]);

  // Validate naming compliance when asset name changes
  useEffect(() => {
    if (data.asset_name) {
      validateNamingCompliance(data.asset_name);
    }
  }, [data.asset_name]);

  const validateForm = () => {
    const errors = [];
    
    if (!localData.security_classification) {
      errors.push({ 
        field: 'security_classification', 
        message: 'Security classification is required', 
        code: 'REQUIRED' 
      });
    }

    if (!namingValidation.isValid) {
      errors.push({
        field: 'naming_compliance',
        message: 'Asset name must comply with security standards',
        code: 'NAMING_COMPLIANCE'
      });
    }

    const isValid = errors.length === 0;
    setLocalData(prev => ({ ...prev, validation_passed: isValid }));
    onValidation?.(isValid, errors);
  };

  const validateNamingCompliance = async (assetName: string) => {
    try {
      // This would typically call the backend validation service
      // For now, implement basic validation rules
      const validation = performNamingValidation(assetName);
      setNamingValidation(validation);
      setLocalData(prev => ({ ...prev, naming_compliance: validation.isValid }));
    } catch (error) {
      setNamingValidation({
        isValid: false,
        message: 'Failed to validate naming compliance'
      });
    }
  };

  const performNamingValidation = (name: string): {
    isValid: boolean;
    message: string;
    suggestions?: string[];
  } => {
    const issues = [];
    const suggestions = [];

    // Check for prohibited characters
    if (/[<>:"/\\|?*]/.test(name)) {
      issues.push('Contains prohibited characters');
      suggestions.push('Remove characters: < > : " / \\ | ? *');
    }

    // Check for reserved words (basic set)
    const reservedWords = ['CON', 'PRN', 'AUX', 'NUL', 'COM1', 'COM2', 'LPT1', 'LPT2'];
    if (reservedWords.includes(name.toUpperCase())) {
      issues.push('Uses reserved system name');
      suggestions.push('Choose a different name');
    }

    // Check length
    if (name.length > 100) {
      issues.push('Name too long (max 100 characters)');
      suggestions.push('Shorten the name');
    }

    // Check for leading/trailing spaces
    if (name !== name.trim()) {
      issues.push('Contains leading or trailing spaces');
      suggestions.push('Remove extra spaces');
    }

    // Check for consecutive spaces
    if (/\s{2,}/.test(name)) {
      issues.push('Contains consecutive spaces');
      suggestions.push('Use single spaces only');
    }

    const isValid = issues.length === 0;
    
    return {
      isValid,
      message: isValid ? 'Name complies with security standards' : issues.join(', '),
      suggestions: suggestions.length > 0 ? suggestions : undefined
    };
  };

  const handleClassificationChange = async (classification: SecurityClassificationLevel) => {
    const updatedData = { 
      ...localData,
      security_classification: classification
    };
    
    setLocalData(updatedData);

    try {
      await updateStep('security_validation', updatedData);
      onDataChange?.(updatedData);
    } catch (error) {
      console.error('Failed to update workflow step:', error);
    }
  };

  const getValidationStatus = () => {
    if (!localData.security_classification) {
      return {
        icon: <ExclamationCircleOutlined style={{ color: '#faad14' }} />,
        title: 'Security Classification Required',
        message: 'Please select a security classification for this asset.',
        type: 'warning' as const
      };
    }

    if (!namingValidation.isValid) {
      return {
        icon: <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />,
        title: 'Naming Compliance Issue',
        message: namingValidation.message,
        type: 'error' as const,
        suggestions: namingValidation.suggestions
      };
    }

    return {
      icon: <CheckCircleOutlined style={{ color: '#52c41a' }} />,
      title: 'Security Validation Passed',
      message: 'Asset meets all security requirements and can be created.',
      type: 'success' as const
    };
  };

  const validationStatus = getValidationStatus();

  return (
    <div>
      <div style={{ marginBottom: '32px', textAlign: 'center' }}>
        <Title level={3}>Security Validation</Title>
        <Paragraph type="secondary">
          Set security classification and validate compliance for "{data.asset_name}".
        </Paragraph>
      </div>

      <Row gutter={24}>
        <Col span={12}>
          <Card style={{ height: '100%' }}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <div style={{ display: 'flex', alignItems: 'center', marginBottom: '16px' }}>
                <SecurityScanOutlined style={{ color: '#1890ff', fontSize: '20px', marginRight: '8px' }} />
                <Title level={5} style={{ margin: 0 }}>Security Classification</Title>
              </div>
              
              <SecurityClassificationSelector
                value={localData.security_classification}
                onChange={handleClassificationChange}
                assetType={data.asset_type}
                style={{ width: '100%' }}
              />

              <Divider />

              <div style={{ display: 'flex', alignItems: 'center', marginBottom: '16px' }}>
                <LockOutlined style={{ color: '#1890ff', fontSize: '20px', marginRight: '8px' }} />
                <Title level={5} style={{ margin: 0 }}>Naming Compliance</Title>
              </div>

              <Space direction="vertical" style={{ width: '100%' }}>
                <Text strong>Asset Name</Text>
                <div style={{ 
                  padding: '8px 12px',
                  border: namingValidation.isValid ? '1px solid #d9d9d9' : '1px solid #ff4d4f',
                  borderRadius: '4px',
                  background: '#f5f5f5'
                }}>
                  <Text>{data.asset_name || 'No name provided'}</Text>
                </div>
                {!namingValidation.isValid && (
                  <Alert
                    message={namingValidation.message}
                    type="error"
                    showIcon
                    style={{ marginTop: '8px' }}
                  />
                )}
              </Space>
            </Space>
          </Card>
        </Col>

        <Col span={12}>
          <Card style={{ height: '100%' }}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <div style={{ display: 'flex', alignItems: 'center', marginBottom: '16px' }}>
                {validationStatus.icon}
                <Title level={5} style={{ margin: '0 0 0 8px' }}>
                  Validation Status
                </Title>
              </div>

              <Alert
                message={validationStatus.title}
                description={validationStatus.message}
                type={validationStatus.type}
                showIcon={false}
              />

              {validationStatus.suggestions && (
                <div style={{ marginTop: '16px' }}>
                  <Text strong>Suggestions:</Text>
                  <ul style={{ marginTop: '8px', marginBottom: '0' }}>
                    {validationStatus.suggestions.map((suggestion, index) => (
                      <li key={index}>
                        <Text type="secondary">{suggestion}</Text>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              <div style={{ marginTop: '16px', padding: '12px', backgroundColor: '#f6ffed', borderRadius: '6px' }}>
                <Text strong style={{ color: '#52c41a' }}>Asset Summary</Text>
                <div style={{ marginTop: '8px' }}>
                  <Text type="secondary" style={{ display: 'block' }}>Name: {data.asset_name}</Text>
                  <Text type="secondary" style={{ display: 'block' }}>Type: {data.asset_type}</Text>
                  <Text type="secondary" style={{ display: 'block' }}>Location: {data.parent_path || 'Root Level'}</Text>
                  {localData.security_classification && (
                    <Text type="secondary" style={{ display: 'block' }}>
                      Classification: {localData.security_classification}
                    </Text>
                  )}
                </div>
              </div>
            </Space>
          </Card>
        </Col>
      </Row>

      <div style={{ marginTop: '24px', padding: '16px', backgroundColor: '#fafafa', borderRadius: '6px' }}>
        <Title level={5} style={{ margin: '0 0 8px 0' }}>Security Guidelines</Title>
        <ul style={{ margin: 0, paddingLeft: '20px' }}>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Select appropriate security classification based on asset sensitivity</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Asset names must comply with organizational security standards</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Higher classifications may require additional approval workflows</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Security settings can be updated later with appropriate permissions</Text>
          </li>
        </ul>
      </div>
    </div>
  );
};