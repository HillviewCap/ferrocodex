import React from 'react';
import { Modal, Typography, Button, Space, Divider } from 'antd';
import { ExclamationCircleOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { exit } from '@tauri-apps/plugin-process';

const { Title, Paragraph, Text } = Typography;

interface EulaModalProps {
  visible: boolean;
  onAccept: () => void;
}

const EulaModal: React.FC<EulaModalProps> = ({ visible, onAccept }) => {
  const handleDecline = async () => {
    try {
      await exit(0);
    } catch (error) {
      console.error('Error exiting application:', error);
      // Fallback for environments where Tauri exit might not work
      window.close();
    }
  };

  const handleAccept = () => {
    onAccept();
  };

  return (
    <Modal
      title={
        <Space>
          <ExclamationCircleOutlined style={{ color: '#faad14' }} />
          <span>FerroCodex Alpha - End-User License Agreement</span>
        </Space>
      }
      open={visible}
      closable={false}
      maskClosable={false}
      keyboard={false}
      footer={[
        <Button 
          key="decline" 
          danger 
          onClick={handleDecline}
          style={{ marginRight: '8px' }}
        >
          Decline
        </Button>,
        <Button
          key="accept"
          type="primary"
          icon={<CheckCircleOutlined />}
          onClick={handleAccept}
        >
          Agree and Continue
        </Button>
      ]}
      width={700}
      centered
      destroyOnHidden={false}
    >
      <div style={{ maxHeight: '60vh', overflowY: 'auto', paddingRight: '8px' }}>
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          <Paragraph style={{ fontSize: '16px', marginBottom: '16px' }}>
            Welcome to the Alpha version of FerroCodex. Please read the following terms carefully before using the software.
          </Paragraph>

          <Paragraph style={{ fontSize: '16px', fontWeight: 'bold', marginBottom: '16px' }}>
            By clicking the "Agree and Continue" button below, you acknowledge that you understand and agree to be legally bound by these terms.
          </Paragraph>

          <Divider />

          <div>
            <Title level={4} style={{ color: '#d32f2f', marginBottom: '12px' }}>
              1. ALPHA SOFTWARE NOTICE
            </Title>
            <Paragraph style={{ fontSize: '14px' }}>
              This is a pre-release, Alpha version of the software. It is provided for testing and feedback purposes only. You acknowledge that the software is not yet feature-complete, may contain bugs and errors, and is subject to change without notice.
            </Paragraph>
          </div>

          <div>
            <Title level={4} style={{ color: '#d32f2f', marginBottom: '12px' }}>
              2. "AS-IS" WITH NO WARRANTY
            </Title>
            <Paragraph style={{ fontSize: '14px' }}>
              The software is provided to you "AS-IS" and "AS-AVAILABLE," without any warranties, guarantees, or representations of any kind, whether express or implied. The developer makes no claim that the software is fit for any particular purpose or that it will be error-free.
            </Paragraph>
          </div>

          <div>
            <Title level={4} style={{ color: '#d32f2f', marginBottom: '12px' }}>
              3. ASSUMPTION OF ALL RISK
            </Title>
            <Paragraph style={{ fontSize: '14px' }}>
              You are a knowledgeable professional with experience in OT/ICS environments. You understand that testing pre-release software carries inherent risks. You voluntarily and explicitly assume all risks associated with using this software, including but not limited to data loss, file corruption, interruption of operations, or unexpected behavior of connected equipment.
            </Paragraph>
          </div>

          <div>
            <Title level={4} style={{ color: '#d32f2f', marginBottom: '12px' }}>
              4. LIMITATION OF LIABILITY
            </Title>
            <Paragraph style={{ fontSize: '14px' }}>
              To the maximum extent permitted by law, the developer shall not be held liable for any damages of any kind arising from your use of this Alpha software. This includes, but is not limited to, direct, indirect, special, incidental, or consequential damages such as loss of profits, damage to equipment, production downtime, or loss of data. You are solely responsible for any and all outcomes that result from using this software.
            </Paragraph>
          </div>

          <div>
            <Title level={4} style={{ color: '#d32f2f', marginBottom: '12px' }}>
              5. BACKUPS AND SAFETY
            </Title>
            <Paragraph style={{ fontSize: '14px' }}>
              You are solely responsible for ensuring you have adequate backups and safety procedures in place before, during, and after using this software. Never use this Alpha software in a critical production environment without a thoroughly tested recovery plan.
            </Paragraph>
          </div>

          <Divider />

          <Paragraph style={{ fontSize: '16px', fontWeight: 'bold', textAlign: 'center' }}>
            <Text style={{ color: '#d32f2f' }}>
              By clicking "Agree and Continue," you confirm that you have read, understood, and accepted these terms. 
              If you do not agree, click "Decline" to exit the application.
            </Text>
          </Paragraph>
        </Space>
      </div>
    </Modal>
  );
};

export default EulaModal;