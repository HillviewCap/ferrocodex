import React from 'react';
import { Form, Input } from 'antd';

// Matches MIN_PASSPHRASE_LEN in src-tauri/src/vault/export.rs
export const MIN_EXPORT_PASSPHRASE_LENGTH = 12;

// Passphrase and confirmation fields for an exported vault. Render inside a
// Form; the value is read from the `vaultPassphrase` field.
const ExportPassphraseFields: React.FC = () => (
  <>
    <Form.Item
      label="Vault Export Passphrase"
      name="vaultPassphrase"
      extra="Vault secrets are encrypted with this passphrase and it is needed to import them. It can't be recovered, so share it separately from the export."
      rules={[
        { required: true, message: 'Please enter an export passphrase' },
        { min: MIN_EXPORT_PASSPHRASE_LENGTH, message: `Passphrase must be at least ${MIN_EXPORT_PASSPHRASE_LENGTH} characters` },
      ]}
    >
      <Input.Password placeholder="Export passphrase" autoComplete="new-password" />
    </Form.Item>
    <Form.Item
      label="Confirm Passphrase"
      name="confirmVaultPassphrase"
      dependencies={['vaultPassphrase']}
      rules={[
        { required: true, message: 'Please confirm the passphrase' },
        ({ getFieldValue }) => ({
          validator(_, value) {
            if (!value || getFieldValue('vaultPassphrase') === value) {
              return Promise.resolve();
            }
            return Promise.reject(new Error('Passphrases do not match'));
          },
        }),
      ]}
    >
      <Input.Password placeholder="Confirm passphrase" autoComplete="new-password" />
    </Form.Item>
  </>
);

export default ExportPassphraseFields;
