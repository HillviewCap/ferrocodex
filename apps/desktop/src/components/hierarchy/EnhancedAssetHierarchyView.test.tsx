import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { EnhancedAssetHierarchyView } from './EnhancedAssetHierarchyView';
import { AssetHierarchy } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useHierarchyStore } from '../../store/hierarchy';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('./AssetTreeView', () => ({
  AssetTreeView: ({ hierarchyData }: { hierarchyData: AssetHierarchy[] }) => (
    <div data-testid="asset-tree-view">
      {hierarchyData.map(asset => (
        <div key={asset.id}>{asset.name}</div>
      ))}
    </div>
  ),
}));
vi.mock('./CreateAssetModal', () => ({
  CreateAssetModal: ({ open }: { open: boolean }) => (open ? <div data-testid="create-asset-modal" /> : null),
}));
vi.mock('../VaultAccessIndicator', () => ({ default: () => <span data-testid="vault-access-indicator" /> }));
vi.mock('../bulk', () => ({
  BulkSelectionManager: () => <div data-testid="bulk-selection-manager" />,
  BulkOperationToolbar: () => <div data-testid="bulk-operation-toolbar" />,
  BulkProgressModal: () => null,
}));

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const device: AssetHierarchy = { ...base, id: 2, name: 'PLC-001', description: 'Primary PLC', asset_type: 'Device', parent_id: 1, children: [] };
const folder: AssetHierarchy = { ...base, id: 1, name: 'Production Line 1', asset_type: 'Folder', parent_id: null, children: [device] };

describe('EnhancedAssetHierarchyView', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
    useHierarchyStore.getState().reset();
    useHierarchyStore.setState({ hierarchyData: [folder] });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_asset_hierarchy':
          return [folder];
        case 'get_vault_by_asset_id':
          return { vault: { id: 5, name: 'PLC vault' } };
        default:
          throw new Error(`Unexpected command: ${cmd}`);
      }
    });
  });

  it('renders the breadcrumb, toolbar, tree and empty details', () => {
    render(<EnhancedAssetHierarchyView />);

    expect(screen.getByText('Assets')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Create Asset/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Refresh/ })).toBeInTheDocument();
    expect(screen.getByText('Bulk Mode:')).toBeInTheDocument();
    expect(screen.getByRole('switch')).toBeInTheDocument();
    expect(screen.getByText('Production Line 1')).toBeInTheDocument();
    expect(screen.getByText('Select an asset to view details')).toBeInTheDocument();
  });

  it('shows device details with vault status for the selected asset', async () => {
    useHierarchyStore.setState({ selectedAsset: device });
    render(<EnhancedAssetHierarchyView />);

    expect(screen.getByText('PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Type:')).toBeInTheDocument();
    expect(screen.getByText('Device')).toBeInTheDocument();
    expect(screen.getByText('Primary PLC')).toBeInTheDocument();
    expect(screen.getByText('Device Information')).toBeInTheDocument();
    expect(screen.getByText('Configurations')).toBeInTheDocument();
    expect(screen.getByText('Vault')).toBeInTheDocument();

    expect(await screen.findByText('Protected')).toBeInTheDocument();
    expect(screen.getByTestId('vault-access-indicator')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_vault_by_asset_id', { assetId: device.id });
  });

  it('enables bulk mode and opens the create modal', () => {
    render(<EnhancedAssetHierarchyView />);

    fireEvent.click(screen.getByRole('switch'));
    expect(screen.getByTestId('bulk-selection-manager')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Create Asset/ }));
    expect(screen.getByTestId('create-asset-modal')).toBeInTheDocument();
  });

  it('refreshes the hierarchy from the toolbar', async () => {
    render(<EnhancedAssetHierarchyView />);

    fireEvent.click(screen.getByRole('button', { name: /Refresh/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_asset_hierarchy', { token: 'test-token' }));
    expect(await screen.findByText('Asset hierarchy refreshed')).toBeInTheDocument();
  });

  it('renders the error alert with a retry action', () => {
    useHierarchyStore.setState({ error: 'boom' });
    render(<EnhancedAssetHierarchyView />);

    expect(screen.getByText('Error Loading Asset Hierarchy')).toBeInTheDocument();
    expect(screen.getByText('boom')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
  });
});
