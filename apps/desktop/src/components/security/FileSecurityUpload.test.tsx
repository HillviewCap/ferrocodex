import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import FileSecurityUpload from './FileSecurityUpload';
import { SecurityClassificationLevel } from '../../types/security';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const handlers = () => ({
  onUploadComplete: vi.fn(),
  onUploadProgress: vi.fn(),
  onUploadError: vi.fn(),
});

const pickFile = (file: File) => {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement;
  fireEvent.change(input, { target: { files: [file] } });
};

describe('FileSecurityUpload', () => {
  describe('before any file is selected', () => {
    useConsoleGuard();

    beforeEach(() => {
      vi.clearAllMocks();
      useAuthStore.setState({ token: 'test-token' });
    });

    it('renders the classification selector and the upload area', () => {
      render(<FileSecurityUpload {...handlers()} acceptedFileTypes={['.cfg', '.bin']} maxFileSize={5 * 1024 * 1024} />);

      expect(screen.getByText('Secure File Upload')).toBeInTheDocument();
      expect(screen.getByText('Security Classification:')).toBeInTheDocument();
      expect(screen.getByRole('combobox')).toBeInTheDocument();
      expect(screen.getByText('Click or drag files to this area to upload')).toBeInTheDocument();
      expect(screen.getByText(/Accepted types: \.cfg, \.bin/)).toBeInTheDocument();
      expect(screen.getByText(/Maximum file size: 5MB/)).toBeInTheDocument();
      expect(screen.queryByText('Upload Progress:')).not.toBeInTheDocument();
    });

    it('hides the selector when a classification is preset and rejects disallowed files', async () => {
      const props = handlers();
      render(
        <FileSecurityUpload
          {...props}
          autoClassification={SecurityClassificationLevel.INTERNAL}
          acceptedFileTypes={['.cfg']}
        />,
      );

      expect(screen.queryByText('Security Classification:')).not.toBeInTheDocument();

      pickFile(new File(['x'], 'firmware.exe', { type: 'application/octet-stream' }));
      await waitFor(() => {
        expect(props.onUploadError).toHaveBeenCalledWith('File type not allowed. Accepted types: .cfg');
      });
      expect(invoke).not.toHaveBeenCalled();
    });
  });

  describe('with a selected file', () => {
    useConsoleGuard({
      // List migration was explicitly deferred by d052492; Listy is not a drop-in.
      ignore: [/\[antd: List\]/],
    });

    beforeEach(() => {
      vi.clearAllMocks();
      useAuthStore.setState({ token: 'test-token' });
      // The demo validation uses Math.random for pass/fail; pin it so the file passes
      vi.spyOn(Math, 'random').mockReturnValue(0.5);
      vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'sanitize_filename') return args?.filename;
        throw new Error(`Unexpected command: ${cmd}`);
      });
    });

    it('validates the file, lists it, and completes the upload', async () => {
      const props = handlers();
      render(<FileSecurityUpload {...props} autoClassification={SecurityClassificationLevel.INTERNAL} />);

      pickFile(new File(['configuration'], 'plc.cfg', { type: 'text/plain' }));

      expect(await screen.findByText('plc.cfg')).toBeInTheDocument();
      expect(screen.getByText('Upload Progress:')).toBeInTheDocument();
      expect(screen.getByText('INTERNAL')).toBeInTheDocument();
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('sanitize_filename', { token: 'test-token', filename: 'plc.cfg' });
      });

      // The demo pipeline completes on a 1.5s timer
      expect(await screen.findByText('Validation complete', {}, { timeout: 4000 })).toBeInTheDocument();
      expect(screen.getByText('1/1 files processed')).toBeInTheDocument();
      expect(screen.getByText('All files validated')).toBeInTheDocument();
      expect(screen.getByText('Security: Passed')).toBeInTheDocument();

      const complete = screen.getByRole('button', { name: /Complete Upload \(1 files\)/ });
      expect(complete).toBeEnabled();
      fireEvent.click(complete);
      expect(props.onUploadComplete).toHaveBeenCalledWith([
        expect.objectContaining({ fileSize: 13, isVerified: true, securityScanPassed: true }),
      ]);
    });
  });
});
