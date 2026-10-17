import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ImportConfigurationModal from './ImportConfigurationModal';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

// NOTE: only the first step is exercised. Advancing to the "Asset Details" step (which
// holds the Descriptions `items` conversion) is blocked by a pre-existing bug in
// handleFileSelect: validateFileExtension/validateFileSize return `{ isValid }` objects,
// which the component treats as error strings, so every file selection is rejected.

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
    expect(screen.getByText('Supported File Types')).toBeInTheDocument();
    expect(screen.getByText(/JSON, XML, YAML, TXT/)).toBeInTheDocument();
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
