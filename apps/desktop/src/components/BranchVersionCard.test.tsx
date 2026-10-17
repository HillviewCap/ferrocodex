import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BranchVersionCard from './BranchVersionCard';
import { BranchVersionInfo } from '../types/branches';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const version: BranchVersionInfo = {
  id: 7,
  branch_id: 5,
  branch_name: 'test-new-parameters',
  version_id: 70,
  branch_version_number: 'branch-v2',
  is_latest: true,
  is_branch_latest: true,
  created_at: '2024-02-11T10:00:00Z',
  asset_id: 1,
  version_number: 'v4',
  file_name: 'plc-config.json',
  file_size: 1536,
  content_hash: 'abc',
  author: 1,
  author_username: 'engineer',
  notes: 'Second branch iteration',
  version_created_at: '2024-02-11T10:00:00Z',
};

describe('BranchVersionCard', () => {
  useConsoleGuard();

  it('renders version metadata inside the card body (bodyStyle -> styles.body)', () => {
    render(<BranchVersionCard version={version} onView={vi.fn()} onDownload={vi.fn()} />);

    expect(screen.getByText('branch-v2')).toBeInTheDocument();
    expect(screen.getByText('Latest')).toBeInTheDocument();
    expect(screen.getByText('plc-config.json')).toBeInTheDocument();
    expect(screen.getByText('engineer')).toBeInTheDocument();
    expect(screen.getByText('1.5 KB')).toBeInTheDocument();
    expect(screen.getByText('Main: v4')).toBeInTheDocument();
    expect(screen.getByText('Second branch iteration')).toBeInTheDocument();

    const body = document.querySelector('.ant-card-body') as HTMLElement;
    expect(body).toHaveStyle({ padding: '12px 16px' });
  });

  it('wires the view/download buttons and selection checkbox', () => {
    const onView = vi.fn();
    const onDownload = vi.fn();
    const onSelect = vi.fn();
    render(
      <BranchVersionCard version={{ ...version, is_branch_latest: false }} selectionMode onSelect={onSelect} onView={onView} onDownload={onDownload} />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'eye' }));
    expect(onView).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'download' }));
    expect(onDownload).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('checkbox'));
    expect(onSelect).toHaveBeenCalledWith(true);
    expect(screen.queryByText('Latest')).not.toBeInTheDocument();
  });
});
