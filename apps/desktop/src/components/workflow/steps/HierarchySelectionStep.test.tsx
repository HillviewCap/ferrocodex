import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { HierarchySelectionStep } from './HierarchySelectionStep';
import { useHierarchyStore } from '../../../store/hierarchy';
import { AssetHierarchy } from '../../../types/assets';
import { useConsoleGuard } from '../../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const hierarchy: AssetHierarchy[] = [
  { ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [] },
];

const stepProps = () => ({
  workflowId: 'wf-1',
  onDataChange: vi.fn(),
  onValidation: vi.fn(),
  onNext: vi.fn(),
  onPrevious: vi.fn(),
});

describe('HierarchySelectionStep', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useHierarchyStore.getState().reset();
    useHierarchyStore.setState({ hierarchyData: hierarchy });
  });

  it('requires a folder for devices and shows the placement alert', () => {
    const props = stepProps();
    render(<HierarchySelectionStep {...props} data={{ asset_type: 'Device', asset_name: 'PLC-001' }} />);

    expect(screen.getByText('Select Location')).toBeInTheDocument();
    expect(screen.getByText('Choose where to place "PLC-001" in your asset hierarchy.')).toBeInTheDocument();
    expect(screen.getByText('Device Placement Required')).toBeInTheDocument();
    expect(screen.getByText('Select a parent folder')).toBeInTheDocument();
    expect(screen.getByText('Hierarchy Guidelines')).toBeInTheDocument();
    expect(screen.queryByText('Root-level folders create top-level categories')).not.toBeInTheDocument();

    expect(props.onValidation).toHaveBeenCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'parent_id' })]),
    );
  });

  it('allows root placement for folders and shows the selection summary', () => {
    const props = stepProps();
    render(<HierarchySelectionStep {...props} data={{ asset_type: 'Folder', asset_name: 'Line 2', parent_id: null }} />);

    expect(screen.queryByText('Device Placement Required')).not.toBeInTheDocument();
    expect(screen.getByText('Root Level')).toBeInTheDocument();
    expect(screen.getByText('This asset will be created at the root level of your hierarchy')).toBeInTheDocument();
    expect(screen.getByText('Root-level folders create top-level categories')).toBeInTheDocument();
    expect(props.onValidation).toHaveBeenCalledWith(true, []);
  });
});
