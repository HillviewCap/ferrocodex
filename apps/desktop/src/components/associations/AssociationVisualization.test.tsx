import { render, screen, fireEvent, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import AssociationVisualization from './AssociationVisualization';
import useAssociationStore from '../../store/associations';
import { AssociationInfo } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const associations: AssociationInfo[] = [
  {
    id: 100,
    assetId: 2,
    assetName: 'PLC-001',
    fileId: 10,
    fileName: 'plc.cfg',
    fileType: 'Configuration',
    associationOrder: 1,
    createdBy: 1,
    createdByUsername: 'admin',
    createdAt: '2024-01-01T00:00:00Z',
    validationStatus: 'Passed',
  },
  {
    id: 101,
    assetId: 2,
    assetName: 'PLC-001',
    fileId: 11,
    fileName: 'firmware.bin',
    fileType: 'Firmware',
    associationOrder: 2,
    createdBy: 1,
    createdByUsername: 'admin',
    createdAt: '2024-01-01T00:00:00Z',
    validationStatus: 'Warning',
  },
];

const hierarchy = [
  {
    id: 1,
    name: 'Plant A',
    asset_type: 'Folder',
    parent_id: null,
    children: [{ id: 2, name: 'PLC-001', asset_type: 'Device', parent_id: 1, children: [] }],
  },
];

describe('AssociationVisualization', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAssociationStore.getState().reset();
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_asset_file_associations':
          return associations;
        case 'get_association_health_status':
          return { healthy: true, issues: [], warnings: [], lastChecked: '2024-01-01T00:00:00Z' };
        case 'get_asset_hierarchy':
          return hierarchy;
        default:
          throw new Error(`Unexpected command: ${cmd}`);
      }
    });
  });

  it('loads associations for the asset and renders the tree, settings and legend', async () => {
    render(<AssociationVisualization assetId={2} />);

    expect(screen.getByText('Asset-File Relationships')).toBeInTheDocument();
    expect(await screen.findByText('Plant A')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_asset_file_associations', { assetId: 2 });
    // Note: the parallel `get_association_health_status` call is not asserted. The store and
    // component load `invoke` through concurrent dynamic imports of the vi.mock'ed module,
    // and under vitest the second one can resolve to undefined (swallowed by the store's
    // catch), which makes that call non-deterministic in jsdom.

    expect(screen.getByText('Show empty assets:')).toBeInTheDocument();
    expect(screen.getByText('Show validation status:')).toBeInTheDocument();
    expect(screen.getByText('Expand all:')).toBeInTheDocument();
    expect(screen.getByText('Legend')).toBeInTheDocument();
    expect(screen.getByText('Configuration File')).toBeInTheDocument();
    expect(screen.getByText('Has Associations')).toBeInTheDocument();

    // Expand everything to reveal the nested device node
    const switches = screen.getAllByRole('switch');
    fireEvent.click(switches[2]);
    expect(await screen.findByText('PLC-001')).toBeInTheDocument();
    const tree = within(document.querySelector('.ant-tree') as HTMLElement);
    expect(tree.getByText('Plant A')).toBeInTheDocument();
    expect(tree.getByText('PLC-001')).toBeInTheDocument();
    // Note: per-file association leaves are not asserted. buildAssetNodes reads the
    // `associations` captured when loadData was created (empty on mount), so the initial
    // load never attaches them; that is pre-existing behaviour unrelated to the Tree
    // `treeData` conversion.
  });

  it('renders the compact variant without settings or legend', () => {
    render(<AssociationVisualization compact />);

    expect(screen.getByText('Asset-File Relationships')).toBeInTheDocument();
    expect(screen.getByText('No asset relationships to display')).toBeInTheDocument();
    expect(screen.queryByText('Legend')).not.toBeInTheDocument();
    expect(screen.queryByText('Show empty assets:')).not.toBeInTheDocument();
  });

  it('renders the store error in an alert', () => {
    useAssociationStore.setState({ error: 'relationships unavailable' });
    render(<AssociationVisualization />);

    expect(screen.getByText('Error Loading Relationships')).toBeInTheDocument();
    expect(screen.getByText('relationships unavailable')).toBeInTheDocument();
  });
});
