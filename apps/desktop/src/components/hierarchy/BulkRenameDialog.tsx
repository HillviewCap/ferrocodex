import React, { useState, useCallback, useEffect, useMemo } from 'react';
import {
  Modal,
  Form,
  Input,
  Radio,
  Switch,
  InputNumber,
  Space,
  Typography,
  List,
  Divider,
  Alert,
  Button,
  Tooltip,
  Tag
} from 'antd';
import {
  InfoCircleOutlined,
  EyeOutlined,
  EditOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import { AssetHierarchy } from '../../types/assets';
import { BulkRenameOptions, validateBulkRenameRequest } from '../../types/bulkOperations';
import useBulkOperationsStore from '../../store/bulkOperations';

const { Text, Title } = Typography;
const { TextArea } = Input;

interface BulkRenameDialogProps {
  visible: boolean;
  assets: AssetHierarchy[];
  selectedAssetIds: number[];
  onCancel: () => void;
  onConfirm: (options: BulkRenameOptions) => Promise<void>;
}

interface RenamePreview {
  assetId: number;
  originalName: string;
  newName: string;
  hasConflict: boolean;
  isValid: boolean;
  errorMessage?: string;
}

export const BulkRenameDialog: React.FC<BulkRenameDialogProps> = ({
  visible,
  assets,
  selectedAssetIds,
  onCancel,
  onConfirm
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [renameOptions, setRenameOptions] = useState<BulkRenameOptions>({
    pattern: '{name}',
    use_pattern: false,
    preserve_extension: true,
    start_number: 1,
    prefix: '',
    suffix: ''
  });

  const { validateBulkRename } = useBulkOperationsStore();

  // Get selected assets
  const selectedAssets = useMemo(() => {
    const findAsset = (assetList: AssetHierarchy[], id: number): AssetHierarchy | null => {
      for (const asset of assetList) {
        if (asset.id === id) return asset;
        if (asset.children) {
          const found = findAsset(asset.children, id);
          if (found) return found;
        }
      }
      return null;
    };

    return selectedAssetIds
      .map(id => findAsset(assets, id))
      .filter((asset): asset is AssetHierarchy => asset !== null);
  }, [assets, selectedAssetIds]);

  // Generate rename preview
  const renamePreview = useMemo((): RenamePreview[] => {
    const existingNames = new Set<string>();
    const previews: RenamePreview[] = [];

    // Collect all existing asset names for conflict detection
    const collectNames = (assetList: AssetHierarchy[]) => {
      for (const asset of assetList) {
        if (!selectedAssetIds.includes(asset.id)) {
          existingNames.add(asset.name.toLowerCase());
        }
        if (asset.children) {
          collectNames(asset.children);
        }
      }
    };
    collectNames(assets);

    selectedAssets.forEach((asset, index) => {
      let newName = asset.name;
      const originalName = asset.name;
      const fileExtension = originalName.includes('.') ? 
        originalName.substring(originalName.lastIndexOf('.')) : '';
      const baseName = originalName.includes('.') ? 
        originalName.substring(0, originalName.lastIndexOf('.')) : originalName;

      if (renameOptions.use_pattern && renameOptions.pattern) {
        // Apply pattern-based renaming
        const currentNumber = renameOptions.start_number + index;
        newName = renameOptions.pattern
          .replace(/\{name\}/g, baseName)
          .replace(/\{original\}/g, baseName)
          .replace(/\{number\}/g, currentNumber.toString())
          .replace(/\{number:(\d+)\}/g, (match, digits) => 
            currentNumber.toString().padStart(parseInt(digits, 10), '0')
          )
          .replace(/\{index\}/g, (index + 1).toString())
          .replace(/\{index:(\d+)\}/g, (match, digits) => 
            (index + 1).toString().padStart(parseInt(digits, 10), '0')
          );
      } else {
        // Apply prefix/suffix
        newName = baseName;
      }

      // Add prefix and suffix
      if (renameOptions.prefix) {
        newName = renameOptions.prefix + newName;
      }
      if (renameOptions.suffix) {
        newName = newName + renameOptions.suffix;
      }

      // Add extension back if preserving
      if (renameOptions.preserve_extension && fileExtension) {
        newName = newName + fileExtension;
      }

      // Validate new name
      const isValid = newName.trim().length > 0 && 
        !/[<>:"/\\|?*]/.test(newName) && // Invalid filename characters
        newName !== '.' && newName !== '..';

      const hasConflict = existingNames.has(newName.toLowerCase()) ||
        previews.some(p => p.newName.toLowerCase() === newName.toLowerCase());

      let errorMessage: string | undefined;
      if (!isValid) {
        if (newName.trim().length === 0) {
          errorMessage = 'Name cannot be empty';
        } else if (/[<>:"/\\|?*]/.test(newName)) {
          errorMessage = 'Name contains invalid characters';
        } else {
          errorMessage = 'Invalid name';
        }
      } else if (hasConflict) {
        errorMessage = 'Name conflicts with existing asset';
      }

      previews.push({
        assetId: asset.id,
        originalName,
        newName,
        hasConflict,
        isValid: isValid && !hasConflict,
        errorMessage
      });

      // Add new name to existing names to catch duplicates within the selection
      if (isValid && !hasConflict) {
        existingNames.add(newName.toLowerCase());
      }
    });

    return previews;
  }, [selectedAssets, selectedAssetIds, renameOptions, assets]);

  // Handle form changes
  const handleFormChange = useCallback(() => {
    const values = form.getFieldsValue();
    setRenameOptions(prev => ({
      ...prev,
      ...values
    }));
  }, [form]);

  // Handle confirm
  const handleConfirm = useCallback(async () => {
    try {
      setLoading(true);

      // Validate all previews
      const invalidPreviews = renamePreview.filter(p => !p.isValid);
      if (invalidPreviews.length > 0) {
        Modal.error({
          title: 'Validation Error',
          content: `${invalidPreviews.length} asset(s) have invalid names. Please fix the issues before proceeding.`
        });
        return;
      }

      // Backend validation
      const validationResult = await validateBulkRename(selectedAssetIds, renameOptions);
      if (!validationResult.is_valid) {
        Modal.error({
          title: 'Validation Failed',
          content: validationResult.errors.map(e => e.message).join('\n')
        });
        return;
      }

      await onConfirm(renameOptions);
    } catch (error) {
      console.error('Bulk rename failed:', error);
    } finally {
      setLoading(false);
    }
  }, [renameOptions, renamePreview, selectedAssetIds, validateBulkRename, onConfirm]);

  // Reset form when modal opens
  useEffect(() => {
    if (visible) {
      form.resetFields();
      setRenameOptions({
        pattern: '{name}',
        use_pattern: false,
        preserve_extension: true,
        start_number: 1,
        prefix: '',
        suffix: ''
      });
      setShowPreview(false);
    }
  }, [visible, form]);

  // Pattern examples
  const patternExamples = [
    { pattern: '{name}_{number}', description: 'Add sequential numbers' },
    { pattern: '{name}_{number:3}', description: 'Add zero-padded numbers (001, 002, etc.)' },
    { pattern: 'Asset_{index:2}', description: 'Replace with sequential naming' },
    { pattern: '{name}_backup', description: 'Add suffix to all names' },
    { pattern: 'new_{original}', description: 'Add prefix to all names' }
  ];

  const validPreviews = renamePreview.filter(p => p.isValid).length;
  const totalPreviews = renamePreview.length;

  return (
    <Modal
      title={
        <Space>
          <EditOutlined />
          <span>Bulk Rename Assets</span>
          <Tag color="blue">{selectedAssetIds.length} assets</Tag>
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      width={800}
      footer={[
        <Button key="cancel" onClick={onCancel}>
          Cancel
        </Button>,
        <Button
          key="preview"
          icon={<EyeOutlined />}
          onClick={() => setShowPreview(!showPreview)}
        >
          {showPreview ? 'Hide Preview' : 'Show Preview'}
        </Button>,
        <Button
          key="confirm"
          type="primary"
          loading={loading}
          disabled={validPreviews !== totalPreviews}
          onClick={handleConfirm}
        >
          Rename {validPreviews} Asset{validPreviews !== 1 ? 's' : ''}
        </Button>
      ]}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        {/* Summary */}
        <Alert
          message={`Renaming ${selectedAssetIds.length} selected assets`}
          description={
            showPreview && validPreviews !== totalPreviews ?
              `${validPreviews} valid, ${totalPreviews - validPreviews} invalid` :
              'Configure your renaming options below'
          }
          type={showPreview && validPreviews !== totalPreviews ? 'warning' : 'info'}
          showIcon
        />

        {/* Form */}
        <Form
          form={form}
          layout="vertical"
          initialValues={renameOptions}
          onValuesChange={handleFormChange}
        >
          <Form.Item
            name="use_pattern"
            valuePropName="checked"
          >
            <Switch
              checkedChildren="Pattern Mode"
              unCheckedChildren="Simple Mode"
              onChange={(checked) => setRenameOptions(prev => ({ ...prev, use_pattern: checked }))}
            />
          </Form.Item>

          {renameOptions.use_pattern ? (
            <>
              <Form.Item
                label={
                  <Space>
                    <span>Rename Pattern</span>
                    <Tooltip title="Use {name}, {original}, {number}, {number:3}, {index}, {index:2}">
                      <InfoCircleOutlined />
                    </Tooltip>
                  </Space>
                }
                name="pattern"
                rules={[{ required: true, message: 'Pattern is required' }]}
              >
                <Input
                  placeholder="e.g., {name}_{number:3}"
                  onChange={(e) => setRenameOptions(prev => ({ ...prev, pattern: e.target.value }))}
                />
              </Form.Item>

              <Form.Item
                label="Starting Number"
                name="start_number"
              >
                <InputNumber
                  min={0}
                  max={9999}
                  onChange={(value) => setRenameOptions(prev => ({ ...prev, start_number: value || 1 }))}
                />
              </Form.Item>

              {/* Pattern Examples */}
              <Form.Item label="Pattern Examples">
                <List
                  size="small"
                  dataSource={patternExamples}
                  renderItem={(example) => (
                    <List.Item
                      style={{ cursor: 'pointer', padding: '4px 8px' }}
                      onClick={() => {
                        form.setFieldValue('pattern', example.pattern);
                        setRenameOptions(prev => ({ ...prev, pattern: example.pattern }));
                      }}
                    >
                      <Space>
                        <Text code>{example.pattern}</Text>
                        <Text type="secondary">{example.description}</Text>
                      </Space>
                    </List.Item>
                  )}
                />
              </Form.Item>
            </>
          ) : (
            <>
              <Form.Item
                label="Prefix"
                name="prefix"
              >
                <Input
                  placeholder="Text to add before each name"
                  onChange={(e) => setRenameOptions(prev => ({ ...prev, prefix: e.target.value }))}
                />
              </Form.Item>

              <Form.Item
                label="Suffix"
                name="suffix"
              >
                <Input
                  placeholder="Text to add after each name"
                  onChange={(e) => setRenameOptions(prev => ({ ...prev, suffix: e.target.value }))}
                />
              </Form.Item>
            </>
          )}

          <Form.Item
            name="preserve_extension"
            valuePropName="checked"
          >
            <Switch
              checkedChildren="Preserve Extensions"
              unCheckedChildren="Include in Rename"
              onChange={(checked) => setRenameOptions(prev => ({ ...prev, preserve_extension: checked }))}
            />
          </Form.Item>
        </Form>

        {/* Preview */}
        {showPreview && (
          <>
            <Divider>Preview</Divider>
            <div style={{ maxHeight: '300px', overflowY: 'auto' }}>
              <List
                size="small"
                dataSource={renamePreview}
                renderItem={(preview) => (
                  <List.Item
                    style={{
                      backgroundColor: preview.isValid ? '#f6ffed' : '#fff2f0',
                      border: `1px solid ${preview.isValid ? '#b7eb8f' : '#ffccc7'}`,
                      borderRadius: '4px',
                      marginBottom: '4px',
                      padding: '8px 12px'
                    }}
                  >
                    <Space direction="vertical" size={2} style={{ width: '100%' }}>
                      <Space justify="space-between" style={{ width: '100%' }}>
                        <Space>
                          {preview.isValid ? (
                            <CheckCircleOutlined style={{ color: '#52c41a' }} />
                          ) : (
                            <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />
                          )}
                          <Text delete={!preview.isValid}>{preview.originalName}</Text>
                          <span>→</span>
                          <Text strong={preview.isValid} type={preview.isValid ? 'success' : 'danger'}>
                            {preview.newName}
                          </Text>
                        </Space>
                      </Space>
                      {preview.errorMessage && (
                        <Text type="danger" style={{ fontSize: '12px' }}>
                          {preview.errorMessage}
                        </Text>
                      )}
                    </Space>
                  </List.Item>
                )}
              />
            </div>
          </>
        )}
      </Space>
    </Modal>
  );
};

export default BulkRenameDialog;