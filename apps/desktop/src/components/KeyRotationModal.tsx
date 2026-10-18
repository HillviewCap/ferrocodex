import React, { useCallback, useEffect, useState } from 'react';
import { Alert, Button, Checkbox, Input, Modal, Popconfirm, Progress, Space, Spin, Typography } from 'antd';
import useAuthStore from '../store/auth';
import useKeyRotationStore from '../store/keyRotation';
import { isAdministrator } from '../utils/roleUtils';
import { friendlyErrorMessage } from '../utils/keyRotationErrors';
import type { RemainingCounts } from '../types/keyRotation';

const { Text, Paragraph } = Typography;

interface KeyRotationModalProps {
  open: boolean;
  onClose: () => void;
}

const TABLE_LABELS: Array<[keyof RemainingCounts, string]> = [
  ['configuration_versions', 'Configuration versions'],
  ['vault_secrets', 'Vault secrets'],
  ['standalone_credentials', 'Standalone credentials'],
  ['firmware_versions', 'Firmware versions'],
];

const KeyRotationModal: React.FC<KeyRotationModalProps> = ({ open, onClose }) => {
  const { token, user, showRecoveryCode } = useAuthStore();
  const { status, progress, processedTotal, finalizeResult, isRunning, error, fetchStatus, begin, runUntilDone, stop, finalize } =
    useKeyRotationStore();

  const [loaded, setLoaded] = useState(false);
  const [password, setPassword] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  const [isBeginning, setIsBeginning] = useState(false);
  const [isFinalizing, setIsFinalizing] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const admin = isAdministrator(user ?? null);

  useEffect(() => {
    if (!open || !token) return;
    setLoaded(false);
    setPassword('');
    setAcknowledged(false);
    setLocalError(null);
    fetchStatus(token).finally(() => setLoaded(true));
    // Stop a running loop when the modal closes or unmounts; rotation stays resumable
    return () => stop();
  }, [open, token, fetchStatus, stop]);

  const details = status?.details ?? null;
  const usersWithoutKeypair = details?.users_without_keypair ?? [];
  const activeJobs = details?.active_analysis_jobs ?? 0;
  const remaining = progress?.remaining ?? details?.remaining ?? null;
  const failures = details?.failures ?? [];
  const finalizedByLoop = progress?.finalized === true;
  const needsFinalize =
    !finalizedByLoop &&
    (progress?.done === true || (progress === null && remaining?.total === 0)) &&
    (failures.length > 0 || (progress?.failure_count ?? 0) > 0);
  const inProgress = status?.in_progress === true && !finalizeResult && !finalizedByLoop;
  const succeeded = finalizeResult !== null || finalizedByLoop;

  const handleRun = useCallback(async () => {
    if (!token) return;
    setLocalError(null);
    await runUntilDone(token);
  }, [token, runUntilDone]);

  const handleBegin = async () => {
    if (!token) return;
    setIsBeginning(true);
    setLocalError(null);
    try {
      const response = await begin(token, password, acknowledged);
      setPassword('');
      showRecoveryCode(response.recovery_code);
      await fetchStatus(token);
      setIsBeginning(false);
      await runUntilDone(token);
    } catch (err) {
      setLocalError(friendlyErrorMessage(err));
    } finally {
      setIsBeginning(false);
    }
  };

  const handleFinalize = async () => {
    if (!token) return;
    setIsFinalizing(true);
    setLocalError(null);
    try {
      await finalize(token, true);
    } catch (err) {
      setLocalError(friendlyErrorMessage(err));
    } finally {
      setIsFinalizing(false);
    }
  };

  const canBegin =
    password.length > 0 &&
    (usersWithoutKeypair.length === 0 || acknowledged) &&
    activeJobs === 0 &&
    !isBeginning;

  const shownError = localError ?? (error ? friendlyErrorMessage(error) : null);

  const renderPreflight = () => (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Paragraph>
        Rotating the encryption key re-encrypts every configuration, vault secret, credential, and
        firmware file under a brand-new key and issues a new recovery key. Current key version:{' '}
        <Text strong>{status?.active_version ?? 'unknown'}</Text>.
      </Paragraph>
      <Alert
        type="warning"
        showIcon
        title="Older backups stay readable with the old key"
        description="Backups or copies of the database taken before the rotation can still be opened with the old password or recovery key. Rotation protects data from now on; it does not reach back into copies that already exist."
      />
      <Alert
        type="info"
        showIcon
        title="Other sessions will be signed out"
        description="Everyone else is logged out when rotation starts, and encrypted data is unavailable to them until it finishes. Keep this window open until it completes."
      />
      {activeJobs > 0 && (
        <Alert
          type="error"
          showIcon
          title="Firmware analysis is running"
          description={`${activeJobs} firmware analysis job(s) are queued or running. Wait for them to finish before rotating the key.`}
        />
      )}
      {usersWithoutKeypair.length > 0 && (
        <Alert
          type="warning"
          showIcon
          title="These users will lose access to encrypted data"
          description={
            <div>
              <ul style={{ margin: '4px 0', paddingLeft: 20 }}>
                {usersWithoutKeypair.map((u) => (
                  <li key={u.id}>
                    {u.username} ({u.role})
                  </li>
                ))}
              </ul>
              <Text>
                An administrator must use Reset password for each of them after the rotation.
              </Text>
            </div>
          }
        />
      )}
      <div>
        <Text strong>Your password</Text>
        <Input.Password
          aria-label="Administrator password"
          placeholder="Enter your password to confirm"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
          style={{ marginTop: 4 }}
        />
      </div>
      {usersWithoutKeypair.length > 0 && (
        <Checkbox checked={acknowledged} onChange={(e) => setAcknowledged(e.target.checked)}>
          I understand these users will lose access until their password is reset
        </Checkbox>
      )}
    </Space>
  );

  const renderCounts = () =>
    remaining && (
      <ul style={{ margin: 0, paddingLeft: 20 }}>
        {TABLE_LABELS.map(([key, label]) => (
          <li key={key}>
            {label}: {remaining[key]} remaining
          </li>
        ))}
      </ul>
    );

  const renderProgress = () => {
    const total = processedTotal + (remaining?.total ?? 0);
    const percent = total === 0 ? 100 : Math.round((processedTotal / total) * 100);
    return (
      <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        <Paragraph>
          {isRunning
            ? 'Re-encrypting data. Keep this window open.'
            : needsFinalize
              ? 'Re-encryption finished, but some items could not be read.'
              : 'Key rotation is in progress. Encrypted data is unavailable to other users until it finishes.'}
        </Paragraph>
        <Progress percent={percent} status={isRunning ? 'active' : 'normal'} />
        <Text type="secondary">
          {processedTotal} re-encrypted, {remaining?.total ?? 0} remaining
        </Text>
        {renderCounts()}
        {needsFinalize && (
          <Alert
            type="error"
            showIcon
            title="Some items could not be re-encrypted"
            description={
              <div>
                <ul style={{ margin: '4px 0', paddingLeft: 20 }}>
                  {failures.map((f) => (
                    <li key={f.id}>
                      {f.table_name} row {f.row_id}: {f.error}
                    </li>
                  ))}
                </ul>
                <Text>
                  Finalizing discards these items permanently; they stay unreadable. Resume to retry them first.
                </Text>
              </div>
            }
          />
        )}
      </Space>
    );
  };

  const renderSuccess = () => (
    <Alert
      type="success"
      showIcon
      title="Encryption key rotated"
      description={
        <div>
          <div>
            Key version {finalizeResult?.retired_version ?? progress?.from_version} retired; version{' '}
            {finalizeResult?.active_version ?? progress?.to_version} is now active.
          </div>
          {finalizeResult && (
            <>
              <div>Unreadable items discarded: {finalizeResult.discarded}</div>
              <div>Orphaned files removed: {finalizeResult.orphaned_files_removed}</div>
            </>
          )}
          <div style={{ marginTop: 8 }}>
            Other users can sign in again. Users who lost access need an administrator to use Reset password.
          </div>
        </div>
      }
    />
  );

  const renderBody = () => {
    if (!loaded) return <Spin />;
    if (succeeded) return renderSuccess();
    if (inProgress || isBeginning) return renderProgress();
    return renderPreflight();
  };

  const renderFooter = () => {
    if (succeeded) {
      return [
        <Button key="close" type="primary" onClick={onClose}>
          Close
        </Button>,
      ];
    }
    if (inProgress) {
      return [
        <Button key="close" onClick={onClose}>
          Close
        </Button>,
        isRunning ? (
          <Button key="pause" onClick={stop}>
            Pause
          </Button>
        ) : needsFinalize ? (
          <Space key="actions">
            <Button onClick={handleRun}>Resume</Button>
            <Popconfirm
              title="Discard unreadable items?"
              description="These items cannot be recovered after finalizing."
              okText="Discard and finalize"
              cancelText="Cancel"
              onConfirm={handleFinalize}
            >
              <Button danger loading={isFinalizing}>
                Finalize and discard unreadable items
              </Button>
            </Popconfirm>
          </Space>
        ) : (
          <Button key="resume" type="primary" onClick={handleRun}>
            Resume
          </Button>
        ),
      ];
    }
    return [
      <Button key="cancel" onClick={onClose}>
        Cancel
      </Button>,
      <Button key="begin" type="primary" danger disabled={!canBegin} loading={isBeginning} onClick={handleBegin}>
        Begin rotation
      </Button>,
    ];
  };

  if (!admin) return null;

  return (
    <Modal
      title="Rotate Encryption Key"
      open={open}
      onCancel={onClose}
      footer={renderFooter()}
      maskClosable={false}
      width={600}
    >
      {shownError && (
        <Alert type="error" showIcon title={shownError} style={{ marginBottom: 16 }} />
      )}
      {renderBody()}
    </Modal>
  );
};

export default KeyRotationModal;
