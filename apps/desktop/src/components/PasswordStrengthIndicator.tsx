import React from 'react';
import { Progress, Typography, Tag, Space } from 'antd';
import { CheckCircleOutlined, CloseCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { PasswordStrength, getStrengthColor, getStrengthLabel } from '../types/vault';

const { Text } = Typography;

interface PasswordStrengthIndicatorProps {
  strength: PasswordStrength;
  showDetails?: boolean;
  size?: 'small' | 'default';
}

const PasswordStrengthIndicator: React.FC<PasswordStrengthIndicatorProps> = ({
  strength,
  showDetails = false,
  size = 'default'
}) => {
  const color = getStrengthColor(strength.score);
  const label = getStrengthLabel(strength.score);

  const getIcon = () => {
    if (strength.score >= 80) return <CheckCircleOutlined style={{ color }} />;
    if (strength.score >= 40) return <ExclamationCircleOutlined style={{ color }} />;
    return <CloseCircleOutlined style={{ color }} />;
  };

  const formatEntropy = (entropy: number) => {
    return Math.round(entropy * 10) / 10;
  };

  return (
    <div>
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        {/* Main strength indicator */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {getIcon()}
          <Progress
            percent={strength.score}
            strokeColor={color}
            showInfo={false}
            size={size}
            style={{ flex: 1 }}
          />
          <Tag color={color} style={{ margin: 0 }}>
            {label} ({strength.score}/100)
          </Tag>
        </div>

        {/* Details */}
        {showDetails && (
          <Space direction="vertical" size="small" style={{ width: '100%' }}>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginTop: '8px' }}>
              <Space size="small">
                <Text type="secondary">Length:</Text>
                <Tag color={strength.length >= 12 ? 'green' : strength.length >= 8 ? 'orange' : 'red'}>
                  {strength.length} chars
                </Tag>
              </Space>
              
              <Space size="small">
                <Text type="secondary">Entropy:</Text>
                <Tag color={strength.entropy >= 60 ? 'green' : strength.entropy >= 40 ? 'orange' : 'red'}>
                  {formatEntropy(strength.entropy)} bits
                </Tag>
              </Space>
            </div>

            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
              <Tag color={strength.has_uppercase ? 'green' : 'red'} icon={strength.has_uppercase ? <CheckCircleOutlined /> : <CloseCircleOutlined />}>
                Uppercase
              </Tag>
              <Tag color={strength.has_lowercase ? 'green' : 'red'} icon={strength.has_lowercase ? <CheckCircleOutlined /> : <CloseCircleOutlined />}>
                Lowercase
              </Tag>
              <Tag color={strength.has_numbers ? 'green' : 'red'} icon={strength.has_numbers ? <CheckCircleOutlined /> : <CloseCircleOutlined />}>
                Numbers
              </Tag>
              <Tag color={strength.has_special ? 'green' : 'red'} icon={strength.has_special ? <CheckCircleOutlined /> : <CloseCircleOutlined />}>
                Special
              </Tag>
            </div>

            {/* Feedback messages */}
            {strength.feedback.length > 0 && (
              <div style={{ marginTop: '8px' }}>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  Recommendations:
                </Text>
                <ul style={{ margin: '4px 0 0 16px', fontSize: '12px' }}>
                  {strength.feedback.map((feedback, index) => (
                    <li key={index} style={{ color: '#8c8c8c', marginBottom: '2px' }}>
                      {feedback}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </Space>
        )}
      </Space>
    </div>
  );
};

export default PasswordStrengthIndicator;