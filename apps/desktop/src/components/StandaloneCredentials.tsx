import React, { useState, useEffect } from 'react';
import {
  Typography,
  Button,
  Card,
  Space,
  Spin,
  Empty,
  App,
  Table,
  Modal,
  Form,
  Input,
  Select,
  Tag,
  Tooltip,
  Popconfirm,
  Alert,
  Tabs,
  Tree,
  Row,
  Col,
  Divider,
  Badge,
  Dropdown,
  Menu,
  TreeProps
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  LockOutlined,
  GlobalOutlined,
  KeyOutlined,
  FileTextOutlined,
  SearchOutlined,
  FolderOutlined,
  FolderOpenOutlined,
  CloudServerOutlined,
  DatabaseOutlined,
  CloudOutlined,
  AppstoreOutlined,
  HistoryOutlined,
  CopyOutlined,
  MoreOutlined,
  TagOutlined,
  FilterOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { 
  StandaloneCredentialInfo,
  CategoryWithChildren,
  CreateStandaloneCredentialRequest,
  UpdateStandaloneCredentialRequest,
  SearchCredentialsRequest,
  SearchCredentialsResponse,
  SecretType,
  CreateCategoryRequest,
  secretTypeDisplayNames,
  secretTypeIcons,
  defaultCategoryIcons,
  getStrengthColor,
  getStrengthLabel
} from '../types/vault';
import useAuthStore from '../store/auth';
import PasswordGenerator from './PasswordGenerator';
import PasswordInput from './PasswordInput';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { TabPane } = Tabs;

const StandaloneCredentials: React.FC = () => {
  const { token, user } = useAuthStore();
  const { message } = App.useApp();
  
  // State management
  const [credentials, setCredentials] = useState<StandaloneCredentialInfo[]>([]);
  const [categories, setCategories] = useState<CategoryWithChildren[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<number | null>(null);
  const [selectedCredential, setSelectedCredential] = useState<StandaloneCredentialInfo | null>(null);
  const [visibleCredentials, setVisibleCredentials] = useState<Set<number>>(new Set());
  const [decryptedValues, setDecryptedValues] = useState<Map<number, string>>(new Map());
  
  // Modal states
  const [createCredentialVisible, setCreateCredentialVisible] = useState(false);
  const [editCredentialVisible, setEditCredentialVisible] = useState(false);
  const [categoryModalVisible, setCategoryModalVisible] = useState(false);
  const [passwordGeneratorVisible, setPasswordGeneratorVisible] = useState(false);
  const [credentialHistoryVisible, setCredentialHistoryVisible] = useState(false);
  
  // Form instances
  const [createCredentialForm] = Form.useForm();
  const [editCredentialForm] = Form.useForm();
  const [categoryForm] = Form.useForm();
  const [searchForm] = Form.useForm();
  
  // Search and pagination
  const [searchParams, setSearchParams] = useState<SearchCredentialsRequest>({
    limit: 50,
    offset: 0
  });
  const [totalCount, setTotalCount] = useState(0);
  const [currentPage, setCurrentPage] = useState(1);

  useEffect(() => {
    if (token) {
      fetchCategories();
      searchCredentials();
    }
  }, [token]);

  const fetchCategories = async () => {
    if (!token) return;
    
    try {
      const categoriesData = await invoke<CategoryWithChildren[]>('get_credential_categories', { token });
      setCategories(categoriesData);
    } catch (error) {
      console.error('Failed to fetch categories:', error);
      message.error('Failed to load categories');
    }
  };

  const searchCredentials = async (params?: SearchCredentialsRequest) => {
    if (!token) return;
    
    setSearchLoading(true);
    try {
      const searchRequest = { ...searchParams, ...params };
      const response = await invoke<SearchCredentialsResponse>('search_credentials', {
        token,
        searchRequest
      });
      
      setCredentials(response.credentials);
      setTotalCount(response.total_count);
      setCurrentPage(response.page);
    } catch (error) {
      console.error('Failed to search credentials:', error);
      message.error('Failed to load credentials');
    } finally {
      setSearchLoading(false);
    }
  };

  const createCredential = async (values: any) => {
    if (!token || !user) return;
    
    setLoading(true);
    try {
      const request: CreateStandaloneCredentialRequest = {
        name: values.name,
        description: values.description || '',
        credential_type: values.credential_type,
        category_id: values.category_id,
        value: values.value,
        tags: values.tags ? values.tags.split(',').map((t: string) => t.trim()) : [],
        created_by: user.id
      };
      
      await invoke('create_standalone_credential', {
        token,
        credentialRequest: request
      });
      
      message.success('Credential created successfully');
      setCreateCredentialVisible(false);
      createCredentialForm.resetFields();
      searchCredentials();
    } catch (error) {
      console.error('Failed to create credential:', error);
      message.error(`Failed to create credential: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const updateCredential = async (values: any) => {
    if (!token || !user || !selectedCredential) return;
    
    setLoading(true);
    try {
      const request: UpdateStandaloneCredentialRequest = {
        id: selectedCredential.credential.id,
        name: values.name,
        description: values.description,
        category_id: values.category_id,
        value: values.value,
        author_id: user.id
      };
      
      await invoke('update_standalone_credential', {
        token,
        updateRequest: request
      });
      
      message.success('Credential updated successfully');
      setEditCredentialVisible(false);
      editCredentialForm.resetFields();
      searchCredentials();
    } catch (error) {
      console.error('Failed to update credential:', error);
      message.error(`Failed to update credential: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const deleteCredential = async (credentialId: number) => {
    if (!token) return;
    
    try {
      await invoke('delete_standalone_credential', {
        token,
        credentialId
      });
      
      message.success('Credential deleted successfully');
      searchCredentials();
    } catch (error) {
      console.error('Failed to delete credential:', error);
      message.error(`Failed to delete credential: ${error}`);
    }
  };

  const toggleCredentialVisibility = async (credentialId: number) => {
    if (visibleCredentials.has(credentialId)) {
      // Hide credential
      setVisibleCredentials(prev => {
        const newSet = new Set(prev);
        newSet.delete(credentialId);
        return newSet;
      });
      setDecryptedValues(prev => {
        const newMap = new Map(prev);
        newMap.delete(credentialId);
        return newMap;
      });
    } else {
      // Decrypt and show credential
      if (!token) return;
      
      try {
        const decryptedValue = await invoke<string>('decrypt_standalone_credential', {
          token,
          credentialId
        });
        
        setDecryptedValues(prev => new Map(prev).set(credentialId, decryptedValue));
        setVisibleCredentials(prev => new Set(prev).add(credentialId));
      } catch (error) {
        console.error('Failed to decrypt credential:', error);
        message.error('Failed to decrypt credential');
      }
    }
  };

  const handlePasswordGenerated = (password: string) => {
    if (createCredentialVisible) {
      createCredentialForm.setFieldsValue({ value: password });
    } else if (editCredentialVisible) {
      editCredentialForm.setFieldsValue({ value: password });
    }
    setPasswordGeneratorVisible(false);
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      message.success('Copied to clipboard');
    } catch (error) {
      message.error('Failed to copy to clipboard');
    }
  };

  const renderCategoryTree = (): TreeProps['treeData'] => {
    const renderNodes = (nodes: CategoryWithChildren[]): TreeProps['treeData'] => {
      return nodes.map(node => ({
        title: (
          <Space>
            <span>{node.category.name}</span>
            <Badge count={node.credential_count} style={{ backgroundColor: '#52c41a' }} />
          </Space>
        ),
        key: node.category.id.toString(),
        icon: getIconForCategory(node.category.name),
        children: node.children.length > 0 ? renderNodes(node.children) : undefined
      }));
    };

    return renderNodes(categories);
  };

  const getIconForCategory = (name: string) => {
    const iconMap: Record<string, React.ReactNode> = {
      'Jump Hosts': <CloudServerOutlined />,
      'Databases': <DatabaseOutlined />,
      'Network Equipment': <GlobalOutlined />,
      'Applications': <AppstoreOutlined />,
      'Cloud Services': <CloudOutlined />
    };
    
    return iconMap[name] || <FolderOutlined />;
  };

  const getSecretTypeIcon = (type: SecretType) => {
    const iconMap: Record<SecretType, React.ReactNode> = {
      Password: <LockOutlined />,
      IpAddress: <GlobalOutlined />,
      VpnKey: <KeyOutlined />,
      LicenseFile: <FileTextOutlined />
    };
    
    return iconMap[type];
  };

  const columns = [
    {
      title: 'Name',
      dataIndex: ['credential', 'name'],
      key: 'name',
      render: (text: string, record: StandaloneCredentialInfo) => (
        <Space>
          {getSecretTypeIcon(record.credential.credential_type)}
          <Text strong>{text}</Text>
        </Space>
      )
    },
    {
      title: 'Type',
      dataIndex: ['credential', 'credential_type'],
      key: 'type',
      render: (type: SecretType) => (
        <Tag color="blue">{secretTypeDisplayNames[type]}</Tag>
      )
    },
    {
      title: 'Category',
      key: 'category',
      render: (record: StandaloneCredentialInfo) => (
        record.category ? (
          <Tag icon={getIconForCategory(record.category.name)} color={record.category.color_code}>
            {record.category.name}
          </Tag>
        ) : <Text type="secondary">No category</Text>
      )
    },
    {
      title: 'Tags',
      key: 'tags',
      render: (record: StandaloneCredentialInfo) => (
        <Space>
          {record.tags.map(tag => (
            <Tag key={tag} icon={<TagOutlined />}>{tag}</Tag>
          ))}
        </Space>
      )
    },
    {
      title: 'Last Accessed',
      dataIndex: ['credential', 'last_accessed'],
      key: 'last_accessed',
      render: (date: string) => date ? new Date(date).toLocaleDateString() : 'Never'
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (record: StandaloneCredentialInfo) => (
        <Space>
          <Tooltip title={visibleCredentials.has(record.credential.id) ? "Hide" : "View"}>
            <Button
              icon={visibleCredentials.has(record.credential.id) ? <EyeInvisibleOutlined /> : <EyeOutlined />}
              onClick={() => toggleCredentialVisibility(record.credential.id)}
            />
          </Tooltip>
          {visibleCredentials.has(record.credential.id) && decryptedValues.has(record.credential.id) && (
            <Tooltip title="Copy">
              <Button
                icon={<CopyOutlined />}
                onClick={() => copyToClipboard(decryptedValues.get(record.credential.id)!)}
              />
            </Tooltip>
          )}
          <Tooltip title="Edit">
            <Button
              icon={<EditOutlined />}
              onClick={() => {
                setSelectedCredential(record);
                editCredentialForm.setFieldsValue({
                  name: record.credential.name,
                  description: record.credential.description,
                  category_id: record.credential.category_id,
                  credential_type: record.credential.credential_type
                });
                setEditCredentialVisible(true);
              }}
            />
          </Tooltip>
          <Popconfirm
            title="Delete this credential?"
            description="This action cannot be undone."
            onConfirm={() => deleteCredential(record.credential.id)}
            okText="Yes"
            cancelText="No"
          >
            <Tooltip title="Delete">
              <Button danger icon={<DeleteOutlined />} />
            </Tooltip>
          </Popconfirm>
          <Dropdown
            menu={{
              items: [
                {
                  key: 'history',
                  icon: <HistoryOutlined />,
                  label: 'View History',
                  onClick: () => {
                    setSelectedCredential(record);
                    setCredentialHistoryVisible(true);
                  }
                }
              ]
            }}
          >
            <Button icon={<MoreOutlined />} />
          </Dropdown>
        </Space>
      )
    }
  ];

  const expandedRowRender = (record: StandaloneCredentialInfo) => {
    if (!visibleCredentials.has(record.credential.id) || !decryptedValues.has(record.credential.id)) {
      return null;
    }

    const value = decryptedValues.get(record.credential.id)!;
    
    return (
      <Card size="small">
        <Space direction="vertical" style={{ width: '100%' }}>
          <Text type="secondary">Decrypted Value:</Text>
          <Input.Password
            value={value}
            readOnly
            visibilityToggle={false}
            style={{ fontFamily: 'monospace' }}
          />
          <Space>
            <Button size="small" icon={<CopyOutlined />} onClick={() => copyToClipboard(value)}>
              Copy
            </Button>
          </Space>
        </Space>
      </Card>
    );
  };

  return (
    <div>
      <Card>
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <Row justify="space-between" align="middle">
            <Col>
              <Title level={3}>
                <LockOutlined /> Standalone Credentials
              </Title>
              <Text type="secondary">
                Manage passwords and credentials for IT assets not tracked as PLCs
              </Text>
            </Col>
            <Col>
              <Button 
                type="primary" 
                icon={<PlusOutlined />}
                onClick={() => setCreateCredentialVisible(true)}
              >
                Add Credential
              </Button>
            </Col>
          </Row>

          <Row gutter={16}>
            <Col span={6}>
              <Card title="Categories" size="small">
                <Tree
                  showIcon
                  defaultExpandAll
                  treeData={renderCategoryTree()}
                  onSelect={(selectedKeys) => {
                    const categoryId = selectedKeys.length > 0 ? parseInt(selectedKeys[0] as string) : null;
                    setSelectedCategory(categoryId);
                    searchCredentials({ category_id: categoryId || undefined });
                  }}
                />
                <Divider />
                <Button 
                  block 
                  icon={<PlusOutlined />}
                  onClick={() => setCategoryModalVisible(true)}
                >
                  Manage Categories
                </Button>
              </Card>
            </Col>
            
            <Col span={18}>
              <Card 
                title="Credentials" 
                size="small"
                extra={
                  <Form
                    form={searchForm}
                    layout="inline"
                    onFinish={(values) => searchCredentials({
                      query: values.query,
                      credential_type: values.type,
                      tags: values.tags ? values.tags.split(',').map((t: string) => t.trim()) : undefined
                    })}
                  >
                    <Form.Item name="query">
                      <Input 
                        placeholder="Search credentials..." 
                        prefix={<SearchOutlined />}
                        allowClear
                      />
                    </Form.Item>
                    <Form.Item name="type">
                      <Select 
                        placeholder="Type" 
                        style={{ width: 120 }}
                        allowClear
                      >
                        {Object.entries(secretTypeDisplayNames).map(([key, value]) => (
                          <Select.Option key={key} value={key}>{value}</Select.Option>
                        ))}
                      </Select>
                    </Form.Item>
                    <Form.Item>
                      <Button htmlType="submit" icon={<FilterOutlined />}>
                        Filter
                      </Button>
                    </Form.Item>
                  </Form>
                }
              >
                <Table
                  columns={columns}
                  dataSource={credentials}
                  rowKey={(record) => record.credential.id}
                  loading={searchLoading}
                  expandable={{ expandedRowRender }}
                  locale={{
                    emptyText: (
                      <Empty 
                        description="No credentials found"
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                      />
                    )
                  }}
                  pagination={{
                    current: currentPage,
                    total: totalCount,
                    pageSize: 50,
                    onChange: (page) => {
                      searchCredentials({ offset: (page - 1) * 50 });
                    }
                  }}
                />
              </Card>
            </Col>
          </Row>
        </Space>
      </Card>

      {/* Create Credential Modal */}
      <Modal
        title="Create Standalone Credential"
        open={createCredentialVisible}
        onOk={() => createCredentialForm.submit()}
        onCancel={() => {
          setCreateCredentialVisible(false);
          createCredentialForm.resetFields();
        }}
        confirmLoading={loading}
        width={600}
      >
        <Form
          form={createCredentialForm}
          layout="vertical"
          onFinish={createCredential}
        >
          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: 'Please enter a name' }]}
          >
            <Input placeholder="e.g., Production Database Password" />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description"
          >
            <TextArea rows={2} placeholder="Optional description" />
          </Form.Item>

          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name="credential_type"
                label="Type"
                rules={[{ required: true, message: 'Please select a type' }]}
              >
                <Select>
                  {Object.entries(secretTypeDisplayNames).map(([key, value]) => (
                    <Select.Option key={key} value={key}>
                      {getSecretTypeIcon(key as SecretType)} {value}
                    </Select.Option>
                  ))}
                </Select>
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name="category_id"
                label="Category"
              >
                <Select placeholder="Select a category" allowClear>
                  {categories.map(cat => (
                    <Select.OptGroup key={cat.category.id} label={cat.category.name}>
                      <Select.Option value={cat.category.id}>
                        {cat.category.name}
                      </Select.Option>
                      {cat.children.map(child => (
                        <Select.Option key={child.category.id} value={child.category.id}>
                          &nbsp;&nbsp;&nbsp;&nbsp;{child.category.name}
                        </Select.Option>
                      ))}
                    </Select.OptGroup>
                  ))}
                </Select>
              </Form.Item>
            </Col>
          </Row>

          <Form.Item
            name="value"
            label={
              <Space>
                <span>Value</span>
                <Button 
                  size="small" 
                  icon={<KeyOutlined />}
                  onClick={() => setPasswordGeneratorVisible(true)}
                >
                  Generate
                </Button>
              </Space>
            }
            rules={[{ required: true, message: 'Please enter a value' }]}
          >
            <PasswordInput />
          </Form.Item>

          <Form.Item
            name="tags"
            label="Tags"
            help="Comma-separated tags for organization"
          >
            <Input placeholder="e.g., production, mysql, primary" />
          </Form.Item>
        </Form>
      </Modal>

      {/* Edit Credential Modal */}
      <Modal
        title="Edit Credential"
        open={editCredentialVisible}
        onOk={() => editCredentialForm.submit()}
        onCancel={() => {
          setEditCredentialVisible(false);
          editCredentialForm.resetFields();
          setSelectedCredential(null);
        }}
        confirmLoading={loading}
        width={600}
      >
        <Form
          form={editCredentialForm}
          layout="vertical"
          onFinish={updateCredential}
        >
          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: 'Please enter a name' }]}
          >
            <Input />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description"
          >
            <TextArea rows={2} />
          </Form.Item>

          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                name="credential_type"
                label="Type"
              >
                <Select disabled>
                  {Object.entries(secretTypeDisplayNames).map(([key, value]) => (
                    <Select.Option key={key} value={key}>
                      {getSecretTypeIcon(key as SecretType)} {value}
                    </Select.Option>
                  ))}
                </Select>
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item
                name="category_id"
                label="Category"
              >
                <Select placeholder="Select a category" allowClear>
                  {categories.map(cat => (
                    <Select.OptGroup key={cat.category.id} label={cat.category.name}>
                      <Select.Option value={cat.category.id}>
                        {cat.category.name}
                      </Select.Option>
                      {cat.children.map(child => (
                        <Select.Option key={child.category.id} value={child.category.id}>
                          &nbsp;&nbsp;&nbsp;&nbsp;{child.category.name}
                        </Select.Option>
                      ))}
                    </Select.OptGroup>
                  ))}
                </Select>
              </Form.Item>
            </Col>
          </Row>

          <Form.Item
            name="value"
            label={
              <Space>
                <span>New Value (leave empty to keep current)</span>
                <Button 
                  size="small" 
                  icon={<KeyOutlined />}
                  onClick={() => setPasswordGeneratorVisible(true)}
                >
                  Generate
                </Button>
              </Space>
            }
          >
            <PasswordInput />
          </Form.Item>
        </Form>
      </Modal>

      {/* Password Generator Modal */}
      <PasswordGenerator
        visible={passwordGeneratorVisible}
        onClose={() => setPasswordGeneratorVisible(false)}
        onGenerate={handlePasswordGenerated}
      />

      {/* Category Management Modal */}
      <Modal
        title="Manage Categories"
        open={categoryModalVisible}
        onCancel={() => setCategoryModalVisible(false)}
        footer={null}
        width={600}
      >
        <Alert 
          message="Category management interface will be implemented in the next iteration" 
          type="info" 
          showIcon 
        />
      </Modal>

      {/* Credential History Modal */}
      <Modal
        title="Credential History"
        open={credentialHistoryVisible}
        onCancel={() => {
          setCredentialHistoryVisible(false);
          setSelectedCredential(null);
        }}
        footer={null}
        width={800}
      >
        <Alert 
          message="Credential history interface will be implemented in the next iteration" 
          type="info" 
          showIcon 
        />
      </Modal>
    </div>
  );
};

export default StandaloneCredentials;