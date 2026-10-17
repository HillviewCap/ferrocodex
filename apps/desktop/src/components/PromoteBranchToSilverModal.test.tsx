import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PromoteBranchToSilverModal from './PromoteBranchToSilverModal';
import { BranchInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const branch: BranchInfo = {
  id: 5,
  name: 'test-new-parameters',
  description: 'Experimenting with PID values',
  asset_id: 1,
  parent_version_id: 3,
  parent_version_number: 'v3',
  parent_version_status: 'Approved',
  created_by: 1,
  created_by_username: 'engineer',
  created_at: '2024-02-10T09:30:00Z',
  updated_at: '2024-02-10T09:30:00Z',
  is_active: true,
  version_count: 2,
};

describe('PromoteBranchToSilverModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    useAssetStore.setState({ selectedAsset: null });
    vi.mocked(invoke).mockResolvedValue(42);
  });

  it('renders alerts, branch information and the promotion path', () => {
    render(<PromoteBranchToSilverModal visible onCancel={vi.fn()} onSuccess={vi.fn()} branch={branch} assetId={1} />);

    expect(screen.getByText('Promote Branch to Silver')).toBeInTheDocument();
    expect(screen.getByText('Promote to Silver Status')).toBeInTheDocument();
    expect(screen.getByText('What happens next?')).toBeInTheDocument();
    expect(screen.getByText(/A new configuration version will be created/)).toBeInTheDocument();
    expect(screen.getByText('Branch Information')).toBeInTheDocument();
    expect(screen.getByText('test-new-parameters')).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('Experimenting with PID values')).toBeInTheDocument();
    expect(screen.getByText('Created by: engineer')).toBeInTheDocument();

    expect(screen.getByText('Promotion Path:')).toBeInTheDocument();
    for (const stage of ['Branch', 'Silver', 'Approved', 'Golden']) {
      expect(screen.getByText(stage)).toBeInTheDocument();
    }
    expect(screen.getByLabelText('Promotion Notes')).toBeInTheDocument();
  });

  it('requires notes and then promotes the branch', async () => {
    const onSuccess = vi.fn();
    render(<PromoteBranchToSilverModal visible onCancel={vi.fn()} onSuccess={onSuccess} branch={branch} assetId={1} />);

    fireEvent.click(screen.getByRole('button', { name: /Promote to Silver$/ }));
    expect(await screen.findByText('Please provide promotion notes')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText('Promotion Notes'), { target: { value: 'Ready for review' } });
    fireEvent.click(screen.getByRole('button', { name: /Promote to Silver$/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('promote_branch_to_silver', {
        token: 'test-token',
        branchId: 5,
        promotionNotes: 'Ready for review',
      });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalled());
  });
});
