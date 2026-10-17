import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { CreateAssetModal } from './CreateAssetModal';
import { AssetHierarchy } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const hierarchy: AssetHierarchy[] = [
  { ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [] },
];

describe('CreateAssetModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
  });

  it('renders the step list from items with titles and descriptions', () => {
    render(<CreateAssetModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getByText('Create New Asset')).toBeInTheDocument();
    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Asset Type', 'Location', 'Details', 'Confirm']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Choose asset type', 'Choose parent folder', 'Enter asset information', 'Review and create']) {
      expect(steps.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('walks through the steps when an asset type is preselected', async () => {
    render(
      <CreateAssetModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} initialAssetType="Folder" />,
    );

    const next = screen.getByRole('button', { name: 'Next' });
    expect(next).toBeEnabled();
    fireEvent.click(next);

    expect(await screen.findByText('Parent Location')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Place in root level/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(await screen.findByText('Asset Information')).toBeInTheDocument();
    expect(screen.getByLabelText('Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'New Folder' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    expect(await screen.findByText('Review and Confirm')).toBeInTheDocument();
    expect(screen.getByText('New Folder')).toBeInTheDocument();
    expect(screen.getByText('Root Level')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Previous' }));
    expect(await screen.findByText('Asset Information')).toBeInTheDocument();
  });

  it('creates the asset from the confirm step', async () => {
    const onSuccess = vi.fn();
    vi.mocked(invoke).mockResolvedValue({ id: 10 });
    render(
      <CreateAssetModal open onCancel={vi.fn()} onSuccess={onSuccess} hierarchyData={hierarchy} initialAssetType="Device" />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await screen.findByText('Parent Location');
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await screen.findByText('Asset Information');
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'PLC-100' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await screen.findByText('Review and Confirm');

    fireEvent.click(screen.getByRole('button', { name: 'Create Device' }));

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
    render(<CreateAssetModal open={false} onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);
    expect(screen.queryByText('Create New Asset')).not.toBeInTheDocument();
  });
});
