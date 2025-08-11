import React, { useEffect, useState } from 'react';
import { 
  Card, 
  Typography, 
  Space, 
  Table, 
  Tag, 
  Alert, 
  Checkbox,
  Tooltip,
  Select,
  Input
} from 'antd';
import { 
  CheckCircleOutlined, 
  ExclamationCircleOutlined,
  FileOutlined,
  DatabaseOutlined
} from '@ant-design/icons';
import { 
  FileAssociationWizardState, 
  CreateAssociationRequest, 
  AssociationType,
  ValidationResult 
} from '../../types/associations';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { Option } = Select;

interface AssociationReviewProps {
  wizardState: FileAssociationWizardState;
  onAssociationsUpdate: (associations: CreateAssociationRequest[]) => void;
}

interface ReviewAssociation extends CreateAssociationRequest {
  fileName: string;
  fileSize: number;
  validationStatus: ValidationResult;
  validationMessages: string[];
  selected: boolean;
}

const AssociationReview: React.FC<AssociationReviewProps> = ({
  wizardState,
  onAssociationsUpdate
}) => {
  const [reviewAssociations, setReviewAssociations] = useState<ReviewAssociation[]>([]);
  const [selectAll, setSelectAll] = useState(true);

  useEffect(() => {
    if (wizardState.selectedAsset && wizardState.uploadedFiles.length > 0) {
      const associations = createAssociationsFromFiles();
      setReviewAssociations(associations);
      updateParentAssociations(associations);
    }
  }, [wizardState.selectedAsset, wizardState.uploadedFiles]);

  const createAssociationsFromFiles = (): ReviewAssociation[] => {
    if (!wizardState.selectedAsset) return [];

    return wizardState.uploadedFiles
      .filter(f => f.status === 'completed' && f.fileId)
      .map((uploadedFile, index) => {
        const fileType = detectFileType(uploadedFile.file.name);
        const validation = validateAssociation(uploadedFile.file, fileType);
        
        return {
          assetId: wizardState.selectedAsset!.id,
          fileId: uploadedFile.fileId!,
          fileType,
          metadata: JSON.stringify({
            originalFileName: uploadedFile.file.name,
            fileSize: uploadedFile.file.size,
            uploadDate: new Date().toISOString(),
            associationOrder: index
          }),
          createdBy: 1, // This would come from auth context
          fileName: uploadedFile.file.name,
          fileSize: uploadedFile.file.size,
          validationStatus: validation.status,
          validationMessages: validation.messages,
          selected: true
        };
      });
  };

  const detectFileType = (fileName: string): AssociationType => {
    const extension = fileName.split('.').pop()?.toLowerCase();
    const firmwareExtensions = ['bin', 'hex', 'srec', 'elf', 'img', 'rom', 'fw'];
    
    return firmwareExtensions.includes(extension || '') ? 'Firmware' : 'Configuration';
  };

  const validateAssociation = (file: File, fileType: AssociationType): { 
    status: ValidationResult; 
    messages: string[] 
  } => {
    const messages: string[] = [];
    let status: ValidationResult = 'Passed';

    // File size validation
    const maxSize = fileType === 'Firmware' ? 2 * 1024 * 1024 * 1024 : 100 * 1024 * 1024;
    if (file.size > maxSize) {
      messages.push(`File size exceeds recommended limit for ${fileType.toLowerCase()} files`);
      status = 'Warning';
    }

    // File name validation
    if (file.name.length > 255) {
      messages.push('File name is too long (max 255 characters)');
      status = 'Warning';
    }

    // Special characters in filename
    if (/[<>:"/\\|?*]/.test(file.name)) {
      messages.push('File name contains special characters that may cause issues');
      status = 'Warning';
    }

    // Security validation (simulate)
    if (file.name.toLowerCase().includes('test') || file.name.toLowerCase().includes('debug')) {
      messages.push('File appears to be a test or debug version - verify before production use');
      status = 'Warning';
    }

    return { status, messages };
  };

  const updateParentAssociations = (associations: ReviewAssociation[]) => {
    const selectedAssociations = associations
      .filter(a => a.selected)
      .map(({ fileName, fileSize, validationStatus, validationMessages, selected, ...rest }) => rest);
    
    onAssociationsUpdate(selectedAssociations);
  };

  const handleSelectionChange = (associationIndex: number, selected: boolean) => {
    const updated = reviewAssociations.map((assoc, index) => 
      index === associationIndex ? { ...assoc, selected } : assoc
    );
    
    setReviewAssociations(updated);
    updateParentAssociations(updated);
  };

  const handleSelectAll = (checked: boolean) => {
    setSelectAll(checked);
    const updated = reviewAssociations.map(assoc => ({ ...assoc, selected: checked }));
    setReviewAssociations(updated);
    updateParentAssociations(updated);
  };

  const handleFileTypeChange = (associationIndex: number, newFileType: AssociationType) => {
    const updated = reviewAssociations.map((assoc, index) => {
      if (index === associationIndex) {
        const validation = validateAssociation({ name: assoc.fileName, size: assoc.fileSize } as File, newFileType);
        return {
          ...assoc,
          fileType: newFileType,
          validationStatus: validation.status,
          validationMessages: validation.messages
        };
      }
      return assoc;
    });
    
    setReviewAssociations(updated);
    updateParentAssociations(updated);
  };

  const handleMetadataChange = (associationIndex: number, metadata: string) => {
    const updated = reviewAssociations.map((assoc, index) => 
      index === associationIndex ? { ...assoc, metadata } : assoc
    );
    
    setReviewAssociations(updated);
    updateParentAssociations(updated);
  };

  const getValidationIcon = (status: ValidationResult) => {
    switch (status) {
      case 'Passed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'Warning':
        return <ExclamationCircleOutlined style={{ color: '#faad14' }} />;
      case 'Failed':
        return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
      default:
        return <FileOutlined />;
    }
  };

  const getValidationColor = (status: ValidationResult) => {
    switch (status) {
      case 'Passed':
        return 'success';
      case 'Warning':
        return 'warning';
      case 'Failed':
        return 'error';
      default:
        return 'default';
    }
  };

  const columns = [
    {
      title: (
        <Checkbox 
          checked={selectAll}
          indeterminate={reviewAssociations.some(a => a.selected) && !reviewAssociations.every(a => a.selected)}
          onChange={(e) => handleSelectAll(e.target.checked)}
        >
          Select
        </Checkbox>
      ),
      key: 'select',
      width: 80,
      render: (_: any, record: ReviewAssociation, index: number) => (
        <Checkbox
          checked={record.selected}
          onChange={(e) => handleSelectionChange(index, e.target.checked)}
        />
      ),
    },
    {
      title: 'File Name',
      key: 'fileName',
      render: (_: any, record: ReviewAssociation) => (
        <Space>
          <FileOutlined />
          <Text>{record.fileName}</Text>
        </Space>
      ),
    },
    {
      title: 'File Type',
      key: 'fileType',
      width: 150,
      render: (_: any, record: ReviewAssociation, index: number) => (
        <Select
          value={record.fileType}
          onChange={(value) => handleFileTypeChange(index, value)}
          style={{ width: '100%' }}
          size="small"
        >
          <Option value="Configuration">Configuration</Option>
          <Option value="Firmware">Firmware</Option>
        </Select>
      ),
    },
    {
      title: 'Size',
      key: 'fileSize',
      width: 100,
      render: (_: any, record: ReviewAssociation) => (
        <Text>{(record.fileSize / 1024 / 1024).toFixed(2)} MB</Text>
      ),
    },
    {
      title: 'Validation',
      key: 'validation',
      width: 120,
      render: (_: any, record: ReviewAssociation) => (
        <Tooltip title={record.validationMessages.join('; ')}>
          <Space>
            {getValidationIcon(record.validationStatus)}
            <Tag color={getValidationColor(record.validationStatus)}>
              {record.validationStatus}
            </Tag>
          </Space>
        </Tooltip>
      ),
    },
    {
      title: 'Metadata',
      key: 'metadata',
      width: 200,
      render: (_: any, record: ReviewAssociation, index: number) => (
        <TextArea
          value={record.metadata}
          onChange={(e) => handleMetadataChange(index, e.target.value)}
          rows={2}
          placeholder="Optional metadata..."
          size="small"
        />
      ),
    },
  ];

  const selectedCount = reviewAssociations.filter(a => a.selected).length;
  const totalCount = reviewAssociations.length;
  const hasValidationIssues = reviewAssociations.some(a => a.validationStatus !== 'Passed');
  const hasFailures = reviewAssociations.some(a => a.validationStatus === 'Failed');

  return (
    <Card 
      title={
        <Space>
          <DatabaseOutlined />
          <Title level={4} style={{ margin: 0 }}>
            Review & Confirm Associations
          </Title>
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Summary */}
        <Card size="small" style={{ backgroundColor: '#fafafa' }}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <Space>
              <Text strong>Target Asset:</Text>
              <Tag color="blue">{wizardState.selectedAsset?.name}</Tag>
            </Space>
            <Space>
              <Text strong>Associations to Create:</Text>
              <Tag color="green">{selectedCount} of {totalCount} selected</Tag>
              {hasValidationIssues && (
                <Tag color="orange">
                  {reviewAssociations.filter(a => a.validationStatus === 'Warning').length} warnings
                </Tag>
              )}
              {hasFailures && (
                <Tag color="red">
                  {reviewAssociations.filter(a => a.validationStatus === 'Failed').length} failures
                </Tag>
              )}
            </Space>
          </Space>
        </Card>

        {/* Validation Alerts */}
        {hasFailures && (
          <Alert
            message="Validation Failures Detected"
            description="Some files have validation failures. Please review and fix issues before proceeding."
            type="error"
            showIcon
          />
        )}

        {hasValidationIssues && !hasFailures && (
          <Alert
            message="Validation Warnings"
            description="Some files have validation warnings. Review carefully but you can proceed if acceptable."
            type="warning"
            showIcon
          />
        )}

        {selectedCount === 0 && (
          <Alert
            message="No Associations Selected"
            description="Please select at least one file association to create."
            type="info"
            showIcon
          />
        )}

        {/* Association Table */}
        <Table
          columns={columns}
          dataSource={reviewAssociations}
          rowKey={(record, index) => `${record.fileId}-${index}`}
          pagination={false}
          size="small"
          scroll={{ x: 800 }}
          locale={{ emptyText: 'No file associations to review' }}
        />

        {/* Instructions */}
        <Alert
          message="Review Instructions"
          description={
            <div>
              <p>• Review each file association carefully before proceeding</p>
              <p>• Verify the file types are correctly detected</p>
              <p>• Add metadata to provide context for the associations</p>
              <p>• Address any validation warnings or failures</p>
              <p>• Uncheck any associations you don't want to create</p>
            </div>
          }
          type="info"
        />
      </Space>
    </Card>
  );
};

export default AssociationReview;