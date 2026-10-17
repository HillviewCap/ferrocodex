import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { SimpleAssetCreation } from './SimpleAssetCreation';
import { AssetHierarchy } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const hierarchy: AssetHierarchy[] = [
  { ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [] },
];

describe('SimpleAssetCreation', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
    });
    vi.mocked(invoke).mockResolvedValue({ id: 10 });
  });

  it('renders the step list from items and the asset type step', () => {
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getByText('Create Asset')).toBeInTheDocument();
    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Asset Type', 'Location', 'Details', 'Confirm']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Choose asset type', 'Choose parent folder', 'Enter asset information', 'Review and create']) {
      expect(steps.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByText('Select Asset Type')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('walks through every step and creates the asset', async () => {
    const onSuccess = vi.fn();
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={onSuccess} hierarchyData={hierarchy} />);

    // Step 0: pick the Device card
    fireEvent.click(screen.getByText('Device'));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Step 1: location (Alert title converted from `message`)
    expect(await screen.findByText('Select Parent Location')).toBeInTheDocument();
    expect(screen.getByText('You can select a parent folder or leave empty to create at root level')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Step 2: details
    expect(await screen.findByText('Asset Details')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
    expect(screen.getByText('Security Classification')).toBeInTheDocument();
    expect(screen.getByRole('combobox')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Asset Name'), { target: { value: 'PLC-100' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Step 3: review
    expect(await screen.findByText('Review Asset Creation')).toBeInTheDocument();
    expect(screen.getByText('Asset Type:')).toBeInTheDocument();
    expect(screen.getByText('Name:')).toBeInTheDocument();
    expect(screen.getByText('Security Classification:')).toBeInTheDocument();
    expect(screen.getByText(/PLC-100/)).toBeInTheDocument();
    expect(screen.getByText(/INTERNAL/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Create Asset' }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_asset', {
        token: 'test-token',
        name: 'PLC-100',
        description: '',
        assetType: 'device',
        parentId: null,
      });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalledWith({ id: 10 }));
  });

  it('renders nothing when closed', () => {
    render(<SimpleAssetCreation open={false} onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);
    expect(screen.queryByText('Create Asset')).not.toBeInTheDocument();
  });
});
