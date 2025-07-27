import React, { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Button,
  message,
  Alert,
  Space,
  Typography,
  Spin,
  Tag,
  Card,
  Divider,
  Checkbox,
  Tooltip
} from 'antd';
import {
  DownloadOutlined,
  FileOutlined,
  RocketOutlined,
  FolderOpenOutlined,
  CheckCircleOutlined,
  LockOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import * as dialog from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { ConfigurationVersionInfo } from '../types/assets';
import { FirmwareVersionInfo } from '../types/firmware';
import useAuthStore from '../store/auth';

const { Text, Title } = Typography;

interface ExportOptions {
  includeVault: boolean;
  vaultAvailable: boolean;
  vaultSecretCount: number;
}

interface ExportRecoveryPackageModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: (manifestPath: string) => void;
  assetId: number;
  assetName: string;
  configuration: ConfigurationVersionInfo;
  linkedFirmwareId?: number;
}

const ExportRecoveryPackageModal: React.FC<ExportRecoveryPackageModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  assetId,
  assetName: _assetName,
  configuration,
  linkedFirmwareId
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [exporting, setExporting] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string>('');
  const [firmwareInfo, setFirmwareInfo] = useState<FirmwareVersionInfo | null>(null);
  const [loadingFirmware, setLoadingFirmware] = useState(false);
  const [exportOptions, setExportOptions] = useState<ExportOptions | null>(null);
  const [includeVault, setIncludeVault] = useState(false);
  const [_loadingOptions, setLoadingOptions] = useState(false);

  useEffect(() => {
    if (visible) {
      if (linkedFirmwareId) {
        fetchFirmwareInfo();
      }
      fetchExportOptions();
    }
  }, [visible, linkedFirmwareId]);

  const fetchFirmwareInfo = async () => {
    if (!token || !linkedFirmwareId) return;
    
    setLoadingFirmware(true);
    try {
      // Get all firmware for the asset
      const firmwareList = await invoke<FirmwareVersionInfo[]>('get_firmware_list', {
        token,
        assetId
      });
      
      // Find the linked firmware
      const firmware = firmwareList.find(f => f.id === linkedFirmwareId);
      if (firmware) {
        setFirmwareInfo(firmware);
      }
    } catch (err) {
      console.error('Failed to fetch firmware info:', err);
    } finally {
      setLoadingFirmware(false);
    }
  };

  const fetchExportOptions = async () => {
    if (!token) return;
    
    setLoadingOptions(true);
    try {
      const options = await invoke<ExportOptions>('get_export_options', {
        token,
        assetId
      });
      
      setExportOptions(options);
      setIncludeVault(options.includeVault);
    } catch (err) {
      console.error('Failed to fetch export options:', err);
    } finally {
      setLoadingOptions(false);
    }
  };

  const handleSelectPath = async () => {
    try {
      const selected = await dialog.open({
        directory: true,
        multiple: false,
        title: 'Select Export Directory'
      });
      
      if (selected) {
        setSelectedPath(selected as string);
        form.setFieldsValue({ exportPath: selected as string });
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
      message.error('Failed to select directory');
    }
  };

  const handleExport = async () => {
    if (!linkedFirmwareId) {
      message.error('No firmware is linked to this configuration');
      return;
    }

    if (!selectedPath) {
      message.error('Please select an export directory');
      return;
    }

    setExporting(true);
    try {
      const manifestPath = await invoke<string>('export_complete_recovery', {
        app: null, // Will be injected by Tauri
        token: token!,
        assetId,
        configVersionId: configuration.id,
        firmwareVersionId: linkedFirmwareId,
        exportDirectory: selectedPath,
        includeVault: includeVault && exportOptions?.vaultAvailable
      });
      
      message.success('Recovery package exported successfully!');
      onSuccess(manifestPath);
    } catch (err) {
      console.error('Failed to export recovery package:', err);
      message.error(`Failed to export recovery package: ${(err as Error).message}`);
    } finally {
      setExporting(false);
    }
  };

  const canExport = linkedFirmwareId && selectedPath && !exporting;

  return (
    <Modal
      title={
        <Space>
          <DownloadOutlined />
          Export Recovery Package
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      width={600}
      footer={[
        <Button key="cancel" onClick={onCancel}>
          Cancel
        </Button>,
        <Button
          key="export"
          type="primary"
          icon={<DownloadOutlined />}
          onClick={handleExport}
          loading={exporting}
          disabled={!canExport}
        >
          Export Package
        </Button>
      ]}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Alert
          message="Recovery Package Export"
          description="Export a complete recovery package containing both configuration and firmware files. This package can be used to restore the asset to a known working state."
          type="info"
          showIcon
        />

        <Card size="small">
          <Title level={5}>Package Contents</Title>
          
          <Space direction="vertical" size="small" style={{ width: '100%', marginTop: 16 }}>
            <div>
              <Space>
                <FileOutlined style={{ color: '#1890ff' }} />
                <Text strong>Configuration</Text>
              </Space>
              <div style={{ marginLeft: 24 }}>
                <Space wrap>
                  <Tag color="blue">{configuration.version_number}</Tag>
                  <Text type="secondary">{configuration.file_name}</Text>
                  <Tag color={configuration.status === 'Golden' ? 'gold' : 'default'}>
                    {configuration.status}
                  </Tag>
                </Space>
              </div>
            </div>

            <Divider style={{ margin: '12px 0' }} />

            <div>
              <Space>
                <RocketOutlined style={{ color: '#52c41a' }} />
                <Text strong>Firmware</Text>
              </Space>
              <div style={{ marginLeft: 24 }}>
                {loadingFirmware ? (
                  <Spin size="small" />
                ) : linkedFirmwareId ? (
                  firmwareInfo ? (
                    <Space wrap>
                      <Tag color="green">{firmwareInfo.version}</Tag>
                      {firmwareInfo.vendor && <Text type="secondary">{firmwareInfo.vendor}</Text>}
                      {firmwareInfo.model && <Text type="secondary">{firmwareInfo.model}</Text>}
                      <Tag color={firmwareInfo.status === 'Golden' ? 'gold' : 'default'}>
                        {firmwareInfo.status}
                      </Tag>
                    </Space>
                  ) : (
                    <Text type="secondary">Firmware information not available</Text>
                  )
                ) : (
                  <Alert
                    message="No firmware linked"
                    description="This configuration does not have a linked firmware version. Link a firmware version first to create a recovery package."
                    type="warning"
                    showIcon
                  />
                )}
              </div>
            </div>

            {exportOptions?.vaultAvailable && (
              <>
                <Divider style={{ margin: '12px 0' }} />
                <div>
                  <Space>
                    <LockOutlined style={{ color: '#fa8c16' }} />
                    <Text strong>Identity Vault</Text>
                    <Tooltip title="Contains encrypted credentials and secrets for this asset">
                      <InfoCircleOutlined style={{ color: '#8c8c8c' }} />
                    </Tooltip>
                  </Space>
                  <div style={{ marginLeft: 24 }}>
                    <Space wrap>
                      <Tag color="orange">{exportOptions.vaultSecretCount} secrets</Tag>
                      <Text type="secondary">Encrypted credentials and passwords</Text>
                    </Space>
                  </div>
                </div>
              </>
            )}

            <Divider style={{ margin: '12px 0' }} />

            <div>
              <Space>
                <CheckCircleOutlined style={{ color: '#52c41a' }} />
                <Text strong>Manifest File</Text>
              </Space>
              <div style={{ marginLeft: 24 }}>
                <Text type="secondary">
                  recovery_manifest.json - Contains metadata and checksums for verification
                </Text>
              </div>
            </div>
          </Space>
        </Card>

        <Form form={form} layout="vertical">
          <Form.Item
            name="exportPath"
            label="Export Directory"
            rules={[{ required: true, message: 'Please select an export directory' }]}
          >
            <Button
              icon={<FolderOpenOutlined />}
              onClick={handleSelectPath}
              style={{ width: '100%', textAlign: 'left' }}
            >
              {selectedPath || 'Click to select export directory...'}
            </Button>
          </Form.Item>

          {exportOptions?.vaultAvailable && (
            <Form.Item>
              <Checkbox
                checked={includeVault}
                onChange={(e) => setIncludeVault(e.target.checked)}
              >
                <Space>
                  <LockOutlined />
                  Include Identity Vault
                  <Tooltip title="Export encrypted credentials and secrets with the recovery package">
                    <InfoCircleOutlined style={{ color: '#8c8c8c' }} />
                  </Tooltip>
                </Space>
              </Checkbox>
              {includeVault && (
                <Alert
                  message="Security Notice"
                  description="The vault data will remain encrypted during export. Only users with appropriate permissions can access the vault contents after import."
                  type="warning"
                  showIcon
                  style={{ marginTop: 8 }}
                />
              )}
            </Form.Item>
          )}
        </Form>

        {selectedPath && (
          <Alert
            message="Selected Directory"
            description={selectedPath}
            type="success"
            showIcon
          />
        )}
      </Space>
    </Modal>
  );
};

export default ExportRecoveryPackageModal;