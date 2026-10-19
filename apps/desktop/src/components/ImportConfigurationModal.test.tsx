import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import ImportConfigurationModal from './ImportConfigurationModal';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

describe('ImportConfigurationModal', () => {
  useConsoleGuard({
    // Pre-existing rc-field-form warning, not an antd deprecation: the modal's useForm
    // instance is reset while the file-selection step (which has no <Form>) is shown.
    ignore: [/Instance created by `useForm` is not connected to any Form element/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
  });

  it('renders the steps from items (title + content) and the file-selection step', () => {
    render(<ImportConfigurationModal visible onCancel={vi.fn()} onSuccess={vi.fn()} />);

    expect(screen.getByText('Import Configuration')).toBeInTheDocument();
    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Select File', 'Asset Details', 'Complete']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const content of ['Choose a configuration file', 'Enter asset information', 'Import successful']) {
      expect(steps.getByText(content)).toBeInTheDocument();
    }
    expect(screen.getByText('Select Configuration File')).toBeInTheDocument();
    expect(screen.getByText('Click to select configuration file')).toBeInTheDocument();
    expect(screen.getByText('Any File Type')).toBeInTheDocument();
    expect(screen.getByText(/Any configuration file is accepted/)).toBeInTheDocument();
  });

  it('accepts a file of any type and moves on to the asset details', async () => {
    vi.mocked(dialog.open).mockResolvedValue('/projects/line1.apx');
    vi.mocked(invoke).mockResolvedValue({ name: 'line1.apx', size: 2048, content_type: 'application/octet-stream', hash: 'h' });
    render(<ImportConfigurationModal visible onCancel={vi.fn()} onSuccess={vi.fn()} />);

    fireEvent.click(screen.getByText('Click to select configuration file'));

    expect(await screen.findByText('Asset Information')).toBeInTheDocument();
    expect(screen.getByText('line1.apx')).toBeInTheDocument();
    expect(dialog.open).toHaveBeenCalledWith({ multiple: false, filters: [{ name: 'All Files', extensions: ['*'] }] });
  });

  it('closes through onCancel and is not mounted when hidden', () => {
    const onCancel = vi.fn();
    render(<ImportConfigurationModal visible onCancel={onCancel} onSuccess={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('renders nothing when initially hidden', () => {
    render(<ImportConfigurationModal visible={false} onCancel={vi.fn()} onSuccess={vi.fn()} />);
    expect(screen.queryByText('Select Configuration File')).not.toBeInTheDocument();
  });
});
