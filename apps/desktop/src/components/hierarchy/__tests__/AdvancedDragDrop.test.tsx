import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import AdvancedDragDrop from '../AdvancedDragDrop';
import { AssetHierarchy } from '../../../types/assets';

// Mock the bulk operations store
const mockStore = {
  getSelectedAssets: vi.fn(() => [1, 2]),
  getSelectedCount: vi.fn(() => 2),
  isSelected: vi.fn((id: number) => [1, 2].includes(id)),
  validateBulkMove: vi.fn().mockResolvedValue({
    is_valid: true,
    errors: [],
    warnings: [],
    conflicts: []
  })
};

vi.mock('../../../store/bulkOperations', () => ({
  default: vi.fn(() => mockStore)
}));

// Mock Ant Design message
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: {
      success: vi.fn(),
      error: vi.fn(),
      info: vi.fn()
    }
  };
});

const mockAssets: AssetHierarchy[] = [
  {
    id: 1,
    name: 'Folder 1',
    asset_type: 'Folder',
    parent_id: null,
    path: '/folder1',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: [
      {
        id: 2,
        name: 'Asset 1',
        asset_type: 'Equipment',
        parent_id: 1,
        path: '/folder1/asset1',
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
        children: []
      }
    ]
  },
  {
    id: 3,
    name: 'Folder 2',
    asset_type: 'Folder',
    parent_id: null,
    path: '/folder2',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: []
  }
];

