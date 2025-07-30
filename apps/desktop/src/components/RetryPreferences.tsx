import React, { useState, useEffect } from 'react';
import {
  Card,
  Form,
  Switch,
  InputNumber,
  Button,
  Space,
  Typography,
  Row,
  Col,
  Alert,
  Spin,
  message,
  Collapse,
  Tooltip,
  Tag
} from 'antd';
import {
  SettingOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  SaveOutlined,
  ExperimentOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import { 
  UserSettings, 
  RetryPreferences
} from '../types/error-handling';

const { Title, Text, Paragraph } = Typography;
const { Panel } = Collapse;

interface RetryPreferencesProps {
  onSettingsUpdate?: (settings: UserSettings) => void;
}

const RetryPreferencesComponent: React.FC<RetryPreferencesProps> = ({ onSettingsUpdate }) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [userSettings, setUserSettings] = useState<UserSettings | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<string>('');

  // Available presets from backend
  const presets = [
    { value: 'conservative', label: 'Conservative', description: 'Safe retry settings for OT environments' },
    { value: 'aggressive', label: 'Aggressive', description: 'More retry attempts for better resilience' },
    { value: 'minimal', label: 'Minimal', description: 'Minimal retry attempts to reduce delays' }
  ];


  useEffect(() => {
    if (token) {
      loadUserSettings();
    }
  }, [token]);

  const loadUserSettings = async (): Promise<void> => {
    if (!token) return;

    setLoading(true);
    try {
      const settings = await invoke<UserSettings>('get_user_settings', { token });
      setUserSettings(settings);
      
      // Update form with loaded settings
      form.setFieldsValue({
        // Global settings
        enable_automatic_recovery: settings.retry_preferences.enable_automatic_recovery,
        show_retry_progress: settings.retry_preferences.show_retry_progress,
        show_technical_details: settings.retry_preferences.show_technical_details,
        auto_fallback_to_manual: settings.retry_preferences.auto_fallback_to_manual,
        recovery_notification_duration_ms: settings.retry_preferences.recovery_notification_duration_ms,
        
        // Global retry strategy
        global_max_attempts: settings.retry_preferences.global_retry_strategy.max_attempts,
        global_initial_delay_ms: settings.retry_preferences.global_retry_strategy.initial_delay_ms,
        global_max_delay_ms: settings.retry_preferences.global_retry_strategy.max_delay_ms,
        global_backoff_multiplier: settings.retry_preferences.global_retry_strategy.backoff_multiplier,
        global_jitter_factor: settings.retry_preferences.global_retry_strategy.jitter_factor,
        global_max_retry_duration_ms: settings.retry_preferences.global_retry_strategy.max_retry_duration_ms,
        global_enabled: settings.retry_preferences.global_retry_strategy.enabled,
      });

    } catch (error) {
      console.error('Failed to load user settings:', error);
      message.error('Failed to load retry preferences');
    } finally {
      setLoading(false);
    }
  };

  const handlePresetChange = async (presetName: string): Promise<void> => {
    if (!token) return;

    try {
      await invoke('apply_settings_preset', { 
        token, 
        presetName 
      });
      
      message.success(`Applied ${presetName} preset successfully`);
      setSelectedPreset(presetName);
      await loadUserSettings(); // Reload to show updated values
      
    } catch (error) {
      console.error('Failed to apply preset:', error);
      message.error(`Failed to apply ${presetName} preset`);
    }
  };

  const handleSave = async (values: any): Promise<void> => {
    if (!token || !userSettings) return;

    setSaving(true);
    try {
      // Build retry preferences from form values
      const retryPreferences: RetryPreferences = {
        global_retry_strategy: {
          max_attempts: values.global_max_attempts,
          initial_delay_ms: values.global_initial_delay_ms,
          max_delay_ms: values.global_max_delay_ms,
          backoff_multiplier: values.global_backoff_multiplier,
          jitter_factor: values.global_jitter_factor,
          max_retry_duration_ms: values.global_max_retry_duration_ms,
          enabled: values.global_enabled
        },
        operation_specific: userSettings.retry_preferences.operation_specific, // Keep existing
        circuit_breaker_configs: userSettings.retry_preferences.circuit_breaker_configs, // Keep existing
        enable_automatic_recovery: values.enable_automatic_recovery,
        show_retry_progress: values.show_retry_progress,
        show_technical_details: values.show_technical_details,
        auto_fallback_to_manual: values.auto_fallback_to_manual,
        recovery_notification_duration_ms: values.recovery_notification_duration_ms
      };

      await invoke('update_user_settings', {
        token,
        retryPreferences
      });

      message.success('Retry preferences saved successfully');
      
      // Reload settings to ensure consistency
      await loadUserSettings();
      
      if (onSettingsUpdate && userSettings) {
        onSettingsUpdate(userSettings);
      }

    } catch (error) {
      console.error('Failed to save retry preferences:', error);
      message.error('Failed to save retry preferences');
    } finally {
      setSaving(false);
    }
  };

  const handleReset = (): void => {
    if (userSettings) {
      form.resetFields();
      loadUserSettings();
      message.info('Form reset to saved values');
    }
  };

  if (loading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', padding: '50px' }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!userSettings) {
    return (
      <Alert
        message="Unable to Load Settings"
        description="Failed to load user settings. Please try refreshing the page."
        type="error"
        showIcon
      />
    );
  }

  return (
    <div style={{ padding: '24px', maxWidth: '1200px' }}>
      <div style={{ marginBottom: '24px' }}>
        <Title level={3}>
          <SettingOutlined style={{ marginRight: '8px' }} />
          Retry & Recovery Preferences
        </Title>
        <Paragraph type="secondary">
          Configure automatic retry behavior, circuit breakers, and recovery settings for system operations.
          These settings help the system recover from transient errors automatically.
        </Paragraph>
      </div>

      {/* Quick Presets */}
      <Card 
        title={
          <span>
            <ExperimentOutlined style={{ marginRight: '8px' }} />
            Quick Presets
          </span>
        }
        style={{ marginBottom: '24px' }}
      >
        <Row gutter={[16, 16]}>
          {presets.map(preset => (
            <Col xs={24} sm={8} key={preset.value}>
              <Card
                size="small"
                hoverable
                onClick={() => handlePresetChange(preset.value)}
                style={{ 
                  cursor: 'pointer',
                  borderColor: selectedPreset === preset.value ? '#1890ff' : undefined
                }}
              >
                <div style={{ textAlign: 'center' }}>
                  <Title level={5} style={{ margin: '0 0 8px 0' }}>
                    {preset.label}
                  </Title>
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    {preset.description}
                  </Text>
                </div>
              </Card>
            </Col>
          ))}
        </Row>
      </Card>

      <Form
        form={form}
        layout="vertical"
        onFinish={handleSave}
        initialValues={{
          enable_automatic_recovery: true,
          show_retry_progress: true,
          show_technical_details: false,
          auto_fallback_to_manual: true,
          recovery_notification_duration_ms: 5000,
          global_max_attempts: 3,
          global_initial_delay_ms: 100,
          global_max_delay_ms: 5000,
          global_backoff_multiplier: 2.0,
          global_jitter_factor: 0.1,
          global_max_retry_duration_ms: 30000,
          global_enabled: true
        }}
      >
        {/* General Settings */}
        <Card title="General Settings" style={{ marginBottom: '24px' }}>
          <Row gutter={[24, 16]}>
            <Col xs={24} sm={12}>
              <Form.Item
                name="enable_automatic_recovery"
                valuePropName="checked"
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <Switch />
                  <span style={{ marginLeft: '12px' }}>Enable Automatic Recovery</span>
                  <Tooltip title="Allow the system to automatically retry failed operations">
                    <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                  </Tooltip>
                </div>
              </Form.Item>
            </Col>
            
            <Col xs={24} sm={12}>
              <Form.Item
                name="show_retry_progress"
                valuePropName="checked"
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <Switch />
                  <span style={{ marginLeft: '12px' }}>Show Retry Progress</span>
                  <Tooltip title="Display progress indicators during retry attempts">
                    <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                  </Tooltip>
                </div>
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="show_technical_details"
                valuePropName="checked"
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <Switch />
                  <span style={{ marginLeft: '12px' }}>Show Technical Details</span>
                  <Tooltip title="Include technical error information in messages">
                    <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                  </Tooltip>
                </div>
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="auto_fallback_to_manual"
                valuePropName="checked"
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <Switch />
                  <span style={{ marginLeft: '12px' }}>Auto-fallback to Manual</span>
                  <Tooltip title="Automatically offer manual recovery options when automatic retry fails">
                    <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                  </Tooltip>
                </div>
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="recovery_notification_duration_ms"
                label={
                  <span>
                    Notification Duration (ms)
                    <Tooltip title="How long recovery messages are displayed">
                      <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                    </Tooltip>
                  </span>
                }
                rules={[
                  { required: true, message: 'Duration is required' },
                  { type: 'number', min: 1000, max: 60000, message: 'Duration must be between 1-60 seconds' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={1000}
                  max={60000}
                  step={1000}
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Global Retry Strategy */}
        <Card title="Global Retry Strategy" style={{ marginBottom: '24px' }}>
          <Row gutter={[24, 16]}>
            <Col xs={24} sm={12}>
              <Form.Item
                name="global_enabled"
                valuePropName="checked"
              >
                <div style={{ display: 'flex', alignItems: 'center' }}>
                  <Switch />
                  <span style={{ marginLeft: '12px' }}>Enable Global Retry</span>
                </div>
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_max_attempts"
                label="Max Retry Attempts"
                rules={[
                  { required: true, message: 'Max attempts is required' },
                  { type: 'number', min: 0, max: 10, message: 'Must be between 0-10' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={0}
                  max={10}
                />
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_initial_delay_ms"
                label="Initial Delay (ms)"
                rules={[
                  { required: true, message: 'Initial delay is required' },
                  { type: 'number', min: 10, max: 10000, message: 'Must be between 10-10000ms' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={10}
                  max={10000}
                />
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_max_delay_ms"
                label="Max Delay (ms)"
                rules={[
                  { required: true, message: 'Max delay is required' },
                  { type: 'number', min: 100, max: 60000, message: 'Must be between 100ms-60s' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={100}
                  max={60000}
                />
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_backoff_multiplier"
                label={
                  <span>
                    Backoff Multiplier
                    <Tooltip title="How much to increase delay between retries (exponential backoff)">
                      <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                    </Tooltip>
                  </span>
                }
                rules={[
                  { required: true, message: 'Backoff multiplier is required' },
                  { type: 'number', min: 1.0, max: 5.0, message: 'Must be between 1.0-5.0' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={1.0}
                  max={5.0}
                  step={0.1}
                  precision={1}
                />
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_jitter_factor"
                label={
                  <span>
                    Jitter Factor
                    <Tooltip title="Random variation to prevent all systems retrying at the same time (0.0-1.0)">
                      <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                    </Tooltip>
                  </span>
                }
                rules={[
                  { required: true, message: 'Jitter factor is required' },
                  { type: 'number', min: 0.0, max: 1.0, message: 'Must be between 0.0-1.0' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={0.0}
                  max={1.0}
                  step={0.1}
                  precision={1}
                />
              </Form.Item>
            </Col>

            <Col xs={24} sm={12}>
              <Form.Item
                name="global_max_retry_duration_ms"
                label={
                  <span>
                    Max Total Duration (ms)
                    <Tooltip title="Maximum time to spend on all retry attempts combined">
                      <QuestionCircleOutlined style={{ marginLeft: '8px', color: '#999' }} />
                    </Tooltip>
                  </span>
                }
                rules={[
                  { type: 'number', min: 1000, max: 300000, message: 'Must be between 1s-5min' }
                ]}
              >
                <InputNumber
                  style={{ width: '100%' }}
                  min={1000}
                  max={300000}
                  step={1000}
                  placeholder="Optional (ms)"
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Advanced Configuration */}
        <Collapse style={{ marginBottom: '24px' }}>
          <Panel 
            header="Advanced Configuration" 
            key="advanced"
            extra={<Tag color="blue">Advanced</Tag>}
          >
            <Alert
              message="Advanced Settings"
              description="These settings are for advanced users. Per-operation and circuit breaker configurations allow fine-tuning of retry behavior for specific operations and services."
              type="info"
              style={{ marginBottom: '16px' }}
            />
            
            <Row gutter={[24, 16]}>
              <Col xs={24} sm={12}>
                <Title level={5}>Operation-Specific Settings</Title>
                <Text type="secondary">
                  Currently configured operations: {Object.keys(userSettings.retry_preferences.operation_specific).length}
                </Text>
                {Object.keys(userSettings.retry_preferences.operation_specific).map(op => (
                  <div key={op} style={{ marginTop: '8px' }}>
                    <Tag>{op}</Tag>
                  </div>
                ))}
              </Col>
              
              <Col xs={24} sm={12}>
                <Title level={5}>Circuit Breaker Settings</Title>
                <Text type="secondary">
                  Currently configured services: {Object.keys(userSettings.retry_preferences.circuit_breaker_configs).length}
                </Text>
                {Object.keys(userSettings.retry_preferences.circuit_breaker_configs).map(service => (
                  <div key={service} style={{ marginTop: '8px' }}>
                    <Tag color="orange">{service}</Tag>
                  </div>
                ))}
              </Col>
            </Row>
          </Panel>
        </Collapse>

        {/* Action Buttons */}
        <div style={{ textAlign: 'right' }}>
          <Space>
            <Button 
              icon={<ReloadOutlined />} 
              onClick={handleReset}
            >
              Reset
            </Button>
            <Button 
              type="primary" 
              icon={<SaveOutlined />} 
              htmlType="submit"
              loading={saving}
            >
              Save Settings
            </Button>
          </Space>
        </div>
      </Form>
    </div>
  );
};

export default RetryPreferencesComponent;