import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import ImportTemplateManager from './ImportTemplateManager';
import useBulkImportStore from '../../store/bulk';
import { ImportTemplate } from '../../types/bulk';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const templates: ImportTemplate[] = [
  {
    id: 1,
    template_name: 'Standard devices',
    template_type: 'assets',
    field_mapping: {},
    required_fields: ['name', 'asset_type', 'parent_name', 'description'],
    optional_fields: ['metadata'],
    validation_rules: {},
    created_by: 1,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
];

describe('ImportTemplateManager', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useBulkImportStore.getState().reset();
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_import_templates') return templates;
      if (cmd === 'generate_import_template_csv') return 'name,asset_type\n';
      if (cmd === 'delete_import_template') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads templates for the selected type and renders them in the table', async () => {
    const onSelectTemplate = vi.fn();
    render(<ImportTemplateManager visible onClose={vi.fn()} onSelectTemplate={onSelectTemplate} />);

    expect(screen.getByText('Import Template Manager')).toBeInTheDocument();
    expect(screen.getByText('Import Templates')).toBeInTheDocument();
    expect(await screen.findByText('Standard devices')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_import_templates', { templateType: 'assets' });

    expect(screen.getByText(/Created: /)).toBeInTheDocument();
    expect(screen.getByText('assets')).toBeInTheDocument();
    expect(screen.getByText('name')).toBeInTheDocument();
    expect(screen.getByText('asset_type')).toBeInTheDocument();
    expect(screen.getByText('+1 more')).toBeInTheDocument();
    expect(screen.getByText('1-1 of 1 templates')).toBeInTheDocument();

    expect(screen.getByRole('button', { name: /Create Template/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Download Basic CSV/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Select' }));
    expect(onSelectTemplate).toHaveBeenCalledWith(templates[0]);
  });

  it('opens the create-template modal with its form fields', async () => {
    render(<ImportTemplateManager visible onClose={vi.fn()} />);
    await screen.findByText('Standard devices');

    fireEvent.click(screen.getByRole('button', { name: /Create Template/ }));

    expect(await screen.findByText('Create Import Template')).toBeInTheDocument();
    expect(screen.getByLabelText('Template Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Default Asset Type')).toBeInTheDocument();
    // "Required Fields" is also a table column header, so expect the form label as a second copy
    expect(screen.getAllByText('Required Fields')).toHaveLength(2);
    expect(screen.getByText('Optional Fields')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
  });

  it('shows the empty state and surfaces store errors as a notification', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_import_templates') return [];
      return undefined;
    });
    render(<ImportTemplateManager visible onClose={vi.fn()} />);
    expect(await screen.findByText(/No assets templates found/)).toBeInTheDocument();

    // The store's error is rendered as a notification and then cleared by the component
    act(() => {
      useBulkImportStore.setState({ error: 'templates unavailable' });
    });
    expect(await screen.findByText('Template Operation Failed')).toBeInTheDocument();
    expect(screen.getByText('templates unavailable')).toBeInTheDocument();
    await waitFor(() => expect(useBulkImportStore.getState().error).toBeNull());
  });
});
