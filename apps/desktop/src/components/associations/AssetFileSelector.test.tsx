import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import AssetFileSelector from './AssetFileSelector';
import { AssetInfo } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const hierarchy = [
  {
    id: 1,
    name: 'Plant A',
    description: 'Main plant',
    assetType: 'Folder',
    sortOrder: 0,
    children: [
      {
        id: 2,
        name: 'PLC-001',
        description: 'Primary PLC',
        assetType: 'Device',
        parentId: 1,
        sortOrder: 0,
        children: [],
        createdBy: 1,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
      },
    ],
    createdBy: 1,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  },
];

const selectedAsset: AssetInfo = {
  id: 2,
  name: 'PLC-001',
  description: 'Primary PLC',
  assetType: 'Device',
  parentId: 1,
  sortOrder: 0,
  createdBy: 1,
  createdByUsername: 'admin',
  createdAt: '2024-01-01T00:00:00Z',
  versionCount: 0,
};

describe('AssetFileSelector', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_asset_hierarchy') return hierarchy;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads the hierarchy and renders instructions, search and the tree', async () => {
    render(<AssetFileSelector onAssetSelect={vi.fn()} fileType="Firmware" />);

    expect(await screen.findByText('Select Target Asset')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_asset_hierarchy');
    expect(screen.getByText('Instructions')).toBeInTheDocument();
    expect(screen.getByText(/Select a device to associate with firmware files/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Search assets...')).toBeInTheDocument();
    expect(screen.getByText('Plant A')).toBeInTheDocument();
  });

  it('shows the selected asset alert', async () => {
    render(<AssetFileSelector onAssetSelect={vi.fn()} selectedAsset={selectedAsset} />);

    expect(await screen.findByText('Selected Asset')).toBeInTheDocument();
    expect(screen.getByText('- Primary PLC')).toBeInTheDocument();
  });

  it('lets a device be selected from the tree', async () => {
    const onAssetSelect = vi.fn();
    render(<AssetFileSelector onAssetSelect={onAssetSelect} />);
    await screen.findByText('Plant A');

    // Searching expands matching nodes so the device becomes visible in the tree
    fireEvent.change(screen.getByPlaceholderText('Search assets...'), { target: { value: 'PLC' } });
    const tree = within(document.querySelector('.ant-tree') as HTMLElement);
    const device = await tree.findByText('PLC-001');
    fireEvent.click(device);

    await waitFor(() => expect(onAssetSelect).toHaveBeenCalledWith(expect.objectContaining({ id: 2, name: 'PLC-001' })));
  });

  it('renders the error alert with a retry action when loading fails', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('hierarchy unavailable'));
    render(<AssetFileSelector onAssetSelect={vi.fn()} />);

    expect(await screen.findByText('Error Loading Assets')).toBeInTheDocument();
    expect(screen.getByText('hierarchy unavailable')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
  });
});
