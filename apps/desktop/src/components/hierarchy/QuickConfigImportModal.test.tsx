import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import QuickConfigImportModal from './QuickConfigImportModal';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

const fileInfo = { name: 'plc.cfg', size: 2048, path: '/tmp/plc.cfg', type: 'cfg' };

describe('QuickConfigImportModal', () => {
  useConsoleGuard({
    // Pre-existing rc-field-form warning, not an antd deprecation: the modal's useForm
    // instance is used while a step without the <Form> is displayed. Form wiring was not
    // part of the antd migration.
    ignore: [/Instance created by `useForm` is not connected to any Form element/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(dialog.open).mockResolvedValue('/tmp/plc.cfg');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_file_info') return fileInfo;
      if (cmd === 'import_configuration_for_asset') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders the step list from items and the file picker step', () => {
    render(<QuickConfigImportModal visible onCancel={vi.fn()} onSuccess={vi.fn()} assetId={7} assetName="PLC-001" />);

    expect(screen.getByText('Quick Configuration Import')).toBeInTheDocument();
    for (const title of ['Select File', 'Configure Import', 'Complete']) {
      expect(screen.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Choose configuration file', 'Add details and notes', 'Import successful']) {
      expect(screen.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByText('Select Configuration File')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Browse Files' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('moves to the configure step after picking a file and imports it', async () => {
    const onSuccess = vi.fn();
    const onCancel = vi.fn();
    render(<QuickConfigImportModal visible onCancel={onCancel} onSuccess={onSuccess} assetId={7} assetName="PLC-001" />);

    fireEvent.click(screen.getByRole('button', { name: 'Browse Files' }));

    expect(await screen.findByText('plc.cfg')).toBeInTheDocument();
    expect(screen.getByText('2 KB')).toBeInTheDocument();
    expect(screen.getByLabelText('Target Asset')).toBeInTheDocument();
    expect(screen.getByLabelText('Version Notes')).toBeInTheDocument();
    expect(screen.getByLabelText('Security Classification')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_file_info', { path: '/tmp/plc.cfg' });

    fireEvent.change(screen.getByLabelText('Version Notes'), { target: { value: 'Initial import' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import Configuration' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('import_configuration_for_asset', {
        token: 'test-token',
        assetId: 7,
        filePath: '/tmp/plc.cfg',
        versionNotes: 'Initial import',
        classification: 'Internal',
      });
    });

    expect(await screen.findByText('Import Successful!', {}, { timeout: 3000 })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Complete' }));
    expect(onSuccess).toHaveBeenCalled();
    expect(onCancel).toHaveBeenCalled();
  });

  it('renders nothing when hidden', () => {
    render(<QuickConfigImportModal visible={false} onCancel={vi.fn()} onSuccess={vi.fn()} assetId={7} />);
    expect(screen.queryByText('Quick Configuration Import')).not.toBeInTheDocument();
  });
});
