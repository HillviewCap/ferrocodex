import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import QuickFirmwareUploadModal from './QuickFirmwareUploadModal';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

const fileInfo = { name: 'firmware.bin', size: 4096, path: '/tmp/firmware.bin', type: 'bin' };

describe('QuickFirmwareUploadModal', () => {
  useConsoleGuard({
    // Pre-existing rc-field-form warning, not an antd deprecation: the modal's useForm
    // instance is used while a step without the <Form> is displayed. Form wiring was not
    // part of the antd migration.
    ignore: [/Instance created by `useForm` is not connected to any Form element/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(dialog.open).mockResolvedValue('/tmp/firmware.bin');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_file_info') return fileInfo;
      if (cmd === 'upload_firmware') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders the step list from items and the file picker step', () => {
    render(<QuickFirmwareUploadModal visible onCancel={vi.fn()} onSuccess={vi.fn()} assetId={7} assetName="PLC-001" />);

    expect(screen.getByText('Quick Firmware Upload')).toBeInTheDocument();
    for (const title of ['Select File', 'Configure Upload', 'Complete']) {
      expect(screen.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Choose firmware file', 'Add firmware details', 'Upload successful']) {
      expect(screen.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByText('Select Firmware File')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Browse Files' })).toBeInTheDocument();
  });

  it('moves to the configure step after picking a file and uploads it', async () => {
    const onSuccess = vi.fn();
    render(<QuickFirmwareUploadModal visible onCancel={vi.fn()} onSuccess={onSuccess} assetId={7} assetName="PLC-001" />);

    fireEvent.click(screen.getByRole('button', { name: 'Browse Files' }));

    expect(await screen.findByText('firmware.bin')).toBeInTheDocument();
    expect(screen.getByLabelText('Vendor')).toBeInTheDocument();
    expect(screen.getByLabelText('Model')).toBeInTheDocument();
    expect(screen.getByLabelText('Firmware Version')).toBeInTheDocument();
    expect(screen.getByLabelText('Release Notes')).toBeInTheDocument();
    expect(screen.getByLabelText('Security Classification')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Vendor'), { target: { value: 'Siemens' } });
    fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'S7-1200' } });
    fireEvent.change(screen.getByLabelText('Firmware Version'), { target: { value: '4.2.3' } });
    fireEvent.click(screen.getByRole('button', { name: 'Upload Firmware' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('upload_firmware', {
        token: 'test-token',
        assetId: 7,
        filePath: '/tmp/firmware.bin',
        vendor: 'Siemens',
        model: 'S7-1200',
        version: '4.2.3',
        notes: null,
      });
    });

    expect(await screen.findByText('Upload Successful!', {}, { timeout: 3000 })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Complete' }));
    expect(onSuccess).toHaveBeenCalled();
  });

  it('renders nothing when hidden', () => {
    render(<QuickFirmwareUploadModal visible={false} onCancel={vi.fn()} onSuccess={vi.fn()} assetId={7} />);
    expect(screen.queryByText('Quick Firmware Upload')).not.toBeInTheDocument();
  });
});
