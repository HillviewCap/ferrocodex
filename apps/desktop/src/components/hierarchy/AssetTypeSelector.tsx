import React from 'react';
import { Card, Radio, Typography, Space } from 'antd';
import { FolderOutlined, ToolOutlined } from '@ant-design/icons';
import { AssetType } from '../../types/assets';

const { Title, Text } = Typography;

export interface AssetTypeSelectorProps {
  value?: AssetType;
  onChange?: (value: AssetType) => void;
  disabled?: boolean;
}

export const AssetTypeSelector: React.FC<AssetTypeSelectorProps> = ({
  value,
  onChange,
  disabled = false,
}) => {
  const handleChange = (e: any) => {
    onChange?.(e.target.value);
  };

  return (
    <div>
      <Title level={5} style={{ marginBottom: 16 }}>Asset Type</Title>
      <Radio.Group value={value} onChange={handleChange} disabled={disabled}>
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Card 
            hoverable={!disabled}
            style={{ 
              cursor: disabled ? 'not-allowed' : 'pointer',
              border: value === 'Folder' ? '2px solid #1890ff' : '1px solid #d9d9d9',
              opacity: disabled ? 0.6 : 1,
            }}
            bodyStyle={{ padding: 16 }}
            onClick={() => !disabled && onChange?.('Folder')}
          >
            <Radio value="Folder" style={{ pointerEvents: 'none' }}>
              <Space align="start" size={12}>
                <FolderOutlined 
                  style={{ 
                    fontSize: 24, 
                    color: value === 'Folder' ? '#1890ff' : '#666',
                    marginTop: 4,
                  }} 
                />
                <div>
                  <div style={{ fontWeight: 500, fontSize: 16 }}>Folder</div>
                  <Text type="secondary" style={{ fontSize: 14 }}>
                    A container that can hold other folders and devices. Use folders to organize 
                    your assets hierarchically (e.g., "Production Line 1", "Building A").
                  </Text>
                </div>
              </Space>
            </Radio>
          </Card>

          <Card 
            hoverable={!disabled}
            style={{ 
              cursor: disabled ? 'not-allowed' : 'pointer',
              border: value === 'Device' ? '2px solid #1890ff' : '1px solid #d9d9d9',
              opacity: disabled ? 0.6 : 1,
            }}
            bodyStyle={{ padding: 16 }}
            onClick={() => !disabled && onChange?.('Device')}
          >
            <Radio value="Device" style={{ pointerEvents: 'none' }}>
              <Space align="start" size={12}>
                <ToolOutlined 
                  style={{ 
                    fontSize: 24, 
                    color: value === 'Device' ? '#52c41a' : '#666',
                    marginTop: 4,
                  }} 
                />
                <div>
                  <div style={{ fontWeight: 500, fontSize: 16 }}>Device</div>
                  <Text type="secondary" style={{ fontSize: 14 }}>
                    An individual piece of equipment or device. Devices can have configurations 
                    and firmware associated with them (e.g., "PLC-001", "HMI-002").
                  </Text>
                </div>
              </Space>
            </Radio>
          </Card>
        </Space>
      </Radio.Group>
    </div>
  );
};

export default AssetTypeSelector;