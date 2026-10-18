import React, { useState } from 'react';
import { Modal, Typography, Alert, Checkbox, Button, Space } from 'antd';
import { KeyOutlined } from '@ant-design/icons';
import useAuthStore from '../store/auth';

const { Paragraph, Text } = Typography;

// Shows a newly issued recovery key once. It can't be dismissed until the
// user confirms they stored it, because it is never shown again.
const RecoveryCodeModal: React.FC = () => {
  const { pendingRecoveryCode, dismissRecoveryCode } = useAuthStore();
  const [stored, setStored] = useState(false);

  const handleDone = () => {
    setStored(false);
    dismissRecoveryCode();
  };

  return (
    <Modal
      open={pendingRecoveryCode !== null}
      title={<Space><KeyOutlined />Save your recovery key</Space>}
      closable={false}
      keyboard={false}
      mask={{ closable: false }}
      footer={
        <Button type="primary" disabled={!stored} onClick={handleDone}>
          Done
        </Button>
      }
    >
      <Space orientation="vertical" size="middle" style={{ width: '100%' }}>
        <Paragraph style={{ marginBottom: 0 }}>
          Configurations, firmware, and vault secrets are encrypted with a key that only
          user passwords and this recovery key can unlock. If every administrator loses
          their password, the recovery key is the only way to get the data back.
        </Paragraph>

        <div style={{ textAlign: 'center' }}>
          <Text code copyable={{ text: pendingRecoveryCode ?? '' }} style={{ fontSize: 16 }}>
            {pendingRecoveryCode}
          </Text>
        </div>

        <Alert
          type="warning"
          showIcon
          title="This key is shown only once"
          description="Write it down or print it, and store it offline away from this computer. Anyone with the recovery key can reset any account's password."
        />

        <Checkbox checked={stored} onChange={(e) => setStored(e.target.checked)}>
          I have stored the recovery key somewhere safe
        </Checkbox>
      </Space>
    </Modal>
  );
};

export default RecoveryCodeModal;
