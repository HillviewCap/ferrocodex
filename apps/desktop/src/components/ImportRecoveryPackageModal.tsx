import React, { useState } from 'react';
import {
  Modal,
  Form,
  Button,
  message,
  Alert,
  Space,
  Typography,
  Select,
  Card,
  Divider,
  Checkbox,
  Tooltip,
  Tag,
  Progress,
  Result
} from 'antd';
import {
  UploadOutlined,
  FileOutlined,
  RocketOutlined,
  FolderOpenOutlined,
  CheckCircleOutlined,
  LockOutlined,
  InfoCircleOutlined,
  SafetyCertificateOutlined
} from '@ant-design/icons';
import * as dialog from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo } from '../types/assets';
import useAuthStore from '../store/auth';

const { Text, Title } = Typography;
const { Option } = Select;

interface RecoveryManifest {
  assetId: number;
  exportDate: string;
  exportedBy: string;
  configuration: {
    versionId: number;
    versionNumber: string;
    filename: string;
    checksum: string;
    fileSize: number;
  };
  firmware: {
    versionId: number;
    version: string;
    filename: string;
    checksum: string;
    vendor: string;
    model: string;
    fileSize: number;
  };
  vault?: {
    vaultId: number;
    vaultName: string;
    filename: string;
    checksum: string;
    secretCount: number;
    fileSize: number;
    encrypted: boolean;
  };
  compatibilityVerified: boolean;
}

interface ImportStep {
  step: 'SelectBundle' | 'ValidateBundle' | 'SelectAsset' | 'Importing' | 'Complete' | 'Error';
  progress?: number;
  message?: string;
}

interface ImportRecoveryPackageModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  assets: AssetInfo[];
}