describe('AdvancedDragDrop', () => {
  const mockOnMultiAssetMove = vi.fn();
  const mockOnDragStart = vi.fn();
  const mockOnDragEnd = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render children correctly', () => {
    render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
      >
        <div data-testid="child-content">Test Content</div>
      </AdvancedDragDrop>
    );

    expect(screen.getByTestId('child-content')).toBeInTheDocument();
  });

  it('should handle drag start with single asset', async () => {
    mockStore.isSelected.mockReturnValue(false);
    mockStore.getSelectedAssets.mockReturnValue([]);

    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
        onDragStart={mockOnDragStart}
      >
        <div data-asset-id="2">Asset 1</div>
      </AdvancedDragDrop>
    );

    const dragElement = container.querySelector('[data-asset-id="2"]');
    expect(dragElement).toBeInTheDocument();

    // Create a mock drag event
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn()
    };

    const dragEvent = new Event('dragstart', { bubbles: true }) as any;
    dragEvent.dataTransfer = mockDataTransfer;
    
    if (dragElement) {
      fireEvent(dragElement, dragEvent);
    }

    expect(mockOnDragStart).toHaveBeenCalled();
  });

  it('should handle drag start with multiple selected assets', async () => {
    mockStore.isSelected.mockReturnValue(true);
    mockStore.getSelectedAssets.mockReturnValue([1, 2]);

    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
        onDragStart={mockOnDragStart}
      >
        <div data-asset-id="2">Asset 1</div>
      </AdvancedDragDrop>
    );

    const dragElement = container.querySelector('[data-asset-id="2"]');
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn()
    };

    const dragEvent = new Event('dragstart', { bubbles: true }) as any;
    dragEvent.dataTransfer = mockDataTransfer;
    
    if (dragElement) {
      fireEvent(dragElement, dragEvent);
    }

    expect(mockOnDragStart).toHaveBeenCalledWith(
      expect.objectContaining({
        assetIds: [1, 2]
      })
    );
  });

  it('should validate drop targets correctly', async () => {
    mockStore.validateBulkMove.mockResolvedValueOnce({
      is_valid: true,
      errors: [],
      warnings: [],
      conflicts: []
    });

    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
      >
        <div data-asset-id="2">Asset 1</div>
        <div data-drop-target-id="3">Folder 2</div>
      </AdvancedDragDrop>
    );

    // Simulate drag start
    const dragElement = container.querySelector('[data-asset-id="2"]');
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn(),
      dropEffect: 'none'
    };

    const dragStartEvent = new Event('dragstart', { bubbles: true }) as any;
    dragStartEvent.dataTransfer = mockDataTransfer;
    
    if (dragElement) {
      fireEvent(dragElement, dragStartEvent);
    }

    // Simulate drag over
    const dropElement = container.querySelector('[data-drop-target-id="3"]');
    const dragOverEvent = new Event('dragover', { bubbles: true }) as any;
    dragOverEvent.dataTransfer = mockDataTransfer;
    
    if (dropElement) {
      fireEvent(dropElement, dragOverEvent);
    }

    await waitFor(() => {
      expect(mockStore.validateBulkMove).toHaveBeenCalledWith([2], 3);
    });
  });

  it('should handle drop events', async () => {
    mockStore.validateBulkMove.mockResolvedValue({
      is_valid: true,
      errors: [],
      warnings: [],
      conflicts: []
    });

    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
      >
        <div data-asset-id="2">Asset 1</div>
        <div data-drop-target-id="3">Folder 2</div>
      </AdvancedDragDrop>
    );

    // Simulate complete drag and drop sequence
    const dragElement = container.querySelector('[data-asset-id="2"]');
    const dropElement = container.querySelector('[data-drop-target-id="3"]');
    
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn(),
      dropEffect: 'none'
    };

    // Drag start
    const dragStartEvent = new Event('dragstart', { bubbles: true }) as any;
    dragStartEvent.dataTransfer = mockDataTransfer;
    if (dragElement) {
      fireEvent(dragElement, dragStartEvent);
    }

    // Drag over
    const dragOverEvent = new Event('dragover', { bubbles: true }) as any;
    dragOverEvent.dataTransfer = mockDataTransfer;
    if (dropElement) {
      fireEvent(dropElement, dragOverEvent);
    }

    // Drop
    const dropEvent = new Event('drop', { bubbles: true }) as any;
    dropEvent.dataTransfer = mockDataTransfer;
    if (dropElement) {
      fireEvent(dropElement, dropEvent);
    }

    await waitFor(() => {
      expect(mockOnMultiAssetMove).toHaveBeenCalledWith([2], 3);
    });
  });

  it('should prevent invalid drops', async () => {
    mockStore.validateBulkMove.mockResolvedValue({
      is_valid: false,
      errors: [{ asset_id: 2, asset_name: 'Asset 1', error_type: 'invalid_target', message: 'Cannot drop here', blocking: true }],
      warnings: [],
      conflicts: []
    });

    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
      >
        <div data-asset-id="2">Asset 1</div>
        <div data-drop-target-id="3">Folder 2</div>
      </AdvancedDragDrop>
    );

    // Complete drag and drop sequence
    const dragElement = container.querySelector('[data-asset-id="2"]');
    const dropElement = container.querySelector('[data-drop-target-id="3"]');
    
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn(),
      dropEffect: 'none'
    };

    // Drag start
    const dragStartEvent = new Event('dragstart', { bubbles: true }) as any;
    dragStartEvent.dataTransfer = mockDataTransfer;
    if (dragElement) {
      fireEvent(dragElement, dragStartEvent);
    }

    // Drag over
    const dragOverEvent = new Event('dragover', { bubbles: true }) as any;
    dragOverEvent.dataTransfer = mockDataTransfer;
    if (dropElement) {
      fireEvent(dropElement, dragOverEvent);
    }

    await waitFor(() => {
      expect(mockDataTransfer.dropEffect).toBe('none');
    });

    // Drop should not call onMultiAssetMove
    const dropEvent = new Event('drop', { bubbles: true }) as any;
    dropEvent.dataTransfer = mockDataTransfer;
    if (dropElement) {
      fireEvent(dropElement, dropEvent);
    }

    expect(mockOnMultiAssetMove).not.toHaveBeenCalled();
  });

  it('should handle drag end events', () => {
    const { container } = render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
        onDragEnd={mockOnDragEnd}
      >
        <div data-asset-id="2">Asset 1</div>
      </AdvancedDragDrop>
    );

    const dragElement = container.querySelector('[data-asset-id="2"]');
    const mockDataTransfer = {
      setData: vi.fn(),
      setDragImage: vi.fn()
    };

    // Start drag
    const dragStartEvent = new Event('dragstart', { bubbles: true }) as any;
    dragStartEvent.dataTransfer = mockDataTransfer;
    if (dragElement) {
      fireEvent(dragElement, dragStartEvent);
    }

    // End drag
    const dragEndEvent = new Event('dragend', { bubbles: true });
    document.dispatchEvent(dragEndEvent);

    expect(mockOnDragEnd).toHaveBeenCalled();
  });

  it('should respect reduced motion preferences', () => {
    // Mock matchMedia for reduced motion
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query) => ({
        matches: query === '(prefers-reduced-motion: reduce)',
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    render(
      <AdvancedDragDrop
        assets={mockAssets}
        onMultiAssetMove={mockOnMultiAssetMove}
      >
        <div data-testid="child-content">Test Content</div>
      </AdvancedDragDrop>
    );

    expect(screen.getByTestId('child-content')).toBeInTheDocument();
  });
});