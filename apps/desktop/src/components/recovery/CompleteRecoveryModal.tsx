import React, { useState, useEffect } from 'react';
import { Modal, Select, Button, Alert, Steps, Divider, Typography, Space, Row, Col, Spin } from 'antd';
import { ExportOutlined, CheckCircleOutlined, CloseOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { 
  RecoveryManifest, 
  ExportStep,
  RecoveryDialogProps 
} from '../../types/recovery';
import useAuthStore from '../../store/auth';
import { message } from 'antd';

const { Option } = Select;
const { Text } = Typography;

interface ExportState {
  step: ExportStep;
  progress: number;
  message: string;
  timing?: {
    config_export_ms?: number;
    firmware_export_ms?: number;
    total_ms?: number;
  };
  error?: string;
  manifest?: RecoveryManifest;
}

export const CompleteRecoveryModal: React.FC<RecoveryDialogProps> = ({
  visible,
  onClose,
  assetId,
  assetName,
  configurationVersions,
  firmwareVersions,
}) => {
  const [selectedConfigId, setSelectedConfigId] = useState<number | undefined>();
  const [selectedFirmwareId, setSelectedFirmwareId] = useState<number | undefined>();
  const [exportDirectory, setExportDirectory] = useState<string>('');
  const [exportState, setExportState] = useState<ExportState>({
    step: 'selecting',
    progress: 0,
    message: 'Select versions and export directory',
  });
  const [isExporting, setIsExporting] = useState(false);
  const { token } = useAuthStore();

  // Reset state when modal opens/closes
  useEffect(() => {
    if (visible) {
      setSelectedConfigId(undefined);
      setSelectedFirmwareId(undefined);
      setExportDirectory('');
      setExportState({
        step: 'selecting',
        progress: 0,
        message: 'Select versions and export directory',
      });
      setIsExporting(false);
    }
  }, [visible]);

  // Auto-select if there's only one option for each
  useEffect(() => {
    if (configurationVersions.length === 1) {
      setSelectedConfigId(configurationVersions[0].id);
    }
    if (firmwareVersions.length === 1) {
      setSelectedFirmwareId(firmwareVersions[0].id);
    }
  }, [configurationVersions, firmwareVersions]);

  const handleSelectDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Select Export Directory',
      });
      
      if (selected && typeof selected === 'string') {
        setExportDirectory(selected);
      }
    } catch (error) {
      console.error('Error selecting directory:', error);
      message.error('Failed to select directory');
    }
  };

  const isLinkedVersion = (configId: number, firmwareId: number): boolean => {
    const config = configurationVersions.find(c => c.id === configId);
    return config?.firmware_version_id === firmwareId;
  };

  const canExport = (): boolean => {
    return !!(selectedConfigId && selectedFirmwareId && exportDirectory && !isExporting);
  };

  const handleExport = async () => {
    if (!canExport() || !token) return;

    setIsExporting(true);
    setExportState({
      step: 'exporting_config',
      progress: 25,
      message: 'Exporting configuration...',
    });

    try {
      // Prepare request for export
      // const request: RecoveryExportRequest = {
      //   asset_id: assetId,
      //   config_version_id: selectedConfigId!,
      //   firmware_version_id: selectedFirmwareId!,
      //   export_directory: exportDirectory,
      // };

      setExportState({
        step: 'exporting_firmware',
        progress: 50,
        message: 'Exporting firmware...',
      });

      const manifest = await invoke<RecoveryManifest>('export_complete_recovery', {
        token,
        assetId,
        configVersionId: selectedConfigId,
        firmwareVersionId: selectedFirmwareId,
        exportDirectory,
      });

      setExportState({
        step: 'completed',
        progress: 100,
        message: 'Export completed successfully!',
        manifest,
      });

      message.success('Complete recovery package exported successfully!');
      
      // Auto-close after 3 seconds on success
      setTimeout(() => {
        if (exportState.step === 'completed') {
          onClose();
        }
      }, 3000);

    } catch (error) {
      console.error('Export failed:', error);
      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
      
      setExportState({
        step: 'error',
        progress: 0,
        message: 'Export failed',
        error: errorMessage,
      });
      
      message.error(`Export failed: ${errorMessage}`);
    } finally {
      setIsExporting(false);
    }
  };

  const getStepStatus = (step: ExportStep): "error" | "wait" | "finish" | "process" => {
    const currentStepIndex = getStepIndex(exportState.step);
    const stepIndex = getStepIndex(step);
    
    if (exportState.step === 'error') return 'error';
    if (stepIndex < currentStepIndex) return 'finish';
    if (stepIndex === currentStepIndex) return 'process';
    return 'wait';
  };

  const getStepIndex = (step: ExportStep): number => {
    const steps = ['selecting', 'exporting_config', 'exporting_firmware', 'creating_manifest', 'completed'];
    return steps.indexOf(step);
  };

  const selectedConfig = configurationVersions.find(c => c.id === selectedConfigId);
  const selectedFirmware = firmwareVersions.find(f => f.id === selectedFirmwareId);
  const isCompatible = selectedConfigId && selectedFirmwareId 
    ? isLinkedVersion(selectedConfigId, selectedFirmwareId) 
    : false;

  const steps = [
    {
      title: 'Select Versions',
      status: getStepStatus('selecting'),
      icon: exportState.step === 'selecting' && isExporting ? <Spin size="small" /> : undefined,
    },
    {
      title: 'Export Config',
      status: getStepStatus('exporting_config'),
      icon: exportState.step === 'exporting_config' && isExporting ? <Spin size="small" /> : undefined,
    },
    {
      title: 'Export Firmware',
      status: getStepStatus('exporting_firmware'),
      icon: exportState.step === 'exporting_firmware' && isExporting ? <Spin size="small" /> : undefined,
    },
    {
      title: 'Create Manifest',
      status: getStepStatus('creating_manifest'),
      icon: exportState.step === 'creating_manifest' && isExporting ? <Spin size="small" /> : undefined,
    },
    {
      title: 'Complete',
      status: getStepStatus('completed'),
      icon: exportState.step === 'completed' ? <CheckCircleOutlined /> : undefined,
    },
  ];

  return (
    <Modal
      title={
        <Space>
          <ExportOutlined />
          Complete Recovery Export - {assetName}
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={800}
      footer={
        exportState.step === 'completed' ? (
          <Button onClick={onClose}>Close</Button>
        ) : exportState.step === 'error' ? (
          <Space>
            <Button onClick={onClose}>Close</Button>
            <Button type="primary" onClick={() => setExportState({ step: 'selecting', progress: 0, message: 'Select versions and export directory' })}>
              Try Again
            </Button>
          </Space>
        ) : (
          <Space>
            <Button onClick={onClose} disabled={isExporting}>Cancel</Button>
            <Button 
              type="primary" 
              onClick={handleExport}
              disabled={!canExport()}
              loading={isExporting}
              icon={<ExportOutlined />}
            >
              Export Recovery Package
            </Button>
          </Space>
        )
      }
    >
      <Steps current={getStepIndex(exportState.step)} items={steps} style={{ marginBottom: 24 }} />
      
      {exportState.step === 'error' && exportState.error && (
        <Alert
          message="Export Failed"
          description={exportState.error}
          type="error"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {exportState.step === 'completed' && exportState.manifest && (
        <Alert
          message="Export Completed Successfully"
          description={
            <div>
              <Text>Recovery package exported to: <Text code>{exportDirectory}</Text></Text>
              <br />
              <Text>Files created:</Text>
              <ul>
                <li>{exportState.manifest.configuration.filename}</li>
                <li>{exportState.manifest.firmware.filename}</li>
                <li>{assetName.replace(/[^a-zA-Z0-9]/g, '_')}_recovery_manifest.json</li>
              </ul>
              {exportState.timing?.total_ms && (
                <Text type="secondary">Export completed in {exportState.timing.total_ms}ms</Text>
              )}
            </div>
          }
          type="success"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {(exportState.step === 'selecting' || exportState.step === 'error') && (
        <>
          <Row gutter={16}>
            <Col span={12}>
              <div style={{ marginBottom: 16 }}>
                <Text strong>Configuration Version:</Text>
                <Select
                  style={{ width: '100%', marginTop: 8 }}
                  placeholder="Select configuration version"
                  value={selectedConfigId}
                  onChange={setSelectedConfigId}
                  disabled={isExporting}
                >
                  {configurationVersions.map(config => (
                    <Option key={config.id} value={config.id}>
                      <Space direction="vertical" size={0}>
                        <Text>{config.version_number}</Text>
                        <Text type="secondary" style={{ fontSize: '12px' }}>
                          {config.status} • {config.author_username}
                        </Text>
                      </Space>
                    </Option>
                  ))}
                </Select>
              </div>
            </Col>

            <Col span={12}>
              <div style={{ marginBottom: 16 }}>
                <Text strong>Firmware Version:</Text>
                <Select
                  style={{ width: '100%', marginTop: 8 }}
                  placeholder="Select firmware version"
                  value={selectedFirmwareId}
                  onChange={setSelectedFirmwareId}
                  disabled={isExporting}
                >
                  {firmwareVersions.map(firmware => (
                    <Option key={firmware.id} value={firmware.id}>
                      <Space direction="vertical" size={0}>
                        <Text>{firmware.version}</Text>
                        <Text type="secondary" style={{ fontSize: '12px' }}>
                          {firmware.status} • {firmware.vendor} {firmware.model}
                        </Text>
                      </Space>
                    </Option>
                  ))}
                </Select>
              </div>
            </Col>
          </Row>

          {selectedConfigId && selectedFirmwareId && (
            <Alert
              message={
                isCompatible ? (
                  <Space>
                    <CheckCircleOutlined style={{ color: '#52c41a' }} />
                    Versions are compatible (firmware is linked to this configuration)
                  </Space>
                ) : (
                  <Space>
                    <CloseOutlined style={{ color: '#ff4d4f' }} />
                    Versions are not linked (compatibility not verified)
                  </Space>
                )
              }
              type={isCompatible ? 'success' : 'warning'}
              style={{ marginBottom: 16 }}
            />
          )}

          <Divider />

          <div style={{ marginBottom: 16 }}>
            <Text strong>Export Directory:</Text>
            <Row gutter={8} style={{ marginTop: 8 }}>
              <Col flex={1}>
                <Select
                  style={{ width: '100%' }}
                  placeholder="Select export directory"
                  value={exportDirectory || undefined}
                  disabled={isExporting}
                  open={false}
                  onClick={handleSelectDirectory}
                >
                  {exportDirectory && (
                    <Option value={exportDirectory}>{exportDirectory}</Option>
                  )}
                </Select>
              </Col>
              <Col>
                <Button onClick={handleSelectDirectory} disabled={isExporting}>
                  Browse
                </Button>
              </Col>
            </Row>
          </div>

          {selectedConfig && selectedFirmware && (
            <div style={{ marginTop: 16 }}>
              <Text type="secondary">
                Selected: {selectedConfig.version_number} + {selectedFirmware.version}
              </Text>
            </div>
          )}
        </>
      )}

      {(exportState.step !== 'selecting' && exportState.step !== 'completed' && exportState.step !== 'error') && (
        <div style={{ textAlign: 'center', padding: '24px 0' }}>
          <Spin size="large" />
          <div style={{ marginTop: 16 }}>
            <Text>{exportState.message}</Text>
          </div>
        </div>
      )}
    </Modal>
  );
};