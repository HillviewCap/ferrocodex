import React, { useState, useEffect } from 'react';
import {
  Modal,
  Steps,
  Form,
  Input,
  Select,
  Button,
  Upload,
  Card,
  Typography,
  Space,
  Table,
  Alert,
  Checkbox,
  notification,
  Divider,
  Tag,
} from 'antd';
import {
  InboxOutlined,
  FileTextOutlined,
  CheckCircleOutlined,
  SettingOutlined,
  PlayCircleOutlined,
  UploadOutlined,
} from '@ant-design/icons';
import { 
  CreateBulkImportSessionRequest, 
  ImportTemplate, 
  ValidationResults, 
  ProcessingOptions, 
  ValidationMode,
  validateSessionName,
  validateImportType,
  validateCSVFile,
} from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';
import ImportTemplateManager from './ImportTemplateManager';

const { Title, Text } = Typography;
const { Option } = Select;
const { Dragger } = Upload;

interface BulkImportWizardProps {
  visible: boolean;
  onClose: () => void;
  onComplete: (sessionId: number) => void;
}

const BulkImportWizard: React.FC<BulkImportWizardProps> = ({
  visible,
  onClose,
  onComplete,
}) => {
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [validationResults, setValidationResults] = useState<ValidationResults | null>(null);
  const [processingOptions, setProcessingOptions] = useState<ProcessingOptions>({
    skip_existing: false,
    update_existing: false,
    create_missing_parents: true,
    validation_mode: 'strict',
  });
  const [templateManagerVisible, setTemplateManagerVisible] = useState(false);
  const [sessionId, setSessionId] = useState<number | null>(null);

  const {
    isLoading,
    error,
    createSession,
    uploadFile,
    validateData,
    startProcessing,
    clearError,
  } = useBulkImportStore();

  useEffect(() => {
    if (error) {
      notification.error({
        message: 'Wizard Error',
        description: error,
      });
      clearError();
    }
  }, [error, clearError]);

  const resetWizard = () => {
    setCurrentStep(0);
    setSelectedFile(null);
    setValidationResults(null);
    setSessionId(null);
    form.resetFields();
  };

  const handleClose = () => {
    resetWizard();
    onClose();
  };

  // Step 1: Session Configuration
  const handleSessionConfig = async (values: any) => {
    try {
      const sessionRequest: CreateBulkImportSessionRequest = {
        session_name: values.sessionName,
        import_type: values.importType,
        template_path: values.templatePath,
      };

      const session = await createSession(sessionRequest);
      setSessionId(session.id);
      setCurrentStep(1);
    } catch (error) {
      // Error handled by store
    }
  };

  // Step 2: File Upload
  const handleFileUpload = async () => {
    if (!selectedFile || !sessionId) {
      notification.warning({
        message: 'No File Selected',
        description: 'Please select a CSV file to upload',
      });
      return;
    }

    try {
      // In a real implementation, we would handle file upload differently
      // For now, we'll simulate with the file path
      const filePath = selectedFile.name; // This would be the actual file path
      await uploadFile(sessionId, filePath);
      setCurrentStep(2);
      
      notification.success({
        message: 'File Uploaded',
        description: 'CSV file uploaded successfully',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  // Step 3: Data Validation
  const handleDataValidation = async () => {
    if (!sessionId) return;

    try {
      const results = await validateData(sessionId);
      setValidationResults(results);
      setCurrentStep(3);
    } catch (error) {
      // Error handled by store
    }
  };

  // Step 4: Configuration
  const handleConfiguration = () => {
    setCurrentStep(4);
  };

  // Step 5: Processing
  const handleStartProcessing = async () => {
    if (!sessionId) return;

    try {
      await startProcessing(sessionId, processingOptions);
      onComplete(sessionId);
      handleClose();
      
      notification.success({
        message: 'Import Started',
        description: 'Bulk import processing has been started',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleTemplateSelect = (template: ImportTemplate) => {
    form.setFieldsValue({
      templatePath: template.template_name,
      importType: template.template_type,
    });
  };

  const handleFileChange = (info: any) => {
    const { fileList } = info;
    if (fileList.length > 0) {
      const file = fileList[0].originFileObj;
      const validationError = validateCSVFile(file);
      
      if (validationError) {
        notification.error({
          message: 'Invalid File',
          description: validationError,
        });
        return false;
      }
      
      setSelectedFile(file);
    } else {
      setSelectedFile(null);
    }
    return false; // Prevent automatic upload
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <Card title="Import Session Configuration">
            <Form
              form={form}
              layout="vertical"
              onFinish={handleSessionConfig}
              initialValues={{
                importType: 'assets',
                validation_mode: 'strict',
              }}
            >
              <Form.Item
                name="sessionName"
                label="Session Name"
                rules={[
                  { required: true, message: 'Please enter a session name' },
                  {
                    validator: (_, value) => {
                      const error = validateSessionName(value);
                      return error ? Promise.reject(error) : Promise.resolve();
                    },
                  },
                ]}
              >
                <Input placeholder="Enter a descriptive name for this import session" />
              </Form.Item>

              <Form.Item
                name="importType"
                label="Import Type"
                rules={[
                  { required: true, message: 'Please select an import type' },
                  {
                    validator: (_, value) => {
                      const error = validateImportType(value);
                      return error ? Promise.reject(error) : Promise.resolve();
                    },
                  },
                ]}
              >
                <Select placeholder="Select what type of data you're importing">
                  <Option value="assets">Assets</Option>
                  <Option value="configurations">Configurations</Option>
                  <Option value="metadata">Metadata</Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="templatePath"
                label="Template (Optional)"
                extra="Select a pre-configured template to guide the import process"
              >
                <Input.Group compact>
                  <Input
                    style={{ width: 'calc(100% - 120px)' }}
                    placeholder="Select or enter template name"
                  />
                  <Button
                    style={{ width: '120px' }}
                    onClick={() => setTemplateManagerVisible(true)}
                  >
                    Browse Templates
                  </Button>
                </Input.Group>
              </Form.Item>

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
                <Button onClick={handleClose}>Cancel</Button>
                <Button type="primary" htmlType="submit" loading={isLoading}>
                  Next
                </Button>
              </Space>
            </Form>
          </Card>
        );

      case 1:
        return (
          <Card title="Upload CSV File">
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              <Alert
                message="File Upload Instructions"
                description={
                  <div>
                    <p>Please upload a CSV file containing your import data. The file should include:</p>
                    <ul>
                      <li>Required columns: <Tag>name</Tag>, <Tag>asset_type</Tag></li>
                      <li>Optional columns: <Tag>description</Tag>, <Tag>parent_name</Tag></li>
                      <li>Maximum file size: 50MB</li>
                      <li>Supported formats: .csv</li>
                    </ul>
                  </div>
                }
                type="info"
                showIcon
              />

              <Dragger
                name="file"
                multiple={false}
                accept=".csv"
                beforeUpload={handleFileChange}
                fileList={selectedFile ? [{
                  uid: '1',
                  name: selectedFile.name,
                  status: 'done',
                  size: selectedFile.size,
                }] : []}
                onRemove={() => setSelectedFile(null)}
              >
                <p className="ant-upload-drag-icon">
                  <InboxOutlined />
                </p>
                <p className="ant-upload-text">Click or drag CSV file to this area to upload</p>
                <p className="ant-upload-hint">
                  Support for a single CSV file upload. Ensure your file follows the required format.
                </p>
              </Dragger>

              {selectedFile && (
                <Card size="small" title="File Details">
                  <div>
                    <Text strong>Name:</Text> {selectedFile.name}<br />
                    <Text strong>Size:</Text> {(selectedFile.size / 1024).toFixed(2)} KB<br />
                    <Text strong>Type:</Text> {selectedFile.type}
                  </div>
                </Card>
              )}

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <Button onClick={() => setCurrentStep(0)}>Back</Button>
                <Button 
                  type="primary" 
                  onClick={handleFileUpload}
                  disabled={!selectedFile}
                  loading={isLoading}
                >
                  Upload & Continue
                </Button>
              </Space>
            </Space>
          </Card>
        );

      case 2:
        return (
          <Card title="Data Validation">
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              <Alert
                message="Validate Import Data"
                description="Review your uploaded data and validate it before processing. This step ensures data quality and compatibility."
                type="info"
                showIcon
              />

              <Card size="small">
                <div style={{ textAlign: 'center' }}>
                  <CheckCircleOutlined style={{ fontSize: '48px', color: '#52c41a' }} />
                  <Title level={4}>File Uploaded Successfully</Title>
                  <Text type="secondary">Ready to validate your import data</Text>
                </div>
              </Card>

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <Button onClick={() => setCurrentStep(1)}>Back</Button>
                <Button 
                  type="primary" 
                  onClick={handleDataValidation}
                  loading={isLoading}
                  icon={<CheckCircleOutlined />}
                >
                  Validate Data
                </Button>
              </Space>
            </Space>
          </Card>
        );

      case 3:
        return (
          <Card title="Validation Results">
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              {validationResults && (
                <>
                  {validationResults.is_valid ? (
                    <Alert
                      message="Validation Successful"
                      description="All data passed validation checks. You can proceed with the import."
                      type="success"
                      showIcon
                    />
                  ) : (
                    <Alert
                      message="Validation Issues Found"
                      description="Some issues were found in your data. Review the details below."
                      type="warning"
                      showIcon
                    />
                  )}

                  <Card size="small" title="Data Summary">
                    <div>
                      <Text strong>Total Items:</Text> {validationResults.preview_items.length}<br />
                      <Text strong>Errors:</Text> <Text type={validationResults.errors.length > 0 ? 'danger' : 'success'}>
                        {validationResults.errors.length}
                      </Text><br />
                      <Text strong>Warnings:</Text> <Text type={validationResults.warnings.length > 0 ? 'warning' : 'success'}>
                        {validationResults.warnings.length}
                      </Text>
                    </div>
                  </Card>

                  {validationResults.errors.length > 0 && (
                    <Card size="small" title="Errors" type="inner">
                      <Table
                        size="small"
                        dataSource={validationResults.errors}
                        pagination={false}
                        columns={[
                          { title: 'Row', dataIndex: 'row', key: 'row', width: 60 },
                          { title: 'Field', dataIndex: 'field', key: 'field', width: 100 },
                          { title: 'Value', dataIndex: 'value', key: 'value', width: 120 },
                          { title: 'Message', dataIndex: 'message', key: 'message' },
                        ]}
                      />
                    </Card>
                  )}

                  {validationResults.warnings.length > 0 && (
                    <Card size="small" title="Warnings" type="inner">
                      <Table
                        size="small"
                        dataSource={validationResults.warnings}
                        pagination={false}
                        columns={[
                          { title: 'Row', dataIndex: 'row', key: 'row', width: 60 },
                          { title: 'Field', dataIndex: 'field', key: 'field', width: 100 },
                          { title: 'Value', dataIndex: 'value', key: 'value', width: 120 },
                          { title: 'Message', dataIndex: 'message', key: 'message' },
                        ]}
                      />
                    </Card>
                  )}

                  {validationResults.preview_items.length > 0 && (
                    <Card size="small" title="Data Preview (First 5 Items)" type="inner">
                      <Table
                        size="small"
                        dataSource={validationResults.preview_items.slice(0, 5)}
                        pagination={false}
                        columns={[
                          { title: 'Name', dataIndex: 'name', key: 'name' },
                          { title: 'Type', dataIndex: 'asset_type', key: 'asset_type' },
                          { title: 'Description', dataIndex: 'description', key: 'description' },
                          { title: 'Parent', dataIndex: 'parent_name', key: 'parent_name' },
                        ]}
                      />
                    </Card>
                  )}
                </>
              )}

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <Button onClick={() => setCurrentStep(2)}>Back</Button>
                <Button 
                  type="primary" 
                  onClick={handleConfiguration}
                  disabled={validationResults ? !validationResults.is_valid : false}
                >
                  Continue to Configuration
                </Button>
              </Space>
            </Space>
          </Card>
        );

      case 4:
        return (
          <Card title="Processing Configuration">
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              <Alert
                message="Configure Import Options"
                description="Set up how the import should handle various scenarios during processing."
                type="info"
                showIcon
              />

              <Card size="small" title="Conflict Resolution">
                <Space direction="vertical" style={{ width: '100%' }}>
                  <Checkbox
                    checked={processingOptions.skip_existing}
                    onChange={(e) => setProcessingOptions(prev => ({
                      ...prev,
                      skip_existing: e.target.checked,
                    }))}
                  >
                    Skip existing assets (don't import if asset with same name exists)
                  </Checkbox>
                  
                  <Checkbox
                    checked={processingOptions.update_existing}
                    onChange={(e) => setProcessingOptions(prev => ({
                      ...prev,
                      update_existing: e.target.checked,
                    }))}
                  >
                    Update existing assets (modify existing assets with new data)
                  </Checkbox>
                  
                  <Checkbox
                    checked={processingOptions.create_missing_parents}
                    onChange={(e) => setProcessingOptions(prev => ({
                      ...prev,
                      create_missing_parents: e.target.checked,
                    }))}
                  >
                    Create missing parent folders automatically
                  </Checkbox>
                </Space>
              </Card>

              <Card size="small" title="Validation Mode">
                <Select
                  value={processingOptions.validation_mode}
                  onChange={(value: ValidationMode) => setProcessingOptions(prev => ({
                    ...prev,
                    validation_mode: value,
                  }))}
                  style={{ width: '100%' }}
                >
                  <Option value="strict">Strict - Fail on any validation error</Option>
                  <Option value="permissive">Permissive - Continue with warnings</Option>
                </Select>
              </Card>

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <Button onClick={() => setCurrentStep(3)}>Back</Button>
                <Button 
                  type="primary" 
                  onClick={handleStartProcessing}
                  loading={isLoading}
                  icon={<PlayCircleOutlined />}
                >
                  Start Import
                </Button>
              </Space>
            </Space>
          </Card>
        );

      default:
        return null;
    }
  };

  const steps = [
    {
      title: 'Session Setup',
      icon: <SettingOutlined />,
      description: 'Configure import session',
    },
    {
      title: 'File Upload',
      icon: <UploadOutlined />,
      description: 'Upload CSV data file',
    },
    {
      title: 'Validation',
      icon: <CheckCircleOutlined />,
      description: 'Validate import data',
    },
    {
      title: 'Review',
      icon: <FileTextOutlined />,
      description: 'Review validation results',
    },
    {
      title: 'Configuration',
      icon: <SettingOutlined />,
      description: 'Configure processing options',
    },
  ];

  return (
    <>
      <Modal
        title="Bulk Import Wizard"
        open={visible}
        onCancel={handleClose}
        width={900}
        footer={null}
        destroyOnClose
      >
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          <Steps current={currentStep} items={steps} />
          
          <div style={{ minHeight: '400px' }}>
            {renderStepContent()}
          </div>
        </Space>
      </Modal>

      <ImportTemplateManager
        visible={templateManagerVisible}
        onClose={() => setTemplateManagerVisible(false)}
        onSelectTemplate={handleTemplateSelect}
        templateType={form.getFieldValue('importType')}
      />
    </>
  );
};

export default BulkImportWizard;