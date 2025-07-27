import React, { useState, useEffect } from 'react';
import { Card, Form, Select, Switch, Button, Space, notification, Spin, Alert, Typography, Divider } from 'antd';
import { CalendarOutlined, SaveOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { RotationSchedule, CreateRotationScheduleRequest, UpdateRotationScheduleRequest, rotationIntervalOptions, rotationAlertOptions } from '../types/vault';
import useAuthStore from '../store/auth';

const { Text } = Typography;

interface RotationScheduleManagerProps {
  vaultId: number;
  vaultName: string;
  onUpdate?: () => void;
}

const RotationScheduleManager: React.FC<RotationScheduleManagerProps> = ({ vaultId, vaultName, onUpdate }) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [schedule, setSchedule] = useState<RotationSchedule | null>(null);
  const [hasChanges, setHasChanges] = useState(false);
  
  const { token, user } = useAuthStore();

  useEffect(() => {
    loadSchedule();
  }, [vaultId]);

  const loadSchedule = async () => {
    if (!token) return;

    try {
      setLoading(true);
      const existingSchedule = await invoke<RotationSchedule | null>('get_rotation_schedule', {
        token,
        vault_id: vaultId,
      });

      if (existingSchedule) {
        setSchedule(existingSchedule);
        form.setFieldsValue({
          rotation_interval: existingSchedule.rotation_interval,
          alert_days_before: existingSchedule.alert_days_before,
          is_active: existingSchedule.is_active,
        });
      } else {
        // Set default values for new schedule
        form.setFieldsValue({
          rotation_interval: 90,
          alert_days_before: 14,
          is_active: true,
        });
      }
    } catch (error) {
      console.error('Failed to load rotation schedule:', error);
      notification.error({
        message: 'Load Failed',
        description: 'Failed to load rotation schedule',
      });
    } finally {
      setLoading(false);
    }
  };

  const handleFormChange = () => {
    setHasChanges(true);
  };

  const handleSave = async () => {
    if (!token || !user) return;

    try {
      setSaving(true);
      const values = form.getFieldsValue();

      if (schedule) {
        // Update existing schedule
        const updateRequest: UpdateRotationScheduleRequest = {
          schedule_id: schedule.schedule_id,
          rotation_interval: values.rotation_interval,
          alert_days_before: values.alert_days_before,
          is_active: values.is_active,
        };

        await invoke('update_rotation_policy', {
          token,
          request: updateRequest,
        });

        notification.success({
          message: 'Schedule Updated',
          description: 'Rotation schedule has been updated successfully',
        });
      } else {
        // Create new schedule
        const createRequest: CreateRotationScheduleRequest = {
          vault_id: vaultId,
          rotation_interval: values.rotation_interval,
          alert_days_before: values.alert_days_before,
          created_by: user.id,
        };

        const newSchedule = await invoke<RotationSchedule>('create_rotation_schedule', {
          token,
          request: createRequest,
        });

        setSchedule(newSchedule);

        notification.success({
          message: 'Schedule Created',
          description: 'Rotation schedule has been created successfully',
        });
      }

      setHasChanges(false);
      if (onUpdate) onUpdate();
      
      // Reload to get the latest data
      await loadSchedule();
    } catch (error) {
      notification.error({
        message: 'Save Failed',
        description: error instanceof Error ? error.message : 'Failed to save rotation schedule',
      });
    } finally {
      setSaving(false);
    }
  };

  const calculateNextRotation = () => {
    const interval = form.getFieldValue('rotation_interval');
    if (!interval) return null;

    const nextDate = new Date();
    nextDate.setDate(nextDate.getDate() + interval);
    return nextDate.toLocaleDateString();
  };

  const calculateAlertDate = () => {
    const interval = form.getFieldValue('rotation_interval');
    const alertDays = form.getFieldValue('alert_days_before');
    if (!interval || !alertDays) return null;

    const alertDate = new Date();
    alertDate.setDate(alertDate.getDate() + interval - alertDays);
    return alertDate.toLocaleDateString();
  };

  return (
    <Card
      title={
        <Space>
          <CalendarOutlined />
          Rotation Schedule - {vaultName}
        </Space>
      }
      extra={
        user?.role === 'Administrator' && (
          <Button
            type="primary"
            icon={<SaveOutlined />}
            onClick={handleSave}
            loading={saving}
            disabled={!hasChanges}
          >
            Save Schedule
          </Button>
        )
      }
    >
      <Spin spinning={loading}>
        <Form
          form={form}
          layout="vertical"
          onValuesChange={handleFormChange}
          disabled={user?.role !== 'Administrator'}
        >
          <Alert
            message="Password Rotation Policy"
            description="Configure automatic password rotation reminders and compliance tracking for this vault."
            type="info"
            showIcon
            style={{ marginBottom: 24 }}
          />

          <Form.Item
            name="rotation_interval"
            label="Rotation Interval"
            rules={[{ required: true, message: 'Please select rotation interval' }]}
          >
            <Select
              size="large"
              options={rotationIntervalOptions}
              placeholder="Select rotation interval"
            />
          </Form.Item>

          <Form.Item
            name="alert_days_before"
            label="Alert Before Rotation"
            rules={[{ required: true, message: 'Please select alert timing' }]}
          >
            <Select
              size="large"
              options={rotationAlertOptions}
              placeholder="Select when to alert"
            />
          </Form.Item>

          <Form.Item
            name="is_active"
            label="Schedule Status"
            valuePropName="checked"
          >
            <Switch checkedChildren="Active" unCheckedChildren="Inactive" />
          </Form.Item>

          <Divider />

          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <Text type="secondary">Next Rotation Due:</Text>
              <br />
              <Text strong>{calculateNextRotation() || 'Not calculated'}</Text>
            </div>
            
            <div>
              <Text type="secondary">Alert Will Be Sent On:</Text>
              <br />
              <Text strong>{calculateAlertDate() || 'Not calculated'}</Text>
            </div>

            {schedule && (
              <>
                <div>
                  <Text type="secondary">Created On:</Text>
                  <br />
                  <Text>{new Date(schedule.created_at).toLocaleString()}</Text>
                </div>
                
                <div>
                  <Text type="secondary">Last Updated:</Text>
                  <br />
                  <Text>{new Date(schedule.updated_at).toLocaleString()}</Text>
                </div>
              </>
            )}
          </Space>

          {user?.role !== 'Administrator' && (
            <Alert
              message="View Only"
              description="Only administrators can modify rotation schedules."
              type="warning"
              showIcon
              style={{ marginTop: 24 }}
            />
          )}
        </Form>
      </Spin>
    </Card>
  );
};

export default RotationScheduleManager;