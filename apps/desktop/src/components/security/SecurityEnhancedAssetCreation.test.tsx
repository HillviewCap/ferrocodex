import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SecurityEnhancedAssetCreation } from './SecurityEnhancedAssetCreation';
import { AssetHierarchy } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const hierarchy: AssetHierarchy[] = [
  { ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [] },
];

describe('SecurityEnhancedAssetCreation', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
    });
  });

  it('renders the step list from items and the asset type step', () => {
    render(<SecurityEnhancedAssetCreation open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getByText('Create Secure Asset')).toBeInTheDocument();
    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Asset Type', 'Location', 'Details', 'Confirm']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const description of ['Choose asset type', 'Choose parent folder', 'Enter asset information', 'Review and create']) {
      expect(steps.getByText(description)).toBeInTheDocument();
    }
    expect(screen.getByText('Select Asset Type')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('advances to the location step when an asset type is preselected', async () => {
    render(
      <SecurityEnhancedAssetCreation
        open
        onCancel={vi.fn()}
        onSuccess={vi.fn()}
        hierarchyData={hierarchy}
        initialAssetType="Folder"
      />,
    );

    const next = screen.getByRole('button', { name: 'Next' });
    expect(next).toBeEnabled();
    fireEvent.click(next);

    expect(await screen.findByText('Select Parent Location')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Place in root level/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeInTheDocument();
  });

  it('renders nothing when closed', () => {
    render(<SecurityEnhancedAssetCreation open={false} onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);
    expect(screen.queryByText('Create Secure Asset')).not.toBeInTheDocument();
  });
});
