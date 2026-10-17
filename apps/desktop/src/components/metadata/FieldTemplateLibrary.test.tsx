import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import FieldTemplateLibrary from './FieldTemplateLibrary';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const templates = [
  {
    id: 'ip_address',
    name: 'ip_address',
    display_name: 'IP Address',
    description: 'IPv4 address of the device management interface',
    category: 'network',
    data_type: 'string',
    validation_rules: { pattern: '^\\d+\\.\\d+\\.\\d+\\.\\d+$' },
  },
  {
    id: 'rack_position',
    name: 'rack_position',
    display_name: 'Rack Position',
    description: 'Position of the device in the rack',
    category: 'physical',
    data_type: 'number',
    unit: 'U',
    options: ['1', '2'],
  },
];

describe('FieldTemplateLibrary', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    // The component assigns random popularity and sorts by it; pin it so card order is stable
    vi.spyOn(Math, 'random').mockReturnValue(0.5);
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'list_metadata_field_templates') return templates;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads templates on mount and renders a card per template', async () => {
    render(<FieldTemplateLibrary />);

    expect(screen.getByText('Field Template Library')).toBeInTheDocument();
    expect(await screen.findByText('IP Address')).toBeInTheDocument();
    expect(screen.getByText('Rack Position')).toBeInTheDocument();
    expect(screen.getByText('2 templates found')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('list_metadata_field_templates', { token: 'test-token' });

    // Category filter buttons plus per-card category tags
    expect(screen.getByRole('button', { name: /All Categories/ })).toBeInTheDocument();
    expect(screen.getAllByText('Network').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Physical').length).toBeGreaterThan(0);
  });

  it('filters by category', async () => {
    render(<FieldTemplateLibrary />);
    await screen.findByText('IP Address');

    fireEvent.click(screen.getByRole('button', { name: /Physical/ }));

    await waitFor(() => {
      expect(screen.queryByText('IP Address')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Rack Position')).toBeInTheDocument();
    expect(screen.getByText('1 templates found in Physical')).toBeInTheDocument();
  });

  it('opens the preview modal for a template', async () => {
    render(<FieldTemplateLibrary />);
    await screen.findByText('IP Address');

    const previewButtons = screen.getAllByRole('button', { name: 'eye' });
    fireEvent.click(previewButtons[0]);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(screen.getByText('Field Name:')).toBeInTheDocument();
    expect(screen.getByText('Validation Rules:')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Copy JSON/ })).toBeInTheDocument();
  });
});
