import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BranchCard from './BranchCard';
import { BranchInfo } from '../types/branches';
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

describe('BranchCard', () => {
  useConsoleGuard();

  it('renders branch summary and the vertical action column', () => {
    const handlers = {
      onViewDetails: vi.fn(),
      onSelectBranch: vi.fn(),
      onViewHistory: vi.fn(),
      onImportVersion: vi.fn(),
      onExportLatestVersion: vi.fn(),
      onPromoteToSilver: vi.fn(),
    };
    render(<BranchCard branch={branch} versionCount={3} latestVersionNumber="branch-v3" {...handlers} />);

    expect(screen.getByText(/test-new-parameters/)).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('engineer')).toBeInTheDocument();
    expect(screen.getByText('Branched from:')).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
    expect(screen.getByText('3 versions')).toBeInTheDocument();
    expect(screen.getByText('Latest: branch-v3')).toBeInTheDocument();
    expect(screen.getByText('Experimenting with PID values')).toBeInTheDocument();

    // Actions are laid out in a vertical Space (direction -> orientation)
    expect(document.querySelector('.ant-space-vertical')).not.toBeNull();
    for (const [label, handler] of [
      ['View Details', handlers.onViewDetails],
      ['Import Version', handlers.onImportVersion],
      ['Export Latest', handlers.onExportLatestVersion],
      ['Promote to Silver', handlers.onPromoteToSilver],
      ['View History', handlers.onViewHistory],
      ['Select Branch', handlers.onSelectBranch],
    ] as const) {
      fireEvent.click(screen.getByRole('button', { name: new RegExp(label) }));
      expect(handler).toHaveBeenCalledWith(branch);
    }
  });

  it('hides actions when showActions is false and flags archived parents', () => {
    render(
      <BranchCard
        branch={{ ...branch, parent_version_status: 'Archived', is_active: false }}
        showActions={false}
        onViewDetails={vi.fn()}
      />,
    );

    expect(screen.queryByRole('button', { name: /View Details/ })).not.toBeInTheDocument();
    expect(screen.getByText('Inactive')).toBeInTheDocument();
    expect(screen.getByTitle('Parent version v3 is Archived')).toBeInTheDocument();
  });
});
