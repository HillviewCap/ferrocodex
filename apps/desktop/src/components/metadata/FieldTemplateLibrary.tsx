import React, { useState, useEffect } from 'react';
import { Card, Row, Col, Typography, Tag, Input, Space, Spin, message, Tooltip, Badge, Button, Modal, Divider } from 'antd';
import { SearchOutlined, AppstoreOutlined, ApiOutlined, SettingOutlined, SafetyOutlined, DashboardOutlined, CopyOutlined, EyeOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';

const { Title, Text, Paragraph } = Typography;
const { Search } = Input;

interface FieldTemplate {
  id: string;
  name: string;
  display_name: string;
  description: string;
  category: string;
  data_type: string;
  validation_rules?: any;
  options?: string[];
  constraints?: any;
  unit?: string;
  popularity?: number;
  usage_count?: number;
}

interface TemplateCategory {
  key: string;
  label: string;
  icon: React.ReactNode;
  color: string;
}

const templateCategories: TemplateCategory[] = [
  { key: 'network', label: 'Network', icon: <ApiOutlined />, color: '#1890ff' },
  { key: 'physical', label: 'Physical', icon: <AppstoreOutlined />, color: '#52c41a' },
  { key: 'device', label: 'Device', icon: <SettingOutlined />, color: '#fa8c16' },
  { key: 'operational', label: 'Operational', icon: <DashboardOutlined />, color: '#722ed1' },
  { key: 'security', label: 'Security', icon: <SafetyOutlined />, color: '#f5222d' },
];

const FieldTemplateLibrary: React.FC = () => {
  const { token } = useAuthStore();
  const [templates, setTemplates] = useState<FieldTemplate[]>([]);
  const [filteredTemplates, setFilteredTemplates] = useState<FieldTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchText, setSearchText] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [previewTemplate, setPreviewTemplate] = useState<FieldTemplate | null>(null);

  useEffect(() => {
    loadTemplates();
  }, [token]);

  useEffect(() => {
    filterTemplates();
  }, [templates, searchText, selectedCategory]);

  const loadTemplates = async () => {
    if (!token) return;
    
    setLoading(true);
    try {
      const result = await invoke<FieldTemplate[]>('list_metadata_field_templates', { token });
      
      // Add usage statistics (mock data for now, could be fetched from backend)
      const templatesWithStats = result.map(template => ({
        ...template,
        popularity: Math.floor(Math.random() * 100) + 1,
        usage_count: Math.floor(Math.random() * 500) + 10
      }));
      
      setTemplates(templatesWithStats);
    } catch (error) {
      console.error('Failed to load field templates:', error);
      message.error('Failed to load field templates');
    } finally {
      setLoading(false);
    }
  };

  const filterTemplates = () => {
    let filtered = [...templates];
    
    // Filter by category
    if (selectedCategory) {
      filtered = filtered.filter(t => t.category === selectedCategory);
    }
    
    // Filter by search text
    if (searchText) {
      const search = searchText.toLowerCase();
      filtered = filtered.filter(t => 
        t.name.toLowerCase().includes(search) ||
        t.display_name.toLowerCase().includes(search) ||
        t.description.toLowerCase().includes(search)
      );
    }
    
    // Sort by popularity
    filtered.sort((a, b) => (b.popularity || 0) - (a.popularity || 0));
    
    setFilteredTemplates(filtered);
  };

  const getCategoryConfig = (category: string) => {
    return templateCategories.find(c => c.key === category) || templateCategories[0];
  };

  const getDataTypeColor = (dataType: string) => {
    const typeColors: Record<string, string> = {
      'string': '#1890ff',
      'number': '#52c41a',
      'boolean': '#fa8c16',
      'array': '#722ed1',
      'object': '#eb2f96'
    };
    return typeColors[dataType] || '#8c8c8c';
  };

  const copyToClipboard = (template: FieldTemplate) => {
    const templateJson = JSON.stringify({
      name: template.name,
      display_name: template.display_name,
      description: template.description,
      data_type: template.data_type,
      validation_rules: template.validation_rules,
      options: template.options,
      constraints: template.constraints,
      unit: template.unit
    }, null, 2);
    
    navigator.clipboard.writeText(templateJson);
    message.success('Template copied to clipboard');
  };

  const showPreview = (template: FieldTemplate) => {
    setPreviewTemplate(template);
  };

  const renderTemplateCard = (template: FieldTemplate) => {
    const category = getCategoryConfig(template.category);
    
    return (
      <Card
        hoverable
        style={{ height: '100%' }}
        actions={[
          <Tooltip title="Preview">
            <Button type="text" icon={<EyeOutlined />} onClick={() => showPreview(template)} />
          </Tooltip>,
          <Tooltip title="Copy JSON">
            <Button type="text" icon={<CopyOutlined />} onClick={() => copyToClipboard(template)} />
          </Tooltip>
        ]}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
            <Space>
              <div style={{ color: category.color, fontSize: '24px' }}>
                {category.icon}
              </div>
              <div>
                <Text strong>{template.display_name}</Text>
                <br />
                <Text type="secondary" style={{ fontSize: '12px' }}>{template.name}</Text>
              </div>
            </Space>
            <Badge count={template.usage_count} showZero color="#8c8c8c" />
          </div>
          
          <Paragraph ellipsis={{ rows: 2 }} type="secondary" style={{ marginBottom: '8px' }}>
            {template.description}
          </Paragraph>
          
          <Space size="small" wrap>
            <Tag color={category.color}>{category.label}</Tag>
            <Tag color={getDataTypeColor(template.data_type)}>{template.data_type}</Tag>
            {template.unit && <Tag>{template.unit}</Tag>}
          </Space>
          
          {template.validation_rules && (
            <div style={{ marginTop: '8px' }}>
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Validation: {Object.keys(template.validation_rules).join(', ')}
              </Text>
            </div>
          )}
        </Space>
      </Card>
    );
  };

  return (
    <div style={{ padding: '24px' }}>
      <div style={{ marginBottom: '24px' }}>
        <Title level={2}>Field Template Library</Title>
        <Paragraph type="secondary">
          Browse and select from pre-configured industrial field templates to build your metadata schemas.
        </Paragraph>
      </div>

      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Row gutter={[16, 16]} align="middle">
          <Col flex="auto">
            <Search
              placeholder="Search templates by name or description"
              prefix={<SearchOutlined />}
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              allowClear
              size="large"
            />
          </Col>
          <Col>
            <Space size="small">
              <Button
                type={selectedCategory === null ? 'primary' : 'default'}
                onClick={() => setSelectedCategory(null)}
              >
                All Categories
              </Button>
              {templateCategories.map(cat => (
                <Button
                  key={cat.key}
                  type={selectedCategory === cat.key ? 'primary' : 'default'}
                  icon={cat.icon}
                  onClick={() => setSelectedCategory(cat.key)}
                >
                  {cat.label}
                </Button>
              ))}
            </Space>
          </Col>
        </Row>

        {loading ? (
          <div style={{ textAlign: 'center', padding: '48px' }}>
            <Spin size="large" />
          </div>
        ) : (
          <>
            <div style={{ marginBottom: '16px' }}>
              <Text type="secondary">
                {filteredTemplates.length} templates found
                {selectedCategory && ` in ${templateCategories.find(c => c.key === selectedCategory)?.label}`}
              </Text>
            </div>
            
            <Row gutter={[16, 16]}>
              {filteredTemplates.map(template => (
                <Col key={template.id} xs={24} sm={12} lg={8} xl={6}>
                  {renderTemplateCard(template)}
                </Col>
              ))}
            </Row>
          </>
        )}
      </Space>

      <Modal
        title={previewTemplate?.display_name}
        open={!!previewTemplate}
        onCancel={() => setPreviewTemplate(null)}
        footer={[
          <Button key="close" onClick={() => setPreviewTemplate(null)}>
            Close
          </Button>,
          <Button
            key="copy"
            type="primary"
            icon={<CopyOutlined />}
            onClick={() => previewTemplate && copyToClipboard(previewTemplate)}
          >
            Copy JSON
          </Button>
        ]}
        width={600}
      >
        {previewTemplate && (
          <Space direction="vertical" style={{ width: '100%' }} size="middle">
            <div>
              <Text strong>Field Name:</Text> <Text code>{previewTemplate.name}</Text>
            </div>
            <div>
              <Text strong>Display Name:</Text> {previewTemplate.display_name}
            </div>
            <div>
              <Text strong>Description:</Text>
              <Paragraph style={{ marginTop: '4px' }}>{previewTemplate.description}</Paragraph>
            </div>
            <Divider />
            <Row gutter={[16, 16]}>
              <Col span={12}>
                <Text strong>Category:</Text> <Tag color={getCategoryConfig(previewTemplate.category).color}>{getCategoryConfig(previewTemplate.category).label}</Tag>
              </Col>
              <Col span={12}>
                <Text strong>Data Type:</Text> <Tag color={getDataTypeColor(previewTemplate.data_type)}>{previewTemplate.data_type}</Tag>
              </Col>
            </Row>
            {previewTemplate.unit && (
              <div>
                <Text strong>Unit:</Text> {previewTemplate.unit}
              </div>
            )}
            {previewTemplate.options && previewTemplate.options.length > 0 && (
              <div>
                <Text strong>Options:</Text>
                <div style={{ marginTop: '8px' }}>
                  {previewTemplate.options.map((opt, idx) => (
                    <Tag key={idx} style={{ marginBottom: '4px' }}>{opt}</Tag>
                  ))}
                </div>
              </div>
            )}
            {previewTemplate.validation_rules && (
              <div>
                <Text strong>Validation Rules:</Text>
                <pre style={{ marginTop: '8px', background: '#f5f5f5', padding: '8px', borderRadius: '4px' }}>
                  {JSON.stringify(previewTemplate.validation_rules, null, 2)}
                </pre>
              </div>
            )}
            {previewTemplate.constraints && (
              <div>
                <Text strong>Constraints:</Text>
                <pre style={{ marginTop: '8px', background: '#f5f5f5', padding: '8px', borderRadius: '4px' }}>
                  {JSON.stringify(previewTemplate.constraints, null, 2)}
                </pre>
              </div>
            )}
          </Space>
        )}
      </Modal>
    </div>
  );
};

export default FieldTemplateLibrary;