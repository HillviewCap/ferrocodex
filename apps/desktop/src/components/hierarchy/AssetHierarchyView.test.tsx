import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { AssetHierarchyView } from './AssetHierarchyView';
import { AssetHierarchy } from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import useAuthStore from '../../store/auth';
import { useHierarchyStore } from '../../store/hierarchy';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

// Collaborators with their own data loading are stubbed so the test covers the view's
// own Tabs / Breadcrumb / Space / Alert conversions.
vi.mock('./AssetTreeView', () => ({
  AssetTreeView: ({ hierarchyData }: { hierarchyData: AssetHierarchy[] }) => (
    <div data-testid="asset-tree-view">
      {hierarchyData.map(asset => (
        <div key={asset.id}>{asset.name}</div>
      ))}
    </div>
  ),
}));
vi.mock('../security/SimpleAssetCreation', () => ({
  SimpleAssetCreation: ({ open }: { open: boolean }) => (open ? <div data-testid="simple-asset-creation" /> : null),
}));
vi.mock('./CreateFolderModal', () => ({
  CreateFolderModal: ({ open, initialParentId }: { open: boolean; initialParentId: number | null }) =>
    open ? <div data-testid="create-folder-modal">parent:{String(initialParentId)}</div> : null,
}));
vi.mock('../IdentityVault', () => ({ default: () => <div data-testid="identity-vault" /> }));
vi.mock('../metadata/MetadataDisplay', () => ({ default: () => <div data-testid="metadata-display" /> }));
vi.mock('../VaultAccessIndicator', () => ({ default: () => <span data-testid="vault-access-indicator" /> }));
vi.mock('./QuickConfigImportModal', () => ({
  default: ({ visible, onSuccess }: { visible: boolean; onSuccess: () => void }) =>
    visible ? (
      <div data-testid="quick-config-modal">
        <button onClick={onSuccess}>Finish config import</button>
      </div>
    ) : null,
}));
vi.mock('./QuickFirmwareUploadModal', () => ({
  default: ({ visible, onSuccess }: { visible: boolean; onSuccess: () => void }) =>
    visible ? (
      <div data-testid="quick-firmware-modal">
        <button onClick={onSuccess}>Finish firmware upload</button>
      </div>
    ) : null,
}));

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const device: AssetHierarchy = {
  ...base, id: 2, name: 'PLC-001', description: 'Primary PLC', asset_type: 'Device', parent_id: 1, children: [],
  security_classification: SecurityClassificationLevel.CONFIDENTIAL,
};
const folder: AssetHierarchy = {
  ...base,
  id: 1,
  name: 'Production Line 1',
  asset_type: 'Folder',
  parent_id: null,
  children: [device],
  security_classification: SecurityClassificationLevel.INTERNAL,
};
const hierarchy = [folder];

const assetInfo = (asset: AssetHierarchy) => ({
  id: asset.id,
  name: asset.name,
  description: asset.description,
  asset_type: asset.asset_type,
  parent_id: asset.parent_id,
  sort_order: asset.sort_order,
  created_by: asset.created_by,
  created_by_username: 'admin',
  created_at: asset.created_at,
  version_count: 0,
  latest_version: null,
  latest_version_notes: null,
  security_classification: asset.security_classification,
});

