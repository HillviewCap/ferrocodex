import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AssetTreeView } from './AssetTreeView';
import { AssetHierarchy } from '../../types/assets';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock auth store
vi.mock('../../store/auth', () => ({
  default: () => ({
    token: 'mock-token',
  }),
}));

describe('AssetTreeView', () => {
  const mockHierarchyData: AssetHierarchy[] = [
    {
      id: 1,
      name: 'Production Line 1',
      description: 'Main production line',
      asset_type: 'Folder',
      parent_id: null,
      sort_order: 0,
      children: [
        {
          id: 2,
          name: 'PLC-001',
          description: 'Primary PLC',
          asset_type: 'Device',
          parent_id: 1,
          sort_order: 0,
          children: [],
          created_by: 1,
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
        {
          id: 3,
          name: 'Control Room',
          description: 'Control room assets',
          asset_type: 'Folder',
          parent_id: 1,
          sort_order: 1,
          children: [
            {
              id: 4,
              name: 'HMI-001',
              description: 'Main HMI',
              asset_type: 'Device',
              parent_id: 3,
              sort_order: 0,
              children: [],
              created_by: 1,
              created_at: '2024-01-01T00:00:00Z',
              updated_at: '2024-01-01T00:00:00Z',
            },
          ],
          created_by: 1,
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ],
      created_by: 1,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
  ];

  const defaultProps = {
    hierarchyData: mockHierarchyData,
    onAssetSelect: vi.fn(),
    onAssetCreate: vi.fn(),
    onAssetEdit: vi.fn(),
    onAssetDelete: vi.fn(),
    onAssetMove: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Folders start collapsed; click a folder's expand switcher to reveal its children
  const expandFolder = (name: string) => {
    const node = screen.getByText(name).closest('.ant-tree-treenode');
    const switcher = node?.querySelector('.ant-tree-switcher');
    expect(switcher).toBeInTheDocument();
    fireEvent.click(switcher!);
  };

  it('renders tree structure correctly', () => {
    render(<AssetTreeView {...defaultProps} />);
    
    expect(screen.getByText('Production Line 1')).toBeInTheDocument();

    expandFolder('Production Line 1');
    expect(screen.getByText('PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Control Room')).toBeInTheDocument();
  });

  it('displays folder and device icons correctly', () => {
    render(<AssetTreeView {...defaultProps} />);
    
    // Check that icons are rendered (folder and tool icons)
    const icons = document.querySelectorAll('.anticon');
    expect(icons.length).toBeGreaterThan(0);
  });

  it('shows child count for folders', () => {
    render(<AssetTreeView {...defaultProps} />);
    
    // Production Line 1 has 2 children
    expect(screen.getByText('(2)')).toBeInTheDocument();

    expandFolder('Production Line 1');
    // Control Room has 1 child
    expect(screen.getByText('(1)')).toBeInTheDocument();
  });

  it('calls onAssetSelect when tree node is clicked', () => {
    const mockOnAssetSelect = vi.fn();
    render(<AssetTreeView {...defaultProps} onAssetSelect={mockOnAssetSelect} />);
    
    const treeNode = screen.getByText('Production Line 1');
    fireEvent.click(treeNode);
    
    expect(mockOnAssetSelect).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 1,
        name: 'Production Line 1',
        asset_type: 'Folder',
      })
    );
  });

  it('highlights selected asset', () => {
    render(<AssetTreeView {...defaultProps} selectedAssetId={1} />);
    
    // The selected node should have the selected class
    const selectedNode = document.querySelector('.ant-tree-node-selected');
    expect(selectedNode).toBeInTheDocument();
  });

  it('shows empty state when no assets', () => {
    render(<AssetTreeView {...defaultProps} hierarchyData={[]} />);
    
    expect(screen.getByText('No assets found. Right-click to create your first asset.')).toBeInTheDocument();
  });

  it('shows loading state', () => {
    render(<AssetTreeView {...defaultProps} loading={true} />);
    
    // Check that the tree has loading state
    const tree = document.querySelector('.ant-tree');
    expect(tree).toBeInTheDocument();
  });

  it('expands and collapses folders', async () => {
    render(<AssetTreeView {...defaultProps} />);
    
    // Initially collapsed, child nodes aren't rendered
    expect(screen.queryByText('PLC-001')).not.toBeInTheDocument();

    expandFolder('Production Line 1');
    expect(screen.getByText('PLC-001')).toBeVisible();

    // rc-tree ignores expand clicks until it clears its list-motion flag in a
    // setTimeout; let that run before collapsing
    await act(() => new Promise(resolve => setTimeout(resolve, 0)));

    expandFolder('Production Line 1');
    await waitFor(() => {
      expect(screen.queryByText('PLC-001')).not.toBeInTheDocument();
    });
  });

  it('handles drag and drop when allowDragDrop is true', () => {
    render(<AssetTreeView {...defaultProps} allowDragDrop={true} />);

    expect(document.querySelector('.ant-tree-treenode[draggable="true"]')).toBeInTheDocument();
  });

  it('disables drag and drop when allowDragDrop is false', () => {
    render(<AssetTreeView {...defaultProps} allowDragDrop={false} />);

    expect(document.querySelector('.ant-tree-treenode')).toBeInTheDocument();
    expect(document.querySelector('.ant-tree-treenode[draggable="true"]')).not.toBeInTheDocument();
  });

  it('displays different content for folders vs devices', () => {
    render(<AssetTreeView {...defaultProps} />);
    
    // Folders should show folder icons and child counts
    const productionLine = screen.getByText('Production Line 1');
    expect(productionLine.closest('.ant-tree-node-content-wrapper')).toBeInTheDocument();
    
    // Devices should show device icons
    expandFolder('Production Line 1');
    const plc = screen.getByText('PLC-001');
    expect(plc.closest('.ant-tree-node-content-wrapper')).toBeInTheDocument();
  });
});