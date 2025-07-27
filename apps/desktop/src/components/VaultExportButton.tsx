import React, { useState, useEffect } from 'react';
import { Button, Modal, Alert, Space, Typography, notification, Tooltip } from 'antd';
import { ExportOutlined, LockOutlined, WarningOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/api/fs';
import useAuthStore from '../store/auth';
import { VaultInfo, VaultAccessInfo } from '../types/vault';

const { Text } = Typography;

interface VaultExportButtonProps {
  vault: VaultInfo;
  buttonText?: string;
  buttonSize?: 'small' | 'middle' | 'large';
  showIcon?: boolean;
  onExportComplete?: () => void;
}

const VaultExportButton: React.FC<VaultExportButtonProps> = ({
  vault,
  buttonText = 'Export Vault',
  buttonSize = 'middle',
  showIcon = true,
  onExportComplete
}) => {
  const { token, user } = useAuthStore();
  const [hasExportPermission, setHasExportPermission] = useState(false);
  const [isAdministrator, setIsAdministrator] = useState(false);
  const [loading] = useState(false);
  const [exportModalVisible, setExportModalVisible] = useState(false);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    checkExportPermission();
  }, [vault.vault.id, token, user]);

  const checkExportPermission = async () => {
    if (!token || !user) return;

    try {
      const accessInfo = await invoke<VaultAccessInfo>('check_vault_access', {
        token,
        request: {
          user_id: user.id,
          vault_id: vault.vault.id,
          permission_type: 'Export'
        }
      });

      setHasExportPermission(accessInfo.has_access);
      setIsAdministrator(accessInfo.is_administrator);
    } catch (error) {
      console.error('Failed to check export permission:', error);
      setHasExportPermission(false);
    }
  };

  const handleExport = async () => {
    setExportModalVisible(true);
  };

  const performExport = async () => {
    if (!token || !user) return;

    setExporting(true);
    try {
      // Export vault data
      const exportData = await invoke<string>('export_vault', {
        token,
        vaultId: vault.vault.id
      });

      // Show save dialog
      const filePath = await save({
        defaultPath: `${vault.vault.name.replace(/[^a-z0-9]/gi, '_')}_vault_export.json`,
        filters: [{
          name: 'JSON files',
          extensions: ['json']
        }]
      });

      if (filePath) {
        // Write the export data to the selected file
        await writeTextFile(filePath, exportData);

        notification.success({
          message: 'Vault Exported',
          description: `Vault has been exported successfully to ${filePath}`
        });

        onExportComplete?.();
        setExportModalVisible(false);
      }
    } catch (error) {
      console.error('Failed to export vault:', error);
      
      if (error && typeof error === 'string' && error.includes('Access denied')) {
        notification.error({
          message: 'Export Denied',
          description: 'You do not have permission to export this vault.'
        });
      } else {
        notification.error({
          message: 'Export Failed',
          description: 'Failed to export vault. Please try again.'
        });
      }
    } finally {
      setExporting(false);
    }
  };

  // Determine button state
  const canExport = hasExportPermission || isAdministrator;
  const buttonDisabled = !canExport || loading;

  const getTooltipText = () => {
    if (isAdministrator) return 'Export vault (Administrator access)';
    if (hasExportPermission) return 'Export vault';
    return 'You do not have permission to export this vault';
  };

  return (
    <>
      <Tooltip title={getTooltipText()}>
        <Button
          size={buttonSize}
          icon={showIcon ? (canExport ? <ExportOutlined /> : <LockOutlined />) : undefined}
          onClick={handleExport}
          disabled={buttonDisabled}
          danger={!canExport}
        >
          {buttonText}
        </Button>
      </Tooltip>

      <Modal
        title="Export Vault"
        open={exportModalVisible}
        onCancel={() => setExportModalVisible(false)}
        footer={null}
        width={600}
      >
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          <Alert
            message="Security Warning"
            description={
              <div>
                <p>You are about to export sensitive vault data. This export will contain:</p>
                <ul style={{ marginLeft: '20px', marginTop: '8px' }}>
                  <li>All secret labels and metadata</li>
                  <li>Encrypted secret values (requires decryption key to read)</li>
                  <li>Vault configuration and history</li>
                </ul>
                <p style={{ marginTop: '12px', marginBottom: 0 }}>
                  <strong>Important:</strong> Handle the exported file with care. It contains sensitive information
                  and should be stored securely.
                </p>
              </div>
            }
            type="warning"
            icon={<WarningOutlined />}
            showIcon
          />

          <div style={{ background: '#f5f5f5', padding: '12px', borderRadius: '4px' }}>
            <Text strong>Vault to Export:</Text>
            <div style={{ marginTop: '8px' }}>
              <Text>{vault.vault.name}</Text>
              <br />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {vault.secret_count} secrets • {vault.secrets.length} items
              </Text>
            </div>
          </div>

          {isAdministrator && (
            <Alert
              message="Administrator Export"
              description="As an administrator, you have full export access to all vaults."
              type="info"
              showIcon
            />
          )}

          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button onClick={() => setExportModalVisible(false)}>
                Cancel
              </Button>
              <Button
                type="primary"
                icon={<ExportOutlined />}
                onClick={performExport}
                loading={exporting}
              >
                Export Vault
              </Button>
            </Space>
          </div>
        </Space>
      </Modal>
    </>
  );
};

export default VaultExportButton;