describe('AssetHierarchyView', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
      isAuthenticated: true,
    });
    useHierarchyStore.getState().reset();
    useHierarchyStore.setState({ hierarchyData: hierarchy });
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case 'get_asset_hierarchy':
          return hierarchy;
        case 'get_asset_path':
          return args?.assetId === device.id ? [assetInfo(folder), assetInfo(device)] : [assetInfo(folder)];
        case 'get_vault_by_asset_id':
          return null;
        case 'get_configuration_versions':
          return [{ id: 1 }, { id: 2 }];
        case 'get_firmware_list':
          return [{ id: 1 }];
        default:
          throw new Error(`Unexpected command: ${cmd}`);
      }
    });
  });

  it('loads the hierarchy on mount and shows the empty details card', async () => {
    render(<AssetHierarchyView />);

    expect(screen.getByText('Asset Hierarchy')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Create Folder/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Create Device/ })).toBeInTheDocument();
    expect(screen.getByText('Select an asset to view details')).toBeInTheDocument();
    expect(await screen.findByText('Production Line 1')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_asset_hierarchy', { token: 'test-token' });
  });

  it('renders Details and Metadata tabs, breadcrumb and file stats for a selected device', async () => {
    useHierarchyStore.setState({ selectedAsset: device });
    render(<AssetHierarchyView />);

    expect(screen.getByRole('tab', { name: 'Details' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Additional Information' })).toBeInTheDocument();

    // Breadcrumb: Root + the parent folder (the selected asset itself is omitted)
    expect(await screen.findByText('Root')).toBeInTheDocument();
    expect(screen.getByText('Location:')).toBeInTheDocument();
    expect(screen.getAllByText('Production Line 1').length).toBeGreaterThan(1);

    expect(screen.getByText('Device')).toBeInTheDocument();
    expect(screen.getByText('Primary PLC')).toBeInTheDocument();
    expect(screen.getByText('Classification:')).toBeInTheDocument();
    expect(screen.getByText('Confidential')).toBeInTheDocument();
    expect(await screen.findByText('2 Configs')).toBeInTheDocument();
    expect(screen.getByText('1 Firmware')).toBeInTheDocument();
    expect(screen.getByTestId('vault-access-indicator')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Add Configuration/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Upload Firmware/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Identity Vault/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Additional Information' }));
    expect(await screen.findByTestId('metadata-display')).toBeInTheDocument();
  });

  it('opens the quick action modals for a device', async () => {
    useHierarchyStore.setState({ selectedAsset: device });
    render(<AssetHierarchyView />);
    await screen.findByText('Root');

    fireEvent.click(screen.getByRole('button', { name: /Add Configuration/ }));
    expect(screen.getByTestId('quick-config-modal')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Upload Firmware/ }));
    expect(screen.getByTestId('quick-firmware-modal')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Identity Vault/ }));
    expect(await screen.findByText('Identity Vault - PLC-001')).toBeInTheDocument();
    expect(screen.getByTestId('identity-vault')).toBeInTheDocument();
  });

  it('refreshes the file counts after a quick upload succeeds', async () => {
    const configs = [{ id: 1 }, { id: 2 }];
    const firmware = [{ id: 1 }];
    const defaultImpl = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_configuration_versions') return [...configs];
      if (cmd === 'get_firmware_list') return [...firmware];
      return defaultImpl(cmd, args);
    });
    useHierarchyStore.setState({ selectedAsset: device });
    render(<AssetHierarchyView />);
    expect(await screen.findByText('2 Configs')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_configuration_versions', { token: 'test-token', assetId: device.id });
    expect(invoke).toHaveBeenCalledWith('get_firmware_list', { token: 'test-token', assetId: device.id });

    firmware.push({ id: 2 });
    fireEvent.click(screen.getByRole('button', { name: /Upload Firmware/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Finish firmware upload' }));
    expect(await screen.findByText('2 Firmware')).toBeInTheDocument();
    expect(screen.queryByTestId('quick-firmware-modal')).not.toBeInTheDocument();

    configs.push({ id: 3 });
    fireEvent.click(screen.getByRole('button', { name: /Add Configuration/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Finish config import' }));
    expect(await screen.findByText('3 Configs')).toBeInTheDocument();
  });

  it('shows an unknown count instead of 0 when a file count fails to load', async () => {
    const consoleError = vi.spyOn(console, 'error');
    const defaultImpl = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_firmware_list') throw new Error('database locked');
      return defaultImpl(cmd, args);
    });
    useHierarchyStore.setState({ selectedAsset: device });
    render(<AssetHierarchyView />);

    expect(await screen.findByText('2 Configs')).toBeInTheDocument();
    expect(screen.getByText('– Firmware')).toBeInTheDocument();
    expect(screen.queryByText('0 Firmware')).not.toBeInTheDocument();
    expect(consoleError).toHaveBeenCalledWith('Failed to load firmware count:', expect.any(Error));
  });

  it('shows folder contents without a Metadata tab', async () => {
    useHierarchyStore.setState({ selectedAsset: folder });
    render(<AssetHierarchyView />);

    expect(screen.getByRole('tab', { name: 'Details' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Additional Information' })).not.toBeInTheDocument();
    expect(screen.getByText('Contents:')).toBeInTheDocument();
    expect(screen.getByText('1 items')).toBeInTheDocument();
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('get_asset_path', { token: 'test-token', assetId: folder.id }));
  });

  it('opens the create modal and surfaces hierarchy errors', async () => {
    // loadHierarchy runs on mount and clears any seeded error, so make the load itself fail
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_asset_hierarchy') throw new Error('database unavailable');
      return null;
    });
    render(<AssetHierarchyView />);

    expect(await screen.findByText('Error loading hierarchy')).toBeInTheDocument();
    expect(screen.getByText('database unavailable')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Create Folder/ }));
    expect(screen.getByTestId('create-folder-modal')).toHaveTextContent('parent:null');
    expect(screen.queryByTestId('simple-asset-creation')).not.toBeInTheDocument();
  });

  it('opens the device wizard from Create Device', () => {
    render(<AssetHierarchyView />);

    fireEvent.click(screen.getByRole('button', { name: /Create Device/ }));
    expect(screen.getByTestId('simple-asset-creation')).toBeInTheDocument();
    expect(screen.queryByTestId('create-folder-modal')).not.toBeInTheDocument();
  });

  it('refreshes and re-selects the asset after its classification changes', async () => {
    const raisedDevice = { ...device, security_classification: SecurityClassificationLevel.SECRET };
    const raisedFolder = { ...folder, security_classification: SecurityClassificationLevel.SECRET, children: [raisedDevice] };
    let current = hierarchy;
    const baseImpl = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_asset_hierarchy') return current;
      if (cmd === 'update_asset_classification') {
        current = [raisedFolder];
        return assetInfo(raisedFolder);
      }
      return baseImpl(cmd, args);
    });
    useHierarchyStore.setState({ selectedAsset: folder });
    render(<AssetHierarchyView />);
    await screen.findByText('Root');

    fireEvent.click(screen.getByRole('button', { name: /Change/ }));
    fireEvent.mouseDown(await screen.findByRole('combobox'));
    const secret = await waitFor(() => {
      const option = Array.from(document.querySelectorAll('.ant-select-item-option')).find(o => o.textContent?.includes('Secret'));
      expect(option).toBeDefined();
      return option as HTMLElement;
    });
    fireEvent.click(secret);
    fireEvent.click(await screen.findByRole('button', { name: 'Raise 1 item and Save' }));

    await waitFor(() => expect(useHierarchyStore.getState().selectedAsset?.security_classification).toBe(SecurityClassificationLevel.SECRET));
    expect(invoke).toHaveBeenCalledWith('update_asset_classification', {
      token: 'test-token',
      assetId: folder.id,
      securityClassification: SecurityClassificationLevel.SECRET,
    });
  });
});
