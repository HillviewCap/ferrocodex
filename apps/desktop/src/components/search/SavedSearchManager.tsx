import React, { useState, useEffect } from 'react';
import {
  List,
  Card,
  Button,
  Space,
  Tag,
  Modal,
  Input,
  Form,
  Typography,
  Tooltip,
  Popconfirm,
  Empty,
  Divider,
  Badge,
  notification,
} from 'antd';
import {
  SaveOutlined,
  PlayCircleOutlined,
  EditOutlined,
  DeleteOutlined,
  CopyOutlined,
  ShareAltOutlined,
  ClockCircleOutlined,
  FilterOutlined,
  SearchOutlined,
} from '@ant-design/icons';
import dayjs from 'dayjs';
import useSearchStore from '../../store/search';
import { searchUtils } from '../../store/search';
import { FilterPreset } from '../../types/search';

const { Text, Title, Paragraph } = Typography;
const { TextArea } = Input;

interface SavedSearchManagerProps {
  className?: string;
  onSearchSelect?: (preset: FilterPreset) => void;
}

const SavedSearchManager: React.FC<SavedSearchManagerProps> = ({
  className,
  onSearchSelect,
}) => {
  const {
    // State
    presets,
    query,
    
    // Actions
    loadPresets,
    savePreset,
    deletePreset,
    applyPreset,
  } = useSearchStore();

  const [editingPreset, setEditingPreset] = useState<FilterPreset | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [createForm] = Form.useForm();
  const [editForm] = Form.useForm();

  // Load presets on mount
  useEffect(() => {
    loadPresets();
  }, [loadPresets]);

  const handleCreatePreset = async (values: { name: string; description?: string }) => {
    if (query.filters.length === 0) {
      notification.warning({
        message: 'No Filters to Save',
        description: 'Please add some filters before creating a preset.',
      });
      return;
    }

    try {
      await savePreset({
        name: values.name,
        description: values.description || '',
      });
      
      notification.success({
        message: 'Preset Created',
        description: `"${values.name}" has been saved successfully.`,
      });
      
      setShowCreateModal(false);
      createForm.resetFields();
    } catch (error) {
      console.error('Failed to create preset:', error);
      notification.error({
        message: 'Creation Failed',
        description: error instanceof Error ? error.message : 'Failed to create preset',
      });
    }
  };

  const handleEditPreset = async (values: { name: string; description?: string }) => {
    if (!editingPreset) return;

    try {
      // Note: This would require an update preset command in the backend
      // For now, we'll delete and recreate
      await deletePreset(editingPreset.id!);
      await savePreset({
        name: values.name,
        description: values.description || '',
      });
      
      notification.success({
        message: 'Preset Updated',
        description: `"${values.name}" has been updated successfully.`,
      });
      
      setEditingPreset(null);
      editForm.resetFields();
    } catch (error) {
      console.error('Failed to update preset:', error);
      notification.error({
        message: 'Update Failed',
        description: error instanceof Error ? error.message : 'Failed to update preset',
      });
    }
  };

  const handleDeletePreset = async (presetId: number, presetName: string) => {
    try {
      await deletePreset(presetId);
      notification.success({
        message: 'Preset Deleted',
        description: `"${presetName}" has been deleted.`,
      });
    } catch (error) {
      console.error('Failed to delete preset:', error);
      notification.error({
        message: 'Delete Failed',
        description: error instanceof Error ? error.message : 'Failed to delete preset',
      });
    }
  };

  const handleApplyPreset = (preset: FilterPreset) => {
    applyPreset(preset);
    onSearchSelect?.(preset);
    notification.info({
      message: 'Preset Applied',
      description: `Filters from "${preset.name}" have been applied.`,
    });
  };

  const handleCopyPreset = (preset: FilterPreset) => {
    const presetData = {
      name: preset.name,
      description: preset.description,
      filters: preset.filters,
      created_at: preset.created_at,
    };
    
    navigator.clipboard.writeText(JSON.stringify(presetData, null, 2));
    notification.success({
      message: 'Preset Copied',
      description: 'Preset data has been copied to clipboard.',
    });
  };

  const renderPresetItem = (preset: FilterPreset) => {
    const filterDescription = searchUtils.buildQueryDescription({
      text_query: undefined,
      filters: preset.filters,
      hierarchy_scope: undefined,
      sort_by: undefined,
      limit: undefined,
      offset: undefined,
    });

    return (
      <List.Item
        key={preset.id}
        actions={[
          <Tooltip title="Apply this preset">
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              onClick={() => handleApplyPreset(preset)}
              size="small"
            >
              Apply
            </Button>
          </Tooltip>,
          <Tooltip title="Edit preset">
            <Button
              icon={<EditOutlined />}
              onClick={() => {
                setEditingPreset(preset);
                editForm.setFieldsValue({
                  name: preset.name,
                  description: preset.description,
                });
              }}
              size="small"
            />
          </Tooltip>,
          <Tooltip title="Copy preset">
            <Button
              icon={<CopyOutlined />}
              onClick={() => handleCopyPreset(preset)}
              size="small"
            />
          </Tooltip>,
          <Popconfirm
            title="Delete preset"
            description={`Are you sure you want to delete "${preset.name}"?`}
            onConfirm={() => handleDeletePreset(preset.id!, preset.name)}
            okText="Delete"
            cancelText="Cancel"
            okButtonProps={{ danger: true }}
          >
            <Tooltip title="Delete preset">
              <Button
                icon={<DeleteOutlined />}
                danger
                size="small"
              />
            </Tooltip>
          </Popconfirm>,
        ]}
      >
        <List.Item.Meta
          title={
            <Space>
              <Text strong>{preset.name}</Text>
              <Badge count={preset.filters.length} color="blue" />
              {preset.usage_count > 0 && (
                <Tag color="green">{preset.usage_count} uses</Tag>
              )}
            </Space>
          }
          description={
            <Space direction="vertical" style={{ width: '100%' }}>
              {preset.description && (
                <Text type="secondary">{preset.description}</Text>
              )}
              <div style={{ fontSize: '12px', color: '#666' }}>
                <FilterOutlined style={{ marginRight: 4 }} />
                {filterDescription}
              </div>
              <div style={{ fontSize: '12px', color: '#999' }}>
                <ClockCircleOutlined style={{ marginRight: 4 }} />
                Created {dayjs(preset.created_at).format('MMM D, YYYY [at] h:mm A')}
              </div>
            </Space>
          }
        />
      </List.Item>
    );
  };

  return (
    <div className={className}>
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Header Actions */}
        <Card size="small">
          <Space style={{ width: '100%', justifyContent: 'space-between' }}>
            <div>
              <Title level={4} style={{ margin: 0 }}>
                <SaveOutlined style={{ marginRight: 8 }} />
                Saved Searches
              </Title>
              <Text type="secondary">
                {presets.length} saved preset{presets.length !== 1 ? 's' : ''}
              </Text>
            </div>
            <Button
              type="primary"
              icon={<SaveOutlined />}
              onClick={() => setShowCreateModal(true)}
              disabled={query.filters.length === 0}
            >
              Save Current
            </Button>
          </Space>
        </Card>

        {/* Current Query Info */}
        {query.filters.length > 0 && (
          <Card size="small" title="Current Query">
            <Space direction="vertical" style={{ width: '100%' }}>
              <div>
                <Badge count={query.filters.length} color="blue" style={{ marginRight: 8 }} />
                <Text>Active filters</Text>
              </div>
              <div style={{ fontSize: '12px', color: '#666', fontFamily: 'monospace' }}>
                {searchUtils.buildQueryDescription(query)}
              </div>
            </Space>
          </Card>
        )}

        {/* Preset List */}
        <Card>
          {presets.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Space direction="vertical">
                  <Text>No saved searches yet</Text>
                  <Text type="secondary">
                    Create filters and save them for quick access
                  </Text>
                </Space>
              }
            />
          ) : (
            <List
              itemLayout="vertical"
              dataSource={presets}
              renderItem={renderPresetItem}
            />
          )}
        </Card>
      </Space>

      {/* Create Preset Modal */}
      <Modal
        title="Save Search Preset"
        open={showCreateModal}
        onCancel={() => {
          setShowCreateModal(false);
          createForm.resetFields();
        }}
        footer={null}
      >
        <Form
          form={createForm}
          layout="vertical"
          onFinish={handleCreatePreset}
        >
          <Form.Item
            name="name"
            label="Preset Name"
            rules={[
              { required: true, message: 'Please enter a preset name' },
              { max: 100, message: 'Name cannot exceed 100 characters' },
            ]}
          >
            <Input placeholder="Enter a descriptive name" />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description (Optional)"
            rules={[
              { max: 500, message: 'Description cannot exceed 500 characters' },
            ]}
          >
            <TextArea
              placeholder="Optional description of what this preset searches for"
              rows={3}
            />
          </Form.Item>

          <div style={{ marginBottom: 16 }}>
            <Text strong>Current Filters ({query.filters.length}):</Text>
            <div style={{ marginTop: 8, fontSize: '12px', color: '#666', fontFamily: 'monospace' }}>
              {searchUtils.buildQueryDescription(query)}
            </div>
          </div>

          <Form.Item style={{ marginBottom: 0 }}>
            <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
              <Button onClick={() => {
                setShowCreateModal(false);
                createForm.resetFields();
              }}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Save Preset
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* Edit Preset Modal */}
      <Modal
        title="Edit Search Preset"
        open={!!editingPreset}
        onCancel={() => {
          setEditingPreset(null);
          editForm.resetFields();
        }}
        footer={null}
      >
        <Form
          form={editForm}
          layout="vertical"
          onFinish={handleEditPreset}
        >
          <Form.Item
            name="name"
            label="Preset Name"
            rules={[
              { required: true, message: 'Please enter a preset name' },
              { max: 100, message: 'Name cannot exceed 100 characters' },
            ]}
          >
            <Input placeholder="Enter a descriptive name" />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description (Optional)"
            rules={[
              { max: 500, message: 'Description cannot exceed 500 characters' },
            ]}
          >
            <TextArea
              placeholder="Optional description of what this preset searches for"
              rows={3}
            />
          </Form.Item>

          {editingPreset && (
            <div style={{ marginBottom: 16 }}>
              <Text strong>Preset Filters ({editingPreset.filters.length}):</Text>
              <div style={{ marginTop: 8, fontSize: '12px', color: '#666', fontFamily: 'monospace' }}>
                {searchUtils.buildQueryDescription({
                  text_query: undefined,
                  filters: editingPreset.filters,
                  hierarchy_scope: undefined,
                  sort_by: undefined,
                  limit: undefined,
                  offset: undefined,
                })}
              </div>
            </div>
          )}

          <Form.Item style={{ marginBottom: 0 }}>
            <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
              <Button onClick={() => {
                setEditingPreset(null);
                editForm.resetFields();
              }}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Update Preset
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default SavedSearchManager;