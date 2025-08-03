import React, { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Input,
  Select,
  Button,
  Table,
  Space,
  Card,
  Typography,
  notification,
  Popconfirm,
  Tooltip,
  Tag,
  Divider,
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  DownloadOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { ImportTemplate, ImportTemplateConfig, validateSessionName } from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';

const { Title, Text } = Typography;
const { Option } = Select;
const { TextArea } = Input;

interface ImportTemplateManagerProps {
  visible: boolean;
  onClose: () => void;
  onSelectTemplate?: (template: ImportTemplate) => void;
  templateType?: string;
}

const ImportTemplateManager: React.FC<ImportTemplateManagerProps> = ({
  visible,
  onClose,
  onSelectTemplate,
  templateType = 'assets',
}) => {
  const [form] = Form.useForm();
  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<ImportTemplate | null>(null);
  const [selectedType, setSelectedType] = useState(templateType);

  const {
    templates,
    isLoading,
    error,
    loadTemplates,
    createTemplate,
    deleteTemplate,
    generateTemplateCSV,
    clearError,
  } = useBulkImportStore();

  useEffect(() => {
    if (visible) {
      loadTemplates(selectedType);
    }
  }, [visible, selectedType, loadTemplates]);

  useEffect(() => {
    if (error) {
      notification.error({
        message: 'Template Operation Failed',
        description: error,
      });
      clearError();
    }
  }, [error, clearError]);

  const handleCreateTemplate = async (values: any) => {
    try {
      const config: ImportTemplateConfig = {
        template_name: values.templateName,
        template_type: selectedType,
        asset_type: values.assetType,
        field_mapping: values.fieldMapping || {},
        required_fields: values.requiredFields || ['name', 'asset_type'],
        optional_fields: values.optionalFields || ['description', 'parent_name'],
        validation_rules: values.validationRules || {},
      };

      await createTemplate(config);
      setCreateModalVisible(false);
      form.resetFields();
      notification.success({
        message: 'Template Created',
        description: 'Import template created successfully',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleDeleteTemplate = async (templateId: number) => {
    try {
      await deleteTemplate(templateId);
      notification.success({
        message: 'Template Deleted',
        description: 'Import template deleted successfully',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleDownloadCSV = async (assetType: string, metadataSchema?: string) => {
    try {
      const csvContent = await generateTemplateCSV(assetType, metadataSchema);
      
      // Create and download file
      const blob = new Blob([csvContent], { type: 'text/csv' });
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `${assetType.toLowerCase()}_import_template.csv`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);

      notification.success({
        message: 'Template Downloaded',
        description: 'CSV template downloaded successfully',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleSelectTemplate = (template: ImportTemplate) => {
    if (onSelectTemplate) {
      onSelectTemplate(template);
      onClose();
    }
  };

  const columns = [
    {
      title: 'Template Name',
      dataIndex: 'template_name',
      key: 'template_name',
      render: (text: string, record: ImportTemplate) => (
        <Space direction="vertical" size={0}>
          <Text strong>{text}</Text>
          <Text type="secondary" style={{ fontSize: '12px' }}>
            Created: {new Date(record.created_at).toLocaleDateString()}
          </Text>
        </Space>
      ),
    },
    {
      title: 'Type',
      dataIndex: 'template_type',
      key: 'template_type',
      render: (type: string) => (
        <Tag color="blue">{type}</Tag>
      ),
    },
    {
      title: 'Required Fields',
      dataIndex: 'required_fields',
      key: 'required_fields',
      render: (fields: string[]) => (
        <Space wrap>
          {fields.slice(0, 3).map(field => (
            <Tag key={field} color="orange" size="small">
              {field}
            </Tag>
          ))}
          {fields.length > 3 && (
            <Tooltip title={fields.slice(3).join(', ')}>
              <Tag size="small">+{fields.length - 3} more</Tag>
            </Tooltip>
          )}
        </Space>
      ),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record: ImportTemplate) => (
        <Space>
          {onSelectTemplate && (
            <Button
              type="primary"
              size="small"
              onClick={() => handleSelectTemplate(record)}
            >
              Select
            </Button>
          )}
          <Tooltip title="Download CSV Template">
            <Button
              icon={<DownloadOutlined />}
              size="small"
              onClick={() => handleDownloadCSV(record.template_type)}
            />
          </Tooltip>
          <Tooltip title="Edit Template">
            <Button
              icon={<EditOutlined />}
              size="small"
              onClick={() => setEditingTemplate(record)}
            />
          </Tooltip>
          <Popconfirm
            title="Delete Template"
            description="Are you sure you want to delete this template?"
            onConfirm={() => handleDeleteTemplate(record.id)}
            okText="Delete"
            cancelText="Cancel"
            okType="danger"
          >
            <Button
              icon={<DeleteOutlined />}
              size="small"
              danger
            />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <Modal
      title="Import Template Manager"
      open={visible}
      onCancel={onClose}
      width={1000}
      footer={[
        <Button key="close" onClick={onClose}>
          Close
        </Button>,
      ]}
    >
      <Card>
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          {/* Header with controls */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <Title level={4} style={{ margin: 0 }}>
                Import Templates
              </Title>
              <Text type="secondary">
                Manage reusable templates for bulk import operations
              </Text>
            </div>
            <Space>
              <Select
                value={selectedType}
                onChange={setSelectedType}
                style={{ width: 150 }}
              >
                <Option value="assets">Assets</Option>
                <Option value="configurations">Configurations</Option>
                <Option value="metadata">Metadata</Option>
              </Select>
              <Button
                type="primary"
                icon={<PlusOutlined />}
                onClick={() => setCreateModalVisible(true)}
              >
                Create Template
              </Button>
              <Button
                icon={<DownloadOutlined />}
                onClick={() => handleDownloadCSV('Device')}
              >
                Download Basic CSV
              </Button>
            </Space>
          </div>

          <Divider />

          {/* Templates table */}
          <Table
            columns={columns}
            dataSource={templates}
            loading={isLoading}
            rowKey="id"
            pagination={{
              showSizeChanger: true,
              showQuickJumper: true,
              showTotal: (total, range) =>
                `${range[0]}-${range[1]} of ${total} templates`,
            }}
            locale={{
              emptyText: selectedType
                ? `No ${selectedType} templates found. Create your first template to get started.`
                : 'No templates found',
            }}
          />
        </Space>
      </Card>

      {/* Create Template Modal */}
      <Modal
        title="Create Import Template"
        open={createModalVisible}
        onCancel={() => {
          setCreateModalVisible(false);
          form.resetFields();
        }}
        onOk={() => form.submit()}
        confirmLoading={isLoading}
        width={600}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleCreateTemplate}
          initialValues={{
            requiredFields: ['name', 'asset_type'],
            optionalFields: ['description', 'parent_name'],
          }}
        >
          <Form.Item
            name="templateName"
            label="Template Name"
            rules={[
              { required: true, message: 'Please enter a template name' },
              {
                validator: (_, value) => {
                  const error = validateSessionName(value);
                  return error ? Promise.reject(error) : Promise.resolve();
                },
              },
            ]}
          >
            <Input placeholder="Enter template name" />
          </Form.Item>

          <Form.Item
            name="assetType"
            label="Default Asset Type"
            rules={[{ required: true, message: 'Please select an asset type' }]}
          >
            <Select placeholder="Select asset type">
              <Option value="Device">Device</Option>
              <Option value="Folder">Folder</Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="requiredFields"
            label={
              <Space>
                Required Fields
                <Tooltip title="Fields that must be present in the CSV file">
                  <InfoCircleOutlined />
                </Tooltip>
              </Space>
            }
          >
            <Select
              mode="tags"
              placeholder="Add required fields"
              style={{ width: '100%' }}
            >
              <Option value="name">name</Option>
              <Option value="asset_type">asset_type</Option>
              <Option value="description">description</Option>
              <Option value="parent_name">parent_name</Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="optionalFields"
            label={
              <Space>
                Optional Fields
                <Tooltip title="Fields that may be present but are not required">
                  <InfoCircleOutlined />
                </Tooltip>
              </Space>
            }
          >
            <Select
              mode="tags"
              placeholder="Add optional fields"
              style={{ width: '100%' }}
            >
              <Option value="description">description</Option>
              <Option value="parent_name">parent_name</Option>
              <Option value="metadata">metadata</Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="templateDescription"
            label="Description"
          >
            <TextArea
              rows={3}
              placeholder="Optional description for this template"
            />
          </Form.Item>
        </Form>
      </Modal>
    </Modal>
  );
};

export default ImportTemplateManager;