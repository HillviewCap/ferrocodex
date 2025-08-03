import React, { useState } from 'react';
import { Select, Badge, Tooltip, Card, Space, Typography, Alert } from 'antd';
import {
  GlobalOutlined,
  TeamOutlined,
  LockOutlined,
  SafetyCertificateOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import {
  SecurityClassificationLevel,
  SecurityClassificationUI,
  SECURITY_CLASSIFICATIONS,
  isHigherClassification
} from '../../types/security';

const { Option } = Select;
const { Text, Title } = Typography;

interface SecurityClassificationSelectorProps {
  value?: SecurityClassificationLevel;
  onChange: (level: SecurityClassificationLevel) => void;
  disabled?: boolean;
  showDescription?: boolean;
  showAccessRequirements?: boolean;
  allowElevation?: boolean;
  currentUserRole?: 'Administrator' | 'Engineer';
  inheritedClassification?: SecurityClassificationLevel;
  placeholder?: string;
  size?: 'small' | 'middle' | 'large';
}

const SecurityClassificationSelector: React.FC<SecurityClassificationSelectorProps> = ({
  value,
  onChange,
  disabled = false,
  showDescription = true,
  showAccessRequirements = false,
  allowElevation = true,
  currentUserRole = 'Engineer',
  inheritedClassification,
  placeholder = 'Select security classification',
  size = 'middle'
}) => {
  const [selectedLevel, setSelectedLevel] = useState<SecurityClassificationLevel | undefined>(value);

  const getIcon = (iconName: string) => {
    const iconMap = {
      'GlobalOutlined': <GlobalOutlined />,
      'TeamOutlined': <TeamOutlined />,
      'LockOutlined': <LockOutlined />,
      'SafetyCertificateOutlined': <SafetyCertificateOutlined />
    };
    return iconMap[iconName as keyof typeof iconMap] || <InfoCircleOutlined />;
  };

  const isLevelDisabled = (level: SecurityClassificationLevel): boolean => {
    if (disabled) return true;
    
    // Engineers cannot select RESTRICTED or SECRET without admin approval
    if (currentUserRole === 'Engineer' && 
        (level === SecurityClassificationLevel.RESTRICTED || level === SecurityClassificationLevel.SECRET)) {
      return !allowElevation;
    }
    
    // Cannot select lower than inherited classification
    if (inheritedClassification && isHigherClassification(inheritedClassification, level)) {
      return true;
    }
    
    return false;
  };

  const handleChange = (newLevel: SecurityClassificationLevel) => {
    setSelectedLevel(newLevel);
    onChange(newLevel);
  };

  const renderClassificationBadge = (classification: SecurityClassificationUI) => (
    <Space>
      <Badge
        color={classification.color}
        count={getIcon(classification.icon)}
        style={{ backgroundColor: 'transparent' }}
      />
      <Text strong style={{ color: classification.color }}>
        {classification.displayName}
      </Text>
    </Space>
  );

  const renderOption = (level: SecurityClassificationLevel) => {
    const classification = SECURITY_CLASSIFICATIONS[level];
    const isDisabled = isLevelDisabled(level);
    
    return (
      <Option 
        key={level} 
        value={level} 
        disabled={isDisabled}
        style={{ 
          opacity: isDisabled ? 0.6 : 1,
          padding: '8px 12px'
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          {renderClassificationBadge(classification)}
          {isDisabled && (
            <Tooltip title={getDisabledReason(level)}>
              <InfoCircleOutlined style={{ color: '#999' }} />
            </Tooltip>
          )}
        </div>
        {showDescription && (
          <div style={{ marginTop: 4, fontSize: '12px', color: '#666' }}>
            {classification.description}
          </div>
        )}
      </Option>
    );
  };

  const getDisabledReason = (level: SecurityClassificationLevel): string => {
    if (currentUserRole === 'Engineer' && 
        (level === SecurityClassificationLevel.RESTRICTED || level === SecurityClassificationLevel.SECRET)) {
      return 'Administrator approval required for this classification level';
    }
    
    if (inheritedClassification && isHigherClassification(inheritedClassification, level)) {
      return `Cannot select lower than inherited classification (${SECURITY_CLASSIFICATIONS[inheritedClassification].displayName})`;
    }
    
    return 'Not available';
  };

  const getSelectedClassification = (): SecurityClassificationUI | undefined => {
    return selectedLevel ? SECURITY_CLASSIFICATIONS[selectedLevel] : undefined;
  };

  return (
    <div>
      <Select
        value={selectedLevel}
        onChange={handleChange}
        placeholder={placeholder}
        disabled={disabled}
        style={{ width: '100%', minWidth: 200 }}
        size={size}
        popupRender={(menu) => (
          <div>
            {menu}
            {showAccessRequirements && selectedLevel && (
              <div style={{ padding: '8px 12px', borderTop: '1px solid #f0f0f0' }}>
                <Text strong style={{ fontSize: '12px', color: '#666' }}>
                  Access Requirements:
                </Text>
                <ul style={{ margin: '4px 0 0 16px', padding: 0, fontSize: '11px', color: '#666' }}>
                  {SECURITY_CLASSIFICATIONS[selectedLevel].accessRequirements.map((req, idx) => (
                    <li key={idx}>{req}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
      >
        {Object.values(SecurityClassificationLevel).map(renderOption)}
      </Select>

      {/* Classification Inheritance Warning */}
      {inheritedClassification && (
        <Alert
          message={`Inherited Classification: ${SECURITY_CLASSIFICATIONS[inheritedClassification].displayName}`}
          description="This asset inherits its classification from its parent folder. You can only select equal or higher classifications."
          type="info"
          showIcon
          style={{ marginTop: 8, fontSize: '12px' }}
          icon={getIcon(SECURITY_CLASSIFICATIONS[inheritedClassification].icon)}
        />
      )}

      {/* Elevation Warning */}
      {selectedLevel && currentUserRole === 'Engineer' && 
       (selectedLevel === SecurityClassificationLevel.RESTRICTED || selectedLevel === SecurityClassificationLevel.SECRET) && (
        <Alert
          message="Administrator Approval Required"
          description="This classification level requires administrator approval. Your request will be submitted for review."
          type="warning"
          showIcon
          style={{ marginTop: 8, fontSize: '12px' }}
        />
      )}

      {/* Selected Classification Details */}
      {showDescription && selectedLevel && (
        <Card 
          size="small" 
          style={{ marginTop: 12 }}
          title={
            <Space>
              {renderClassificationBadge(SECURITY_CLASSIFICATIONS[selectedLevel])}
              <Text type="secondary" style={{ fontSize: '12px' }}>Selected Classification</Text>
            </Space>
          }
        >
          <Space direction="vertical" size="small" style={{ width: '100%' }}>
            <Text style={{ fontSize: '13px' }}>
              {SECURITY_CLASSIFICATIONS[selectedLevel].description}
            </Text>
            
            {showAccessRequirements && (
              <div>
                <Text strong style={{ fontSize: '12px' }}>Access Requirements:</Text>
                <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
                  {SECURITY_CLASSIFICATIONS[selectedLevel].accessRequirements.map((req, idx) => (
                    <li key={idx} style={{ fontSize: '12px', marginBottom: 2 }}>{req}</li>
                  ))}
                </ul>
              </div>
            )}
          </Space>
        </Card>
      )}
    </div>
  );
};

export default SecurityClassificationSelector;