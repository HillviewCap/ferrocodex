import React, { useState, useEffect, useCallback } from 'react';
import { Input, Button, Space, Alert, App } from 'antd';
import { KeyOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { PasswordStrength } from '../types/vault';
import PasswordStrengthIndicator from './PasswordStrengthIndicator';
import PasswordGenerator from './PasswordGenerator';
import useAuthStore from '../store/auth';
// Simple debounce implementation to avoid lodash dependency
const debounce = <T extends (...args: any[]) => any>(
  func: T,
  wait: number
): T & { cancel: () => void } => {
  let timeout: number;
  const debounced = ((...args: Parameters<T>) => {
    clearTimeout(timeout);
    timeout = window.setTimeout(() => func(...args), wait);
  }) as T & { cancel: () => void };
  
  debounced.cancel = () => {
    clearTimeout(timeout);
  };
  
  return debounced;
};

interface PasswordInputProps {
  value?: string;
  onChange?: (value: string) => void;
  onStrengthChange?: (strength: PasswordStrength | null) => void;
  onReuseCheck?: (isReused: boolean) => void;
  placeholder?: string;
  disabled?: boolean;
  secretId?: number; // For password reuse checking
  showStrength?: boolean;
  showGenerator?: boolean;
  checkReuse?: boolean;
  minScore?: number;
}

const PasswordInput: React.FC<PasswordInputProps> = ({
  value = '',
  onChange,
  onStrengthChange,
  onReuseCheck,
  placeholder = 'Enter password',
  disabled = false,
  secretId,
  showStrength = true,
  showGenerator = true,
  checkReuse = true,
  minScore = 40
}) => {
  const { token } = useAuthStore();
  const { message } = App.useApp();
  
  const [showPassword, setShowPassword] = useState(false);
  const [strength, setStrength] = useState<PasswordStrength | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [generatorVisible, setGeneratorVisible] = useState(false);
  const [reuseWarning, setReuseWarning] = useState<string | null>(null);
  const [checkingReuse, setCheckingReuse] = useState(false);

  // Debounced functions
  const debouncedAnalyzePassword = useCallback(
    debounce(async (password: string) => {
      if (!token || !password) {
        setStrength(null);
        onStrengthChange?.(null);
        return;
      }

      setAnalyzing(true);
      try {
        const result = await invoke<PasswordStrength>('validate_password_strength', {
          token,
          password
        });
        setStrength(result);
        onStrengthChange?.(result);
      } catch (err) {
        console.error('Failed to analyze password:', err);
        setStrength(null);
        onStrengthChange?.(null);
      } finally {
        setAnalyzing(false);
      }
    }, 300),
    [token, onStrengthChange]
  );

  const debouncedCheckReuse = useCallback(
    debounce(async (password: string) => {
      if (!token || !password || !checkReuse) {
        setReuseWarning(null);
        onReuseCheck?.(false);
        return;
      }

      setCheckingReuse(true);
      try {
        const isReused = await invoke<boolean>('check_password_reuse', {
          token,
          password,
          excludeSecretId: secretId || null
        });
        
        if (isReused) {
          const warning = 'This password has been used for another credential';
          setReuseWarning(warning);
          onReuseCheck?.(true);
        } else {
          setReuseWarning(null);
          onReuseCheck?.(false);
        }
      } catch (err) {
        console.error('Failed to check password reuse:', err);
        setReuseWarning(null);
        onReuseCheck?.(false);
      } finally {
        setCheckingReuse(false);
      }
    }, 500),
    [token, checkReuse, secretId, onReuseCheck]
  );

  useEffect(() => {
    if (value) {
      debouncedAnalyzePassword(value);
      debouncedCheckReuse(value);
    } else {
      setStrength(null);
      setReuseWarning(null);
      onStrengthChange?.(null);
      onReuseCheck?.(false);
    }

    return () => {
      debouncedAnalyzePassword.cancel();
      debouncedCheckReuse.cancel();
    };
  }, [value, debouncedAnalyzePassword, debouncedCheckReuse]);

  const handlePasswordChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    onChange?.(newValue);
  };

  const handleGeneratedPassword = (generatedPassword: string) => {
    onChange?.(generatedPassword);
    message.success('Generated password applied');
  };

  const isWeakPassword = strength && strength.score < minScore;
  const hasIssues = isWeakPassword || reuseWarning;

  return (
    <Space direction="vertical" style={{ width: '100%' }}>
      <Input.Password
        value={value}
        onChange={handlePasswordChange}
        placeholder={placeholder}
        disabled={disabled}
        visibilityToggle={{
          visible: showPassword,
          onVisibleChange: setShowPassword,
        }}
        suffix={
          showGenerator && (
            <Button
              type="text"
              icon={<KeyOutlined />}
              onClick={() => setGeneratorVisible(true)}
              disabled={disabled}
              size="small"
            />
          )
        }
        status={hasIssues ? 'error' : undefined}
      />

      {/* Password Strength Indicator */}
      {showStrength && value && strength && (
        <PasswordStrengthIndicator
          strength={strength}
          showDetails={false}
          size="small"
        />
      )}

      {/* Warnings and Errors */}
      {reuseWarning && (
        <Alert
          message="Password Reuse Detected"
          description={reuseWarning}
          type="warning"
          showIcon
          icon={<ExclamationCircleOutlined />}
        />
      )}

      {isWeakPassword && (
        <Alert
          message="Weak Password"
          description={`Password strength is below recommended level (${strength.score}/${minScore}). Consider using the password generator for a stronger password.`}
          type="warning"
          showIcon
        />
      )}

      {/* Loading states */}
      {(analyzing || checkingReuse) && (
        <div style={{ fontSize: '12px', color: '#8c8c8c' }}>
          {analyzing && 'Analyzing password strength...'}
          {analyzing && checkingReuse && ' • '}
          {checkingReuse && 'Checking for password reuse...'}
        </div>
      )}

      {/* Password Generator Modal */}
      <PasswordGenerator
        visible={generatorVisible}
        onCancel={() => setGeneratorVisible(false)}
        onGenerated={handleGeneratedPassword}
        title="Generate Secure Password"
      />
    </Space>
  );
};

export default PasswordInput;