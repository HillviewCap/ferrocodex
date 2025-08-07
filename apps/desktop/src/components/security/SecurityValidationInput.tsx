import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Input, AutoComplete, Space, Typography, Alert, Spin, Tooltip } from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  LoadingOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { SecurityValidationResult } from '../../types/security';
import useAuthStore from '../../store/auth';

const { Text } = Typography;

interface SecurityValidationInputProps {
  value?: string;
  onChange: (value: string) => void;
  onValidationChange: (result: SecurityValidationResult) => void;
  placeholder?: string;
  disabled?: boolean;
  showSuggestions?: boolean;
  validationDelay?: number;
  maxLength?: number;
  autoFocus?: boolean;
  size?: 'small' | 'middle' | 'large';
}

interface ValidationState {
  status: 'idle' | 'validating' | 'success' | 'error' | 'warning';
  result?: SecurityValidationResult;
  suggestions: string[];
  isLoading: boolean;
}

const SecurityValidationInput: React.FC<SecurityValidationInputProps> = ({
  value = '',
  onChange,
  onValidationChange,
  placeholder = 'Enter asset name',
  disabled = false,
  showSuggestions = true,
  validationDelay = 100,
  maxLength = 100,
  autoFocus = false,
  size = 'middle'
}) => {
  const [validationState, setValidationState] = useState<ValidationState>({
    status: 'idle',
    suggestions: [],
    isLoading: false
  });
  
  const { token } = useAuthStore();
  const validationTimer = useRef<number | null>(null);
  const lastValidatedValue = useRef<string>('');

  // Debounced validation function
  const validateInput = useCallback(async (inputValue: string) => {
    if (!inputValue.trim()) {
      return;
    }
    
    // Force revalidation by not checking lastValidatedValue for now
    // if (inputValue === lastValidatedValue.current) {
    //   return;
    // }

    lastValidatedValue.current = inputValue;
    
    setValidationState(prev => ({
      ...prev,
      status: 'validating',
      isLoading: true
    }));

    try {
      const result = await invoke<SecurityValidationResult>('validate_asset_name', {
        token,
        name: inputValue
      });

      console.log('Validation result from backend:', result);
      console.log('Result type:', typeof result, 'isValid type:', typeof result.isValid);
      console.log('Result keys:', Object.keys(result));
      
      // Check for both camelCase and snake_case
      const isValid = result.isValid !== undefined ? result.isValid : (result as any).is_valid;
      
      if (isValid === undefined) {
        console.error('Neither isValid nor is_valid found in result:', result);
      }

      const validationResult: SecurityValidationResult = {
        isValid: isValid || false,
        errorCode: result.errorCode || (result as any).error_code,
        errorMessage: result.errorMessage || (result as any).error_message,
        suggestedCorrections: result.suggestedCorrections || (result as any).suggested_corrections || [],
        securityFlags: result.securityFlags || (result as any).security_flags || [],
        validationTimestamp: new Date().toISOString()
      };

      console.log('Setting validation state - isValid:', isValid, 'validationResult:', validationResult);
      
      setValidationState({
        status: isValid ? 'success' : 'error',
        result: validationResult,
        suggestions: validationResult.suggestedCorrections,
        isLoading: false
      });

      onValidationChange(validationResult);

      // Auto-fetch suggestions if validation failed and suggestions are enabled
      if (!isValid && showSuggestions && validationResult.suggestedCorrections.length === 0) {
        try {
          const suggestions = await invoke<string[]>('suggest_compliant_names', {
            token,
            input: inputValue
          });
          
          setValidationState(prev => ({
            ...prev,
            suggestions: suggestions || []
          }));
        } catch (error) {
          console.warn('Failed to fetch name suggestions:', error);
        }
      }

    } catch (error) {
      console.error('Validation error:', error);
      const errorMessage = error instanceof Error ? error.message : 'Validation failed';
      
      const errorResult: SecurityValidationResult = {
        isValid: false,
        errorCode: 'VALIDATION_ERROR',
        errorMessage,
        suggestedCorrections: [],
        securityFlags: [],
        validationTimestamp: new Date().toISOString()
      };

      setValidationState({
        status: 'error',
        result: errorResult,
        suggestions: [],
        isLoading: false
      });

      onValidationChange(errorResult);
    }
  }, [token, onValidationChange, showSuggestions]);

  // Normalize input for preview (same logic as backend)
  const normalizeForPreview = (input: string): string => {
    return input
      .split('')
      .filter(char => /[a-zA-Z0-9_-]/.test(char))
      .join('')
      .toUpperCase();
  };

  // Handle input change with debounced validation
  const handleInputChange = (newValue: string) => {
    onChange(newValue);

    // Clear existing timer
    if (validationTimer.current) {
      window.clearTimeout(validationTimer.current);
    }

    // Reset validation state if input is empty
    if (!newValue.trim()) {
      setValidationState({
        status: 'idle',
        suggestions: [],
        isLoading: false
      });
      return;
    }

    // Set new timer for validation
    validationTimer.current = window.setTimeout(() => {
      validateInput(newValue);
    }, validationDelay);
  };

  // Handle suggestion selection
  const handleSuggestionSelect = (suggestion: string) => {
    onChange(suggestion);
    // Immediately validate the suggestion
    validateInput(suggestion);
  };

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (validationTimer.current) {
        window.clearTimeout(validationTimer.current);
      }
    };
  }, []);

  // Get validation icon and color
  const getValidationIcon = () => {
    switch (validationState.status) {
      case 'validating':
        return <LoadingOutlined style={{ color: '#1890ff' }} />;
      case 'success':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'error':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      case 'warning':
        return <ExclamationCircleOutlined style={{ color: '#faad14' }} />;
      default:
        return null;
    }
  };

  // Get input status for Ant Design styling
  const getInputStatus = (): 'error' | 'warning' | undefined => {
    switch (validationState.status) {
      case 'error':
        return 'error';
      case 'warning':
        return 'warning';
      default:
        return undefined;
    }
  };

  // Character count display
  const getCharacterCount = () => {
    const currentLength = value?.length || 0;
    const remaining = maxLength - currentLength;
    const isOverLimit = remaining < 0;
    
    return (
      <Text 
        style={{ 
          fontSize: '12px', 
          color: isOverLimit ? '#ff4d4f' : remaining < 10 ? '#faad14' : '#666',
          float: 'right'
        }}
      >
        {currentLength}/{maxLength}
      </Text>
    );
  };

  // Format suggestions for AutoComplete
  const suggestionOptions = validationState.suggestions.map(suggestion => ({
    value: suggestion,
    label: (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Text>{suggestion}</Text>
        <CheckCircleOutlined style={{ color: '#52c41a', fontSize: '12px' }} />
      </div>
    )
  }));

  return (
    <div>
      <div style={{ position: 'relative' }}>
        {showSuggestions ? (
          <AutoComplete
            value={value || ''}
            onChange={handleInputChange}
            onSelect={handleSuggestionSelect}
            options={suggestionOptions}
            disabled={disabled}
            style={{ width: '100%' }}
            popupMatchSelectWidth={true}
          >
            <Input
              placeholder={placeholder}
              status={getInputStatus()}
              suffix={
                <Space>
                  {validationState.isLoading && <Spin size="small" />}
                  {getValidationIcon()}
                </Space>
              }
              autoFocus={autoFocus}
              size={size}
            />
          </AutoComplete>
        ) : (
          <Input
            value={value || ''}
            onChange={(e) => handleInputChange(e.target.value)}
            placeholder={placeholder}
            status={getInputStatus()}
            suffix={
              <Space>
                {validationState.isLoading && <Spin size="small" />}
                {getValidationIcon()}
              </Space>
            }
            disabled={disabled}
            autoFocus={autoFocus}
            size={size}
          />
        )}
        
        {/* Character count and normalized preview */}
        <div style={{ marginTop: 4, minHeight: 16, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          {getCharacterCount()}
          {value && value.trim() && normalizeForPreview(value) !== value && (
            <Text 
              style={{ 
                fontSize: '11px', 
                color: '#666',
                fontStyle: 'italic'
              }}
            >
              Will be saved as: {normalizeForPreview(value)}
            </Text>
          )}
        </div>
      </div>

      {/* Validation feedback */}
      {validationState.result && validationState.status !== 'idle' && (
        <div style={{ marginTop: 8 }}>
          {validationState.status === 'success' && (
            <Alert
              message="Valid asset name"
              type="success"
              showIcon
              style={{ fontSize: '12px' }}
            />
          )}
          
          {validationState.status === 'error' && validationState.result.errorMessage && (
            <Alert
              message="Asset name needs adjustment"
              description={validationState.result.errorMessage.replace(/uppercase/gi, '').replace(/security/gi, '').trim()}
              type="warning"
              showIcon
              style={{ fontSize: '12px' }}
              action={
                validationState.suggestions.length > 0 && (
                  <Tooltip title="Click input field to see suggestions">
                    <InfoCircleOutlined style={{ cursor: 'help' }} />
                  </Tooltip>
                )
              }
            />
          )}

        </div>
      )}

      {/* Real-time validation feedback */}
      {validationState.status === 'validating' && (
        <div style={{ marginTop: 4 }}>
          <Text type="secondary" style={{ fontSize: '11px' }}>
            <LoadingOutlined /> Validating name...
          </Text>
        </div>
      )}
    </div>
  );
};

export default SecurityValidationInput;