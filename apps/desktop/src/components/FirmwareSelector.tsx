import React, { useEffect, useState } from 'react';
import { Select, Space, message, Tag } from 'antd';
import { RocketOutlined } from '@ant-design/icons';
import { FirmwareVersionInfo } from '../types/firmware';
import { invoke } from '@tauri-apps/api/core';

interface FirmwareSelectorProps {
  assetId: number;
  configId: number;
  currentFirmwareId?: number;
  token: string;
  onLink: (firmwareId: number) => void;
  onUnlink: () => void;
  disabled?: boolean;
}

const FirmwareSelector: React.FC<FirmwareSelectorProps> = ({
  assetId,
  configId,
  currentFirmwareId,
  token,
  onLink,
  onUnlink,
  disabled = false
}) => {
  const [firmwareVersions, setFirmwareVersions] = useState<FirmwareVersionInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [linking, setLinking] = useState(false);
  const [selectedFirmwareId, setSelectedFirmwareId] = useState<number | undefined>(currentFirmwareId);

  useEffect(() => {
    fetchFirmwareVersions();
  }, [assetId, token]);

  useEffect(() => {
    setSelectedFirmwareId(currentFirmwareId);
  }, [currentFirmwareId]);

  const fetchFirmwareVersions = async () => {
    setLoading(true);
    try {
      const versions = await invoke<FirmwareVersionInfo[]>('get_firmware_list', {
        token,
        assetId
      });
      setFirmwareVersions(versions);
    } catch (err) {
      console.error('Failed to fetch firmware versions:', err);
      message.error('Failed to load firmware versions');
    } finally {
      setLoading(false);
    }
  };

  const handleLink = async (firmwareId?: number) => {
    const targetFirmwareId = firmwareId ?? selectedFirmwareId;
    if (!targetFirmwareId) {
      message.warning('Please select a firmware version to link');
      return;
    }

    setLinking(true);
    try {
      await invoke('link_firmware_to_configuration', {
        token,
        configId,
        firmwareId: targetFirmwareId
      });
      message.success('Firmware linked successfully');
      onLink(targetFirmwareId);
    } catch (err) {
      console.error('Failed to link firmware:', err);
      message.error('Failed to link firmware: ' + (err as Error).message);
    } finally {
      setLinking(false);
    }
  };

  const handleUnlink = async () => {
    setLinking(true);
    try {
      await invoke('unlink_firmware_from_configuration', {
        token,
        configId
      });
      message.success('Firmware unlinked successfully');
      setSelectedFirmwareId(undefined);
      onUnlink();
    } catch (err) {
      console.error('Failed to unlink firmware:', err);
      message.error('Failed to unlink firmware: ' + (err as Error).message);
    } finally {
      setLinking(false);
    }
  };

  const formatFirmwareOption = (firmware: FirmwareVersionInfo) => {
    return (
      <Space>
        <RocketOutlined />
        <span>{firmware.version}</span>
        {firmware.vendor && <Tag color="blue">{firmware.vendor}</Tag>}
        {firmware.model && <Tag color="green">{firmware.model}</Tag>}
        <Tag color={firmware.status === 'Golden' ? 'gold' : 'default'}>
          {firmware.status}
        </Tag>
      </Space>
    );
  };

  const handleFirmwareChange = async (value: number | undefined) => {
    setSelectedFirmwareId(value);
    
    // If a firmware is selected and it's different from current, automatically link it
    if (value && value !== currentFirmwareId) {
      await handleLink(value);
    } else if (!value && currentFirmwareId) {
      // If cleared (undefined) and there was a current firmware, unlink it
      await handleUnlink();
    }
  };

  return (
    <Space size="small">
      <Select
        style={{ minWidth: 300 }}
        placeholder="Select firmware version"
        loading={loading}
        disabled={disabled || linking}
        value={selectedFirmwareId}
        onChange={handleFirmwareChange}
        allowClear
        showSearch
        filterOption={(input, option) => {
          const firmware = firmwareVersions.find(f => f.id === option?.value);
          if (!firmware) return false;
          const searchStr = `${firmware.version} ${firmware.vendor || ''} ${firmware.model || ''}`.toLowerCase();
          return searchStr.includes(input.toLowerCase());
        }}
      >
        {firmwareVersions?.map(firmware => (
          <Select.Option key={firmware.id} value={firmware.id}>
            {formatFirmwareOption(firmware)}
          </Select.Option>
        ))}
      </Select>
    </Space>
  );
};

export default FirmwareSelector;