const ImportRecoveryPackageModal: React.FC<ImportRecoveryPackageModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  assets
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [importStep, setImportStep] = useState<ImportStep>({ step: 'SelectBundle' });
  const [selectedPath, setSelectedPath] = useState<string>('');
  const [manifest, setManifest] = useState<RecoveryManifest | null>(null);
  const [selectedAssetId, setSelectedAssetId] = useState<number | null>(null);
  const [includeVault, setIncludeVault] = useState(true);
  const [importing, setImporting] = useState(false);

  const handleSelectBundle = async () => {
    try {
      const selected = await dialog.open({
        directory: true,
        multiple: false,
        title: 'Select Recovery Bundle Directory'
      });
      
      if (selected) {
        setSelectedPath(selected as string);
        validateBundle(selected as string);
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
      message.error('Failed to select directory');
    }
  };

  const validateBundle = async (bundlePath: string) => {
    setImportStep({ step: 'ValidateBundle', progress: 50, message: 'Validating bundle integrity...' });
    
    try {
      const validatedManifest = await invoke<RecoveryManifest>('validate_bundle_integrity', {
        token: token!,
        bundlePath
      });
      
      setManifest(validatedManifest);
      
      // Check if asset exists
      const matchingAsset = assets.find(a => a.id === validatedManifest.assetId);
      if (matchingAsset) {
        setSelectedAssetId(validatedManifest.assetId);
        setImportStep({ step: 'SelectAsset', message: 'Bundle validated successfully' });
      } else {
        setImportStep({ 
          step: 'SelectAsset', 
          message: `Bundle is for asset ID ${validatedManifest.assetId} which doesn't exist. Select target asset.` 
        });
      }
    } catch (err) {
      console.error('Failed to validate bundle:', err);
      setImportStep({ 
        step: 'Error', 
        message: `Bundle validation failed: ${(err as Error).message}` 
      });
    }
  };

  const handleImport = async () => {
    if (!selectedAssetId || !manifest) {
      message.error('Please select a target asset');
      return;
    }

    setImporting(true);
    setImportStep({ step: 'Importing', progress: 0, message: 'Starting import...' });

    try {
      // Simulate progress updates
      const progressInterval = setInterval(() => {
        setImportStep(prev => ({
          ...prev,
          progress: Math.min((prev.progress || 0) + 10, 90)
        }));
      }, 500);

      await invoke<RecoveryManifest>('import_recovery_bundle', {
        app: null,
        token: token!,
        bundlePath: selectedPath,
        targetAssetId: selectedAssetId,
        importVault: includeVault && !!manifest.vault
      });

      clearInterval(progressInterval);
      
      setImportStep({ step: 'Complete', progress: 100 });
      message.success('Recovery bundle imported successfully!');
      
      // Wait a moment before closing
      setTimeout(() => {
        onSuccess();
      }, 2000);
    } catch (err) {
      console.error('Failed to import recovery bundle:', err);
      setImportStep({ 
        step: 'Error', 
        message: `Import failed: ${(err as Error).message}` 
      });
      message.error(`Failed to import recovery bundle: ${(err as Error).message}`);
    } finally {
      setImporting(false);
    }
  };

  const renderStepContent = () => {
    switch (importStep.step) {
      case 'SelectBundle':
        return (
          <Space direction="vertical" size="large" style={{ width: '100%' }}>
            <Alert
              message="Import Recovery Bundle"
              description="Select a recovery bundle directory that was previously exported. The bundle must contain valid configuration, firmware, and manifest files."
              type="info"
              showIcon
            />
            
            <Button
              icon={<FolderOpenOutlined />}
              onClick={handleSelectBundle}
              size="large"
              style={{ width: '100%' }}
            >
              Select Recovery Bundle Directory
            </Button>
          </Space>
        );

      case 'ValidateBundle':
        return (
          <Space direction="vertical" size="large" style={{ width: '100%' }}>
            <Progress percent={importStep.progress || 0} status="active" />
            <Text>{importStep.message}</Text>
          </Space>
        );

      case 'SelectAsset':
        return (
          <Space direction="vertical" size="large" style={{ width: '100%' }}>
            {manifest && (
              <Card size="small">
                <Title level={5}>Bundle Contents</Title>
                
                <Space direction="vertical" size="small" style={{ width: '100%', marginTop: 16 }}>
                  <div>
                    <Text type="secondary">Exported By:</Text> {manifest.exportedBy}
                  </div>
                  <div>
                    <Text type="secondary">Export Date:</Text> {new Date(manifest.exportDate).toLocaleString()}
                  </div>
                  
                  <Divider style={{ margin: '12px 0' }} />
                  
                  <div>
                    <Space>
                      <FileOutlined style={{ color: '#1890ff' }} />
                      <Text strong>Configuration</Text>
                    </Space>
                    <div style={{ marginLeft: 24 }}>
                      <Space wrap>
                        <Tag color="blue">v{manifest.configuration.versionNumber}</Tag>
                        <Text type="secondary">{manifest.configuration.filename}</Text>
                      </Space>
                    </div>
                  </div>

                  <div>
                    <Space>
                      <RocketOutlined style={{ color: '#52c41a' }} />
                      <Text strong>Firmware</Text>
                    </Space>
                    <div style={{ marginLeft: 24 }}>
                      <Space wrap>
                        <Tag color="green">v{manifest.firmware.version}</Tag>
                        <Text type="secondary">{manifest.firmware.vendor} {manifest.firmware.model}</Text>
                      </Space>
                    </div>
                  </div>

                  {manifest.vault && (
                    <div>
                      <Space>
                        <LockOutlined style={{ color: '#fa8c16' }} />
                        <Text strong>Identity Vault</Text>
                      </Space>
                      <div style={{ marginLeft: 24 }}>
                        <Space wrap>
                          <Tag color="orange">{manifest.vault.secretCount} secrets</Tag>
                          <Text type="secondary">Encrypted</Text>
                        </Space>
                      </div>
                    </div>
                  )}

                  {manifest.compatibilityVerified && (
                    <div>
                      <Space>
                        <SafetyCertificateOutlined style={{ color: '#52c41a' }} />
                        <Text type="secondary">Configuration and firmware are verified compatible</Text>
                      </Space>
                    </div>
                  )}
                </Space>
              </Card>
            )}

            <Form form={form} layout="vertical">
              <Form.Item
                name="targetAsset"
                label="Target Asset"
                rules={[{ required: true, message: 'Please select target asset' }]}
              >
                <Select
                  placeholder="Select asset to import to"
                  value={selectedAssetId}
                  onChange={setSelectedAssetId}
                  showSearch
                  filterOption={(input, option) => {
                    const label = option?.label;
                    if (typeof label === 'string') {
                      return label.toLowerCase().includes(input.toLowerCase());
                    }
                    return false;
                  }}
                >
                  {assets.map(asset => (
                    <Option key={asset.id} value={asset.id}>
                      {asset.name}
                      {asset.id === manifest?.assetId && (
                        <Tag color="green" style={{ marginLeft: 8 }}>Original</Tag>
                      )}
                    </Option>
                  ))}
                </Select>
              </Form.Item>

              {manifest?.vault && (
                <Form.Item>
                  <Checkbox
                    checked={includeVault}
                    onChange={(e) => setIncludeVault(e.target.checked)}
                  >
                    <Space>
                      <LockOutlined />
                      Import Identity Vault
                      <Tooltip title="Import encrypted credentials and secrets">
                        <InfoCircleOutlined style={{ color: '#8c8c8c' }} />
                      </Tooltip>
                    </Space>
                  </Checkbox>
                </Form.Item>
              )}
            </Form>
          </Space>
        );

      case 'Importing':
        return (
          <Space direction="vertical" size="large" style={{ width: '100%', textAlign: 'center' }}>
            <Progress percent={importStep.progress || 0} status="active" />
            <Text>{importStep.message || 'Importing recovery bundle...'}</Text>
          </Space>
        );

      case 'Complete':
        return (
          <Result
            status="success"
            title="Import Successful!"
            subTitle="The recovery bundle has been imported successfully."
            icon={<CheckCircleOutlined style={{ color: '#52c41a' }} />}
          />
        );

      case 'Error':
        return (
          <Result
            status="error"
            title="Import Failed"
            subTitle={importStep.message}
          />
        );
    }
  };

  const getFooterButtons = () => {
    switch (importStep.step) {
      case 'SelectBundle':
      case 'Error':
        return [
          <Button key="cancel" onClick={onCancel}>
            Cancel
          </Button>
        ];
      
      case 'ValidateBundle':
      case 'Importing':
        return [
          <Button key="cancel" onClick={onCancel} disabled={importing}>
            Cancel
          </Button>
        ];
      
      case 'SelectAsset':
        return [
          <Button key="back" onClick={() => setImportStep({ step: 'SelectBundle' })}>
            Back
          </Button>,
          <Button key="cancel" onClick={onCancel}>
            Cancel
          </Button>,
          <Button
            key="import"
            type="primary"
            icon={<UploadOutlined />}
            onClick={handleImport}
            loading={importing}
            disabled={!selectedAssetId}
          >
            Import Bundle
          </Button>
        ];
      
      case 'Complete':
        return [
          <Button key="close" type="primary" onClick={onCancel}>
            Close
          </Button>
        ];
    }
  };

  return (
    <Modal
      title={
        <Space>
          <UploadOutlined />
          Import Recovery Bundle
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      width={600}
      footer={getFooterButtons()}
      closable={importStep.step !== 'Importing'}
      maskClosable={false}
    >
      {renderStepContent()}
    </Modal>
  );
};

export default ImportRecoveryPackageModal;