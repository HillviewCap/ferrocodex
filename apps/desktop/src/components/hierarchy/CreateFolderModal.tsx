import React, { useState, useEffect, useMemo } from 'react';
import { Modal, Form, Input, message } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import {
  AssetHierarchy,
  AssetInfo,
  getInheritedClassification,
  hierarchyHasFolders,
} from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import { AssetHierarchyPicker } from './AssetHierarchyPicker';
import SecurityClassificationSelector from '../security/SecurityClassificationSelector';
import useAuthStore from '../../store/auth';

const { TextArea } = Input;

export interface CreateFolderModalProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: (folder: AssetInfo) => void;
  hierarchyData: AssetHierarchy[];
  initialParentId?: number | null;
}

interface FolderFormValues {
  name: string;
  description?: string;
}

export const CreateFolderModal: React.FC<CreateFolderModalProps> = ({
  open,
  onCancel,
  onSuccess,
  hierarchyData,
  initialParentId = null,
}) => {
  const { token, user } = useAuthStore();
  const [form] = Form.useForm<FolderFormValues>();
  const [loading, setLoading] = useState(false);
  const hasFolders = useMemo(() => hierarchyHasFolders(hierarchyData), [hierarchyData]);
  const [parentId, setParentId] = useState<number | null>(initialParentId);
  const [classification, setClassification] = useState<SecurityClassificationLevel>(
    getInheritedClassification(hierarchyData, initialParentId)
  );

  // Start from the folder the user invoked this from, with its classification
  useEffect(() => {
    if (open) {
      form.resetFields();
      const startParent = hasFolders ? initialParentId : null;
      setParentId(startParent);
      setClassification(getInheritedClassification(hierarchyData, startParent));
    }
    // hierarchyData is deliberately read only when the modal opens, so a
    // background refresh doesn't reset what the user has chosen
  }, [open, initialParentId, form]);

  const handleParentChange = (newParentId: number | null) => {
    setParentId(newParentId);
    setClassification(getInheritedClassification(hierarchyData, newParentId));
  };

  // Only a parent folder constrains the level; at the root anything goes
  const inheritedClassification = parentId != null
    ? getInheritedClassification(hierarchyData, parentId)
    : undefined;

  const handleSubmit = async (values: FolderFormValues) => {
    try {
      setLoading(true);
      const folder = await invoke<AssetInfo>('create_asset', {
        token,
        name: values.name.trim(),
        description: values.description?.trim() || '',
        assetType: 'folder',
        parentId,
        securityClassification: classification,
      });
      message.success(`Folder "${folder.name}" created`);
      onSuccess(folder);
    } catch (error) {
      console.error('Failed to create folder:', error);
      message.error(typeof error === 'string' ? error : error instanceof Error ? error.message : 'Failed to create folder');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title="Create Folder"
      open={open}
      onCancel={onCancel}
      onOk={() => form.submit()}
      okText="Create Folder"
      confirmLoading={loading}
      width={600}
      mask={{ closable: false }}
    >
      <Form form={form} layout="vertical" onFinish={handleSubmit}>
        <Form.Item
          name="name"
          label="Folder Name"
          rules={[
            { required: true, whitespace: true, message: 'Please enter a folder name' },
            { min: 2, message: 'Folder name must be at least 2 characters' },
          ]}
        >
          <Input placeholder="e.g. Production Line 1" maxLength={100} showCount autoFocus />
        </Form.Item>

        <Form.Item name="description" label="Description">
          <TextArea rows={3} placeholder="What this folder groups (optional)" maxLength={500} showCount />
        </Form.Item>

        {hasFolders && (
          <Form.Item label="Parent Folder">
            <AssetHierarchyPicker
              hierarchyData={hierarchyData}
              value={parentId}
              onChange={handleParentChange}
              placeholder="Root level (no parent folder)"
              allowClear
            />
          </Form.Item>
        )}

        <Form.Item label="Security Classification" required>
          <SecurityClassificationSelector
            value={classification}
            onChange={setClassification}
            currentUserRole={user?.role}
            inheritedClassification={inheritedClassification}
            showDescription={false}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default CreateFolderModal;
