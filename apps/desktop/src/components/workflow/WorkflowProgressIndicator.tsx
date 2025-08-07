import React from 'react';
import { Progress, Typography, Row, Col } from 'antd';
import { CheckCircleOutlined, LoadingOutlined } from '@ant-design/icons';
import { WorkflowStepConfig } from '../../types/workflow';

const { Text } = Typography;

interface WorkflowProgressIndicatorProps {
  currentStep: number;
  totalSteps: number;
  steps: WorkflowStepConfig[];
  showPercentage?: boolean;
  showStepNames?: boolean;
}

export const WorkflowProgressIndicator: React.FC<WorkflowProgressIndicatorProps> = ({
  currentStep,
  totalSteps,
  steps,
  showPercentage = true,
  showStepNames = true
}) => {
  const progressPercent = Math.round(((currentStep + 1) / totalSteps) * 100);
  
  const getStepStatus = (stepIndex: number) => {
    if (stepIndex < currentStep) return 'completed';
    if (stepIndex === currentStep) return 'current';
    return 'pending';
  };

  const getStepIcon = (stepIndex: number) => {
    const status = getStepStatus(stepIndex);
    
    switch (status) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a', fontSize: '16px' }} />;
      case 'current':
        return <LoadingOutlined style={{ color: '#1890ff', fontSize: '16px' }} spin />;
      default:
        return (
          <div
            style={{
              width: '16px',
              height: '16px',
              borderRadius: '50%',
              border: '2px solid #d9d9d9',
              backgroundColor: '#f5f5f5'
            }}
          />
        );
    }
  };

  return (
    <div style={{ width: '100%' }}>
      <Row align="middle" gutter={16} style={{ marginBottom: '16px' }}>
        <Col flex="auto">
          <Progress
            percent={progressPercent}
            status="active"
            showInfo={showPercentage}
            strokeColor={{
              '0%': '#108ee9',
              '100%': '#87d068',
            }}
          />
        </Col>
        <Col>
          <Text type="secondary">
            Step {currentStep + 1} of {totalSteps}
          </Text>
        </Col>
      </Row>

      {showStepNames && (
        <Row gutter={[8, 8]} style={{ marginTop: '12px' }}>
          {steps.map((step, index) => {
            const status = getStepStatus(index);
            const isActive = status === 'current';
            const isCompleted = status === 'completed';
            
            return (
              <Col key={step.name} span={24 / Math.min(steps.length, 5)}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    padding: '4px 8px',
                    borderRadius: '6px',
                    backgroundColor: isActive ? '#e6f7ff' : isCompleted ? '#f6ffed' : '#fafafa',
                    border: isActive ? '1px solid #91d5ff' : '1px solid transparent',
                    transition: 'all 0.3s'
                  }}
                >
                  <div style={{ marginRight: '8px' }}>
                    {getStepIcon(index)}
                  </div>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <Text
                      style={{
                        fontSize: '12px',
                        fontWeight: isActive ? 600 : 400,
                        color: isCompleted ? '#52c41a' : isActive ? '#1890ff' : '#8c8c8c'
                      }}
                      ellipsis
                    >
                      {step.title}
                    </Text>
                  </div>
                </div>
              </Col>
            );
          })}
        </Row>
      )}
    </div>
  );
};