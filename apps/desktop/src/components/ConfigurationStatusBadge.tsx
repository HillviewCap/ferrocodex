import React from 'react';
import { Tag, Tooltip } from 'antd';
import {
  EditOutlined,
  CheckCircleOutlined,
  StarOutlined,
  FolderOutlined,
  TrophyOutlined
} from '@ant-design/icons';
import { ConfigurationStatus } from '../types/assets';

interface ConfigurationStatusBadgeProps {
  status: ConfigurationStatus;
  showIcon?: boolean;
  size?: 'small' | 'default';
  onClick?: () => void;
  disabled?: boolean;
}

const ConfigurationStatusBadge: React.FC<ConfigurationStatusBadgeProps> = ({
  status,
  showIcon = true,
  size = 'default',
  onClick,
  disabled = false
}) => {
  const getStatusConfig = (status: ConfigurationStatus) => {
    switch (status) {
      case 'Draft':
        return {
          color: 'default',
          icon: <EditOutlined />,
          description: 'Work in progress - not yet approved'
        };
      case 'Silver':
        return {
          color: 'cyan',
          icon: <TrophyOutlined />,
          description: 'Candidate for review - promoted from branch'
        };
      case 'Approved':
        return {
          color: 'success',
          icon: <CheckCircleOutlined />,
          description: 'Approved for use'
        };
      case 'Golden':
        return {
          color: 'gold',
          icon: <StarOutlined />,
          description: 'Golden master - reference configuration'
        };
      case 'Archived':
        return {
          color: 'warning',
          icon: <FolderOutlined />,
          description: 'Archived - no longer in active use'
        };
      default:
        return {
          color: 'default',
          icon: <EditOutlined />,
          description: 'Unknown status'
        };
    }
  };

  const config = getStatusConfig(status);
  
  const badge = (
    <Tag
      color={config.color}
      icon={showIcon ? config.icon : undefined}
      style={{
        cursor: onClick && !disabled ? 'pointer' : 'default',
        fontSize: size === 'small' ? '11px' : '12px',
        padding: size === 'small' ? '2px 6px' : '4px 8px',
        border: 'none',
        borderRadius: '4px',
        fontWeight: '500',
        opacity: disabled ? 0.6 : 1,
        pointerEvents: disabled ? 'none' : 'auto'
      }}
      onClick={onClick && !disabled ? onClick : undefined}
    >
      {status}
    </Tag>
  );

  if (onClick || config.description) {
    return (
      <Tooltip title={config.description} placement="top">
        {badge}
      </Tooltip>
    );
  }

  return badge;
};

export default ConfigurationStatusBadge;