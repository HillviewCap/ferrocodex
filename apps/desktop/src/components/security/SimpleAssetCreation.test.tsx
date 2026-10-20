import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { SimpleAssetCreation } from './SimpleAssetCreation';
import { AssetHierarchy } from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const hierarchy: AssetHierarchy[] = [
  { ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [], security_classification: SecurityClassificationLevel.CONFIDENTIAL },
];
const devicesOnly: AssetHierarchy[] = [
  { ...base, id: 5, name: 'PLC-5', asset_type: 'Device', parent_id: null, children: [], security_classification: SecurityClassificationLevel.INTERNAL },
];

const stepTitles = () => {
  const steps = within(document.querySelector('.ant-steps') as HTMLElement);
  return ['Asset Type', 'Location', 'Details', 'Confirm'].filter(title => steps.queryByText(title));
};

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

  it('has no asset type step and starts at the location step when folders exist', () => {
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getByText('Create Device')).toBeInTheDocument();
    expect(stepTitles()).toEqual(['Location', 'Details', 'Confirm']);
    for (const description of ['Choose parent folder', 'Enter device information', 'Review and create']) {
      expect(screen.getByText(description)).toBeInTheDocument();
    }
    expect(screen.queryByText('Select Asset Type')).not.toBeInTheDocument();
    expect(screen.queryByText('Folder')).not.toBeInTheDocument();
    expect(screen.getByText('Select Parent Folder')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled();
  });

  it('skips the location step when there are no folders', () => {
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={devicesOnly} />);

    expect(stepTitles()).toEqual(['Details', 'Confirm']);
    expect(screen.queryByText('Select Parent Folder')).not.toBeInTheDocument();
    expect(screen.getByText('Device Details')).toBeInTheDocument();
  });

  it('walks through every step and sends the classification', async () => {
    const onSuccess = vi.fn();
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={onSuccess} hierarchyData={hierarchy} />);

    // Step 0: location (root by default)
    expect(screen.getByText('Select the folder this device belongs in, or leave empty to create it at root level')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Step 1: details
    expect(await screen.findByText('Device Details')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
    expect(screen.getByText('Security Classification')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Device Name'), { target: { value: 'PLC-100' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Step 2: review
    expect(await screen.findByText('Review Device')).toBeInTheDocument();
    expect(screen.getByText(/PLC-100/)).toBeInTheDocument();
    expect(screen.getByText('Root level')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Create Device' }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_asset', {
        token: 'test-token',
        name: 'PLC-100',
        description: '',
        assetType: 'device',
        parentId: null,
        securityClassification: SecurityClassificationLevel.INTERNAL,
      });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalledWith({ id: 10 }));
  });

  it('defaults to the invoking folder and inherits its classification', async () => {
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} initialParentId={1} />);

    // The picker shows the selected folder
    expect(screen.getByText('Asset will be created inside the selected folder')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    expect(await screen.findByText('Inherited Classification: Confidential')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Device Name'), { target: { value: 'PLC-200' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    expect(await screen.findByText('Review Device')).toBeInTheDocument();
    expect(screen.getByText('Plant A')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Create Device' }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_asset', expect.objectContaining({
        name: 'PLC-200',
        parentId: 1,
        securityClassification: SecurityClassificationLevel.CONFIDENTIAL,
      }));
    });
  });

  it('shows the backend error when creation is rejected', async () => {
    vi.mocked(invoke).mockRejectedValue('Failed to create asset: Security classification INTERNAL is below folder');
    render(<SimpleAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={devicesOnly} />);

    fireEvent.change(screen.getByLabelText('Device Name'), { target: { value: 'PLC-300' } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Create Device' }));

    expect(await screen.findByText(/Security classification INTERNAL is below folder/)).toBeInTheDocument();
  });

  it('renders nothing when closed', () => {
    render(<SimpleAssetCreation open={false} onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);
    expect(screen.queryByText('Create Device')).not.toBeInTheDocument();
  });
});
