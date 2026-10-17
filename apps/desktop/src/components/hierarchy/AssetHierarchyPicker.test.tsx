import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { AssetHierarchyPicker } from './AssetHierarchyPicker';
import { AssetHierarchy } from '../../types/assets';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };

const hierarchy: AssetHierarchy[] = [
  {
    ...base,
    id: 1,
    name: 'Plant A',
    asset_type: 'Folder',
    parent_id: null,
    children: [
      { ...base, id: 2, name: 'Line 1', asset_type: 'Folder', parent_id: 1, children: [] },
      { ...base, id: 3, name: 'PLC-001', asset_type: 'Device', parent_id: 1, children: [] },
    ],
  },
  { ...base, id: 4, name: 'HMI-002', asset_type: 'Device', parent_id: null, children: [] },
];

describe('AssetHierarchyPicker', () => {
  useConsoleGuard();

  it('renders the placeholder, root-level hint, and the root shortcut', () => {
    render(<AssetHierarchyPicker hierarchyData={hierarchy} value={null} />);

    expect(screen.getByText('Select parent folder (leave empty for root level)')).toBeInTheDocument();
    expect(screen.getByText('Asset will be created at the root level')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Place in root level/ })).toBeInTheDocument();
  });

  it('only offers folders in the tree dropdown', async () => {
    render(<AssetHierarchyPicker hierarchyData={hierarchy} value={null} />);

    fireEvent.mouseDown(screen.getByRole('combobox'));

    expect(await screen.findByText('Plant A')).toBeInTheDocument();
    expect(screen.getByText('(1 folders)')).toBeInTheDocument();
    expect(screen.queryByText('HMI-002')).not.toBeInTheDocument();
    expect(screen.queryByText('PLC-001')).not.toBeInTheDocument();
  });

  it('reports the selected folder and clears back to root', async () => {
    const onChange = vi.fn();
    const { rerender } = render(<AssetHierarchyPicker hierarchyData={hierarchy} value={null} onChange={onChange} />);

    fireEvent.mouseDown(screen.getByRole('combobox'));
    fireEvent.click(await screen.findByText('Plant A'));
    expect(onChange).toHaveBeenCalledWith(1);

    rerender(<AssetHierarchyPicker hierarchyData={hierarchy} value={1} onChange={onChange} />);
    expect(screen.getByText('Asset will be created inside the selected folder')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Place in root level/ }));
    expect(onChange).toHaveBeenLastCalledWith(null);
  });
});
