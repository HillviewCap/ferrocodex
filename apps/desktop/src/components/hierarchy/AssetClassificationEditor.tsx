import React, { useState } from 'react';
import { Alert, Button, Modal, Space, Typography, message } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import {
  AssetHierarchy,
  AssetInfo,
  countDescendantsBelow,
  findAssetInHierarchy,
} from '../../types/assets';
import { SecurityClassificationLevel, SECURITY_CLASSIFICATIONS } from '../../types/security';
import SecurityClassificationSelector from '../security/SecurityClassificationSelector';
import ClassificationTag from '../security/ClassificationTag';
import useAuthStore from '../../store/auth';

const { Text } = Typography;

export interface AssetClassificationEditorProps {
  asset: AssetHierarchy;
  hierarchyData: AssetHierarchy[];
  onUpdated?: (asset: AssetInfo) => void;
}

// Shows an asset's classification with a dialog to change it. The backend
// rejects levels below the parent folder and raises any descendants that would
// end up below a raised folder; the dialog states that count before saving.
export const AssetClassificationEditor: React.FC<AssetClassificationEditorProps> = ({
  asset,
  hierarchyData,
  onUpdated,
}) => {
  const { token, user } = useAuthStore();
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [level, setLevel] = useState<SecurityClassificationLevel>(asset.security_classification);

  const parent = asset.parent_id != null ? findAssetInHierarchy(hierarchyData, asset.parent_id) : null;
  const raiseCount = countDescendantsBelow(asset, level);
  const levelName = SECURITY_CLASSIFICATIONS[level]?.displayName ?? level;

  const handleOpen = () => {
    setLevel(asset.security_classification);
    setOpen(true);
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      const updated = await invoke<AssetInfo>('update_asset_classification', {
        token,
        assetId: asset.id,
        securityClassification: level,
      });
      message.success(
        raiseCount > 0
          ? `Classification set to ${levelName}; ${raiseCount} item${raiseCount !== 1 ? 's' : ''} raised`
          : `Classification set to ${levelName}`
      );
      setOpen(false);
      onUpdated?.(updated);
    } catch (error) {
      console.error('Failed to update classification:', error);
      message.error(typeof error === 'string' ? error : 'Failed to update classification');
    } finally {
      setSaving(false);
    }
  };

  const unchanged = level === asset.security_classification;

  return (
    <>
      <Space size={4}>
        <ClassificationTag level={asset.security_classification} />
        <Button type="link" size="small" icon={<EditOutlined />} onClick={handleOpen} style={{ padding: 0 }}>
          Change
        </Button>
      </Space>

      <Modal
        title={`Security Classification - ${asset.name}`}
        open={open}
        onCancel={() => setOpen(false)}
        onOk={handleSave}
        okText={raiseCount > 0 ? `Raise ${raiseCount} item${raiseCount !== 1 ? 's' : ''} and Save` : 'Save'}
        okButtonProps={{ disabled: unchanged }}
        confirmLoading={saving}
        destroyOnHidden
      >
        <Space orientation="vertical" style={{ width: '100%' }}>
          <SecurityClassificationSelector
            value={level}
            onChange={setLevel}
            currentUserRole={user?.role}
            inheritedClassification={parent?.security_classification}
            showDescription={false}
          />
          {raiseCount > 0 && (
            <Alert
              type="warning"
              showIcon
              title={`${raiseCount} item${raiseCount !== 1 ? 's' : ''} inside "${asset.name}" will be raised to ${levelName}`}
              description="Assets can't be classified below their folder, so contained items below the new level are raised with it."
            />
          )}
          {asset.asset_type === 'Folder' && !unchanged && raiseCount === 0 && (
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Items inside this folder keep their current classification.
            </Text>
          )}
        </Space>
      </Modal>
    </>
  );
};

export default AssetClassificationEditor;
