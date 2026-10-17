import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import FileIntegrityDisplay from './FileIntegrityDisplay';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const cleanResult = {
  sha256Hash: 'abc123def456',
  fileSize: 2048,
  isVerified: true,
  securityScanPassed: true,
  detectedIssues: [] as string[],
};

describe('FileIntegrityDisplay', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'validate_file_upload') return cleanResult;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('verifies on mount and exposes details in the collapse panel', async () => {
    const onComplete = vi.fn();
    render(<FileIntegrityDisplay filePath="/tmp/plc.cfg" onVerificationComplete={onComplete} />);

    expect(screen.getByText('File Integrity Verification')).toBeInTheDocument();
    expect(await screen.findByText('File integrity verified - All checks passed')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('validate_file_upload', { token: 'test-token', filePath: '/tmp/plc.cfg' });
    expect(onComplete).toHaveBeenCalledWith({ ...cleanResult, isVerified: true });

    fireEvent.click(screen.getByText('Verification Details'));
    expect(screen.getByText('SHA-256 Hash:')).toBeInTheDocument();
    expect(screen.getByText(/abc123def456/)).toBeInTheDocument();
    expect(screen.getByText('Security Scan:')).toBeInTheDocument();
    expect(screen.getByText('Passed')).toBeInTheDocument();
    expect(screen.getByText('Path: /tmp/plc.cfg')).toBeInTheDocument();
    expect(screen.getByText('Size: 2 KB')).toBeInTheDocument();
  });

  it('flags a hash mismatch against the expected hash', async () => {
    render(<FileIntegrityDisplay filePath="/tmp/plc.cfg" expectedHash="expected999" showDetails={false} />);

    expect(await screen.findByText('Hash mismatch detected - Integrity compromised')).toBeInTheDocument();
    expect(screen.getByText('Hash Mismatch')).toBeInTheDocument();
    expect(screen.getByText('Mismatch')).toBeInTheDocument();
    expect(screen.getByText('expected999')).toBeInTheDocument();
  });

  it('lists detected security issues', async () => {
    vi.mocked(invoke).mockResolvedValue({
      ...cleanResult,
      securityScanPassed: false,
      detectedIssues: ['Embedded macro detected'],
    });
    render(<FileIntegrityDisplay filePath="/tmp/plc.cfg" showDetails={false} />);

    expect(await screen.findByText('Security scan detected issues')).toBeInTheDocument();
    expect(screen.getByText('Security Issues Detected')).toBeInTheDocument();
    expect(screen.getByText('Embedded macro detected')).toBeInTheDocument();
  });

  it('shows the error state with a retry button, and the idle state without auto verify', async () => {
    const onError = vi.fn();
    vi.mocked(invoke).mockRejectedValue(new Error('hashing failed'));
    const { unmount } = render(<FileIntegrityDisplay filePath="/tmp/plc.cfg" onVerificationError={onError} />);

    expect(await screen.findByText('hashing failed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry Verification/ })).toBeInTheDocument();
    await waitFor(() => expect(onError).toHaveBeenCalledWith('hashing failed'));
    unmount();

    render(<FileIntegrityDisplay filePath="/tmp/plc.cfg" autoVerify={false} />);
    expect(screen.getByText('Ready for verification')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Verify/ })).toBeInTheDocument();
  });
});
