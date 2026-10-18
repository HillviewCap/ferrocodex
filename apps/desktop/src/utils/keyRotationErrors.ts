export const KEY_ROTATION_GATE_ERROR =
  'Encryption key rotation in progress; an administrator must finish it';

export const KEY_ROTATION_GATE_MESSAGE =
  'Encrypted data is unavailable until an administrator finishes rotating the encryption key.';

export const isKeyRotationGateError = (error: unknown): boolean =>
  String(error ?? '').includes(KEY_ROTATION_GATE_ERROR);

// Turns the backend's gate error into a friendly message; other errors pass through as text.
export const friendlyErrorMessage = (error: unknown): string =>
  isKeyRotationGateError(error) ? KEY_ROTATION_GATE_MESSAGE : String(error ?? 'Unknown error');
