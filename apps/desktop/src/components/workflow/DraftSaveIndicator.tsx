import React from 'react';
import { Typography, Space, Tooltip } from 'antd';
import { CheckCircleOutlined, SaveOutlined, LoadingOutlined, ClockCircleOutlined } from '@ant-design/icons';
import { AutoSaveConfig } from '../../types/workflow';

const { Text } = Typography;

interface DraftSaveIndicatorProps {
  autoSave: AutoSaveConfig;
  lastSaveTime: Date | null;
  onManualSave?: () => void;
}

export const DraftSaveIndicator: React.FC<DraftSaveIndicatorProps> = ({
  autoSave,
  lastSaveTime,
  onManualSave
}) => {
  const formatLastSaveTime = (saveTime: Date | null): string => {
    if (!saveTime) return 'Not saved';

    const now = new Date();
    const diffMs = now.getTime() - saveTime.getTime();
    const diffMinutes = Math.floor(diffMs / 60000);
    const diffSeconds = Math.floor((diffMs % 60000) / 1000);

    if (diffMinutes > 0) {
      return `Saved ${diffMinutes}m ago`;
    } else if (diffSeconds > 0) {
      return `Saved ${diffSeconds}s ago`;
    } else {
      return 'Just saved';
    }
  };

  const getSaveStatus = () => {
    if (autoSave.save_in_progress) {
      return {
        icon: <LoadingOutlined spin style={{ color: '#1890ff' }} />,
        text: 'Saving...',
        color: '#1890ff'
      };
    }

    if (lastSaveTime) {
      const now = new Date();
      const diffMs = now.getTime() - lastSaveTime.getTime();
      const diffMinutes = Math.floor(diffMs / 60000);

      if (diffMinutes < 1) {
        return {
          icon: <CheckCircleOutlined style={{ color: '#52c41a' }} />,
          text: 'Saved',
          color: '#52c41a'
        };
      }
    }

    return {
      icon: <SaveOutlined style={{ color: '#faad14' }} />,
      text: 'Not saved',
      color: '#faad14'
    };
  };

  const status = getSaveStatus();

  const tooltipContent = (
    <div>
      <div><strong>Auto-save:</strong> {autoSave.enabled ? `Every ${autoSave.interval}s` : 'Disabled'}</div>
      <div><strong>Last saved:</strong> {formatLastSaveTime(lastSaveTime)}</div>
      {autoSave.enabled && (
        <div style={{ marginTop: '8px', fontSize: '12px', opacity: 0.8 }}>
          Your progress is automatically saved every {autoSave.interval} seconds
        </div>
      )}
    </div>
  );

  return (
    <Tooltip title={tooltipContent} placement="bottomRight">
      <Space 
        size="small" 
        style={{ 
          cursor: 'pointer',
          padding: '4px 8px',
          borderRadius: '4px',
          backgroundColor: '#fafafa',
          border: '1px solid #f0f0f0',
          transition: 'all 0.3s'
        }}
        onClick={onManualSave}
      >
        {status.icon}
        <Text style={{ fontSize: '12px', color: status.color }}>
          {status.text}
        </Text>
        {autoSave.enabled && (
          <ClockCircleOutlined 
            style={{ 
              fontSize: '12px', 
              color: '#8c8c8c',
              marginLeft: '4px'
            }} 
          />
        )}
      </Space>
    </Tooltip>
  );